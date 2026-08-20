//! Error unificado de la API y su traducción a respuestas HTTP.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use shared::split::SplitError;
use shared::ErrorResponse;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    Unauthorized(String),

    #[error("{0}")]
    Forbidden(String),

    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    Conflict(String),

    #[error("{0}")]
    PayloadTooLarge(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error("error interno: {0}")]
    Internal(String),
}

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        AppError::BadRequest(msg.into())
    }
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        AppError::Unauthorized(msg.into())
    }
    pub fn forbidden(msg: impl Into<String>) -> Self {
        AppError::Forbidden(msg.into())
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        AppError::NotFound(msg.into())
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        AppError::Conflict(msg.into())
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        AppError::Internal(msg.into())
    }
}

impl From<SplitError> for AppError {
    fn from(err: SplitError) -> Self {
        AppError::BadRequest(err.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone()),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            AppError::PayloadTooLarge(m) => (StatusCode::PAYLOAD_TOO_LARGE, m.clone()),

            // Violación de unicidad => 409 en lugar de un 500 opaco.
            AppError::Database(sqlx::Error::Database(db)) if db.is_unique_violation() => (
                StatusCode::CONFLICT,
                "ya existe un registro con esos datos".to_string(),
            ),
            AppError::Database(sqlx::Error::RowNotFound) => (
                StatusCode::NOT_FOUND,
                "no se encontró el recurso".to_string(),
            ),
            AppError::Database(err) => {
                tracing::error!(error = %err, "error de base de datos");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "error interno del servidor".to_string(),
                )
            }
            AppError::Internal(err) => {
                tracing::error!(error = %err, "error interno");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "error interno del servidor".to_string(),
                )
            }
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}
