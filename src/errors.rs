pub type Result<T> = core::result::Result<T, AppError>;

use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
pub enum AppError {
    #[error("Internal server error - {title}: {message}")]
    InternalError { title: String, message: String },

    #[error("Bad request - {title}: {message}")]
    BadRequest { title: String, message: String },

    #[error("Authentication error: {message}")]
    AuthError { message: String },
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::InternalError { .. } => HttpResponse::InternalServerError().json(self),
            AppError::BadRequest { .. } => HttpResponse::BadRequest().json(self),
            AppError::AuthError { .. } => HttpResponse::Unauthorized().json(self),
            // Handle other variants accordingly
        }
    }
}

impl From<base64::DecodeError> for AppError {
    fn from(err: base64::DecodeError) -> AppError {
        AppError::BadRequest {
            title: "Base64 decoding error".to_string(),
            message: err.to_string(),
        }
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for AppError {
    fn from(err: tokio::sync::mpsc::error::SendError<T>) -> AppError {
        AppError::InternalError {
            title: "Channel send error".to_string(),
            message: err.to_string(),
        }
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for AppError {
    fn from(err: tokio::sync::oneshot::error::RecvError) -> AppError {
        AppError::InternalError {
            title: "Oneshot receive error".to_string(),
            message: err.to_string(),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> AppError {
        AppError::InternalError {
            title: "I/O error".to_string(),
            message: err.to_string(),
        }
    }
}

impl From<csv::Error> for AppError {
    fn from(err: csv::Error) -> AppError {
        AppError::InternalError {
            title: "CSV error".to_string(),
            message: err.to_string(),
        }
    }
}
