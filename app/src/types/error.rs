use gloo_storage::errors::StorageError;
use reqwest::StatusCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Network error")]
    NetworkError,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Bad request")]
    BadRequest,
    #[error("Server error")]
    ServerError,
    #[error("Token missing")]
    TokenMissing,
    #[error("Storage error")]
    StorageError,
}

impl From<reqwest::Error> for ApiError {
    fn from(_: reqwest::Error) -> Self {
        Self::NetworkError
    }
}

impl From<StorageError> for ApiError {
    fn from(_: StorageError) -> Self {
        ApiError::StorageError
    }
}

impl From<StatusCode> for ApiError {
    fn from(status: StatusCode) -> Self {
        match status {
            StatusCode::UNAUTHORIZED => Self::Unauthorized,
            status if status.is_client_error() => Self::BadRequest,
            status if status.is_server_error() => Self::ServerError,
            _ => Self::NetworkError,
        }
    }
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
}
