use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("账户或密码错误")]
    InvalidCredentials,

    #[error("账户或密码错误")]
    UserNotFound,

    #[error("账户已被禁用")]
    UserInactive,

    #[error("无效的令牌")]
    InvalidToken,

    #[allow(dead_code)]
    #[error("令牌已过期")]
    TokenExpired,

    #[error("登录已过期，请重新登录")]
    WechatTokenExpired,

    #[error("权限不足")]
    PermissionDenied,

    #[error("资源不存在")]
    NotFound(String),

    #[error("请求参数错误")]
    BadRequest(String),

    #[allow(dead_code)]
    #[error("服务器内部错误")]
    InternalError(String),

    #[error("数据库错误")]
    DatabaseError(#[from] sqlx::Error),

    #[error("令牌验证失败")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    #[error("密码处理失败")]
    BcryptError(#[from] bcrypt::BcryptError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, 401, self.to_string()),
            AppError::UserNotFound => (StatusCode::NOT_FOUND, 404, self.to_string()),
            AppError::UserInactive => (StatusCode::FORBIDDEN, 403, self.to_string()),
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
