use actix_web::{error::ResponseError, HttpResponse};
use bourso_api::client::error::ClientError;
use derive_more::Display;

#[derive(Debug, Display)]
pub enum ApiError {
    #[display(fmt = "Internal Server Error")]
    InternalError,
    #[display(fmt = "Bad Request")]
    BadClientData,
    #[display(fmt = "Unauthorized")]
    Unauthorized,
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::InternalError => HttpResponse::InternalServerError().json("Internal Server Error"),
            ApiError::BadClientData => HttpResponse::BadRequest().json("Bad Request"),
            ApiError::Unauthorized => HttpResponse::Unauthorized().json("Unauthorized"),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast_ref::<ClientError>() {
            Some(ClientError::InvalidCredentials) => ApiError::Unauthorized,
            Some(ClientError::MfaRequired) => ApiError::Unauthorized,
            _ => ApiError::InternalError,
        }
    }
}
