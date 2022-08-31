use actix_web::http::{
    header::{self, HeaderValue},
    StatusCode,
};
use actix_web::{HttpResponse, ResponseError};

use crate::db;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),

    #[error("Failed parse PasswordHash")]
    FailedToParsePasswordHash,

    #[error("Failed hash password")]
    FailedToHashPassword,

    #[error("Unknown user")]
    UnknownUser,

    #[error("User error {0}")]
    UserAction(#[from] db::actions::Error),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match self {
            Error::UnexpectedError(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
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
