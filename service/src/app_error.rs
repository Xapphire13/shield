use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use tracing::error;

pub enum AppError {
    InvalidToken,
    ExpiredToken,
    InvalidOtpCode,
    Unknown(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token".to_string()),
            AppError::ExpiredToken => (StatusCode::UNAUTHORIZED, "Expired token".to_string()),
            AppError::InvalidOtpCode => (StatusCode::UNAUTHORIZED, "Invalid OTP code".to_string()),
            AppError::Unknown(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error!".to_string(),
            ),
        };

        match &self {
            AppError::Unknown(error) => {
                error!("{error}");
            }
            _ => {
                error!("{error_message}");
            }
        }

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::Unknown(err.into())
    }
}
