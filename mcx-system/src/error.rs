use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("invalid token")]
    InvalidToken,

    #[allow(dead_code)]
    #[error("token expired")]
    TokenExpired,

    #[error("wechat token expired")]
    WechatTokenExpired,

    #[error("permission denied")]
    PermissionDenied,

    #[error("resource not found: {0}")]
    NotFound(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[allow(dead_code)]
    #[error("internal server error: {0}")]
    InternalError(String),

    #[error("database error")]
    DatabaseError(#[from] sqlx::Error),

    #[error("jwt error")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    #[error("bcrypt error")]
    BcryptError(#[from] bcrypt::BcryptError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, 401, self.to_string()),
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, 401, self.to_string()),
            AppError::TokenExpired => (StatusCode::UNAUTHORIZED, 401, self.to_string()),
            AppError::WechatTokenExpired => (StatusCode::UNAUTHORIZED, 401, self.to_string()),
            AppError::PermissionDenied => (StatusCode::FORBIDDEN, 403, self.to_string()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, 404, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, 400, msg.clone()),
            AppError::InternalError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, 500, self.to_string())
            }
            AppError::DatabaseError(e) => (StatusCode::INTERNAL_SERVER_ERROR, 500, e.to_string()),
            AppError::JwtError(e) => (StatusCode::INTERNAL_SERVER_ERROR, 500, e.to_string()),
            AppError::BcryptError(e) => (StatusCode::INTERNAL_SERVER_ERROR, 500, e.to_string()),
        };

        let body = Json(json!({
            "code": code,
            "data": null,
            "message": message
        }));

        (status, body).into_response()
    }
}
