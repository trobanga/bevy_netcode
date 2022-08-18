use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use secrecy::{ExposeSecret, Secret};

use crate::{
    db::{self, DbPool},
    spawn_blocking_with_tracing,
};

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),

    #[error("Failed parse PasswordHash")]
    FailedToParsePasswordHash,

    #[error("Failed hash password")]
    FailedToHashPassword,

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &DbPool,
) -> Result<uuid::Uuid, AuthError> {
    let (stored_user_id, stored_password_hash) =
        get_stored_credentials(&credentials.username, pool).await?;
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

#[tracing::instrument(name = "Get stored credentials", skip(username, pool))]
async fn get_stored_credentials(
    username: &str,
    pool: &DbPool,
) -> Result<(uuid::Uuid, Secret<String>), anyhow::Error> {
    let user = db::actions::find_user_by_name(username, pool)?;
    Ok((user.uuid, Secret::new(user.password)))
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

#[tracing::instrument(name = "Change password", skip(password, pool))]
pub async fn change_password(
    user_id: uuid::Uuid,
    password: Secret<String>,
    pool: &DbPool,
) -> Result<(), anyhow::Error> {
    let password_hash =
        spawn_blocking_with_tracing(move || compute_password_hash(password)).await??;
    db::actions::set_password_for_user(user_id, password_hash, pool)?;
    Ok(())
}

fn compute_password_hash(password: Secret<String>) -> Result<Secret<String>, AuthError> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
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
