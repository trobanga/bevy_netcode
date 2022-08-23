use actix_web::http::{
    header::{self, HeaderMap, HeaderValue},
    StatusCode,
};
use actix_web::{HttpResponse, ResponseError};
use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use secrecy::{ExposeSecret, Secret};

use crate::{
    db::{self, DbConnection},
    spawn_blocking_with_tracing,
};

pub async fn basic_authentication(
    headers: &HeaderMap,
    conn: &mut DbConnection,
) -> Result<uuid::Uuid, AuthError> {
    // The header value, if present, must be a valid UTF8 string
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")
        .map_err(AuthError::InvalidCredentials)?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")
        .map_err(AuthError::InvalidCredentials)?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")
        .map_err(AuthError::InvalidCredentials)?;
    let decoded_bytes = base64::decode_config(base64encoded_segment, base64::STANDARD)
        .context("Failed to base64-decode 'Basic' credentials.")
        .map_err(AuthError::InvalidCredentials)?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")
        .map_err(AuthError::InvalidCredentials)?;

    // Split into two segments, using ':' as delimitator
    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))
        .map_err(AuthError::InvalidCredentials)?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))
        .map_err(AuthError::InvalidCredentials)?
        .to_string();

    let credentials = Credentials {
        username,
        password: Secret::new(password),
    };
    validate_credentials(credentials, conn).await
}

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),

    #[error("Failed parse PasswordHash")]
    FailedToParsePasswordHash,

    #[error("Failed hash password")]
    FailedToHashPassword,

    #[error("Unknown user")]
    UnknownUser,

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for AuthError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AuthError::UnexpectedError(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            _ => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="login""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
        }
    }
}

#[derive(Debug)]
pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, conn))]
pub async fn validate_credentials(
    credentials: Credentials,
    conn: &mut DbConnection,
) -> Result<uuid::Uuid, AuthError> {
    let (stored_user_id, stored_password_hash) =
        get_stored_credentials(&credentials.username, conn).await?;
    let user_id = Some(stored_user_id);
    let expected_password_hash = stored_password_hash;

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")??;

    user_id
        .ok_or_else(|| anyhow::anyhow!("Unknown username."))
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Get stored credentials", skip(username, conn))]
async fn get_stored_credentials(
    username: &str,
    conn: &mut DbConnection,
) -> Result<(uuid::Uuid, Secret<String>), AuthError> {
    let user = db::actions::find_user_by_name(username, conn)?;
    if let Some(user) = user {
        Ok((user.uuid, Secret::new(user.password)))
    } else {
        Err(AuthError::UnknownUser)
    }
}

#[tracing::instrument(
    name = "Validate credentials",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .map_err(|_| AuthError::FailedToParsePasswordHash)?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .map_err(|e| AuthError::InvalidCredentials(anyhow::anyhow!(e.to_string())))?;
    Ok(())
}

#[tracing::instrument(name = "Change password", skip(password, conn))]
pub async fn change_password(
    user_id: uuid::Uuid,
    password: Secret<String>,
    conn: &mut DbConnection,
) -> Result<(), anyhow::Error> {
    let password_hash =
        spawn_blocking_with_tracing(move || compute_password_hash(password)).await??;
    db::actions::set_password_for_user(user_id, password_hash, conn)?;
    Ok(())
}

pub fn compute_password_hash(password: Secret<String>) -> Result<Secret<String>, AuthError> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::default()
        .hash_password(password.expose_secret().as_bytes(), &salt)
        .map_err(|_| AuthError::FailedToHashPassword)?
        .to_string();
    Ok(Secret::new(password_hash))
}

#[cfg(test)]
mod tests {
    use secrecy::Secret;

    #[test]
    fn correct_password_returns_ok() {
        let password_hash = super::compute_password_hash(Secret::new("hallo".to_string())).unwrap();

        if let Err(e) = super::verify_password_hash(password_hash, Secret::new("hallo".to_string()))
        {
            panic!("{e}");
        }
    }

    #[test]
    fn wrong_password_is_rejected() {
        let password_hash = super::compute_password_hash(Secret::new("hallo".to_string())).unwrap();

        if let Ok(()) = super::verify_password_hash(password_hash, Secret::new("there".to_string()))
        {
            panic!("Wrong password passed");
        }
    }
}
