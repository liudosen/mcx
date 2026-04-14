use crate::error::AppError;
use crate::models::{AccessCodesResponse, LoginResponse};
use crate::routes::ApiResponse;
use crate::state::AppState;
use axum::{extract::State, http::HeaderMap, Json};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const TOKEN_PREFIX: &str = "welfare:token:";

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub admin_id: u64,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

fn create_token(
    state: &AppState,
    admin_id: u64,
    username: &str,
    role: &str,
) -> Result<String, AppError> {
    use jsonwebtoken::{encode, EncodingKey, Header};

    let now = Utc::now();
    let exp = now + Duration::hours(state.jwt_expiry_hours);

    let claims = Claims {
        sub: username.to_string(),
        admin_id,
        role: role.to_string(),
        exp: exp.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )?;

    Ok(token)
}

pub fn validate_token(state: &AppState, token: &str) -> Result<Claims, AppError> {
    use jsonwebtoken::{decode, DecodingKey, Validation};

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}

pub fn extract_token(auth_header: &str) -> Option<&str> {
    auth_header.strip_prefix("Bearer ")
}

// Store token in Redis with expiry
async fn store_token(state: &AppState, token: &str, admin_id: u64) -> Result<(), AppError> {
    let key = format!("{}{}", TOKEN_PREFIX, token);
    let expiry_seconds = state.jwt_expiry_hours * 3600;

    redis::cmd("SETEX")
        .arg(&key)
        .arg(expiry_seconds)
        .arg(admin_id.to_string())
        .query_async::<_, ()>(&mut state.redis.clone())
        .await
        .map_err(|e| {
            tracing::error!("Redis error storing token: {}", e);
            AppError::InternalError("Failed to store session".to_string())
        })?;

    Ok(())
}

// Check if token exists in Redis
pub async fn check_token_exists(state: &AppState, token: &str) -> Result<bool, AppError> {
    let key = format!("{}{}", TOKEN_PREFIX, token);

    let exists: bool = redis::cmd("EXISTS")
        .arg(&key)
        .query_async::<_, bool>(&mut state.redis.clone())
        .await
        .map_err(|e| {
            tracing::error!("Redis error checking token: {}", e);
            AppError::InternalError("Failed to check session".to_string())
        })?;

    Ok(exists)
}

// Remove token from Redis
async fn remove_token(state: &AppState, token: &str) -> Result<(), AppError> {
    let key = format!("{}{}", TOKEN_PREFIX, token);

    redis::cmd("DEL")
        .arg(&key)
        .query_async::<_, ()>(&mut state.redis.clone())
        .await
        .map_err(|e| {
            tracing::error!("Redis error removing token: {}", e);
            AppError::InternalError("Failed to remove session".to_string())
        })?;

    Ok(())
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginBody>,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    tracing::info!("Login attempt for user: {}", body.username);

    let admin = sqlx::query_as::<_, (u64, String, String, String, bool)>(
        "SELECT id, username, password_hash, role, is_active FROM admin_users WHERE username = ?",
    )
    .bind(&body.username)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::UserNotFound)?;

    let (admin_id, _username, password_hash, role, is_active) = admin;

    if !is_active {
        return Err(AppError::UserInactive);
    }

    let is_valid = bcrypt::verify(&body.password, &password_hash)?;
    if !is_valid {
        tracing::warn!("Invalid password for user: {}", body.username);
        return Err(AppError::InvalidCredentials);
    }

    let token = create_token(&state, admin_id, &body.username, &role)?;

    // Store token in Redis with expiry
    store_token(&state, &token, admin_id).await?;

    let expires_in = state.jwt_expiry_hours * 3600;

    tracing::info!("User logged in successfully: {}", body.username);

    Ok(Json(ApiResponse::success(LoginResponse {
        access_token: token,
        expires_in,
        token_type: "Bearer".to_string(),
    })))
}

pub async fn refresh(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::InvalidToken)?;

    let token = extract_token(auth_header).ok_or(AppError::InvalidToken)?;

    // Check if token exists in Redis
    if !check_token_exists(&state, token).await? {
        return Err(AppError::TokenExpired);
    }

    let claims = validate_token(&state, token)?;

    let new_token = create_token(&state, claims.admin_id, &claims.sub, &claims.role)?;

    // Store new token in Redis
    store_token(&state, &new_token, claims.admin_id).await?;

    let expires_in = state.jwt_expiry_hours * 3600;

    Ok(Json(ApiResponse::success(LoginResponse {
        access_token: new_token,
        expires_in,
        token_type: "Bearer".to_string(),
    })))
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let auth_header = headers.get("Authorization").and_then(|v| v.to_str().ok());

    if let Some(header) = auth_header {
        if let Some(token) = extract_token(header) {
            // Remove token from Redis
            let _ = remove_token(&state, token).await;
        }
    }

    Ok(Json(ApiResponse::success(())))
}

pub async fn get_codes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<AccessCodesResponse>>, AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::InvalidToken)?;

    let token = extract_token(auth_header).ok_or(AppError::InvalidToken)?;

    // Check if token exists in Redis
    if !check_token_exists(&state, token).await? {
        return Err(AppError::TokenExpired);
    }

    let claims = validate_token(&state, token)?;

    let codes: Vec<String> = match claims.role.as_str() {
        "admin" => vec![
            "user:read".to_string(),
            "user:write".to_string(),
            "user:delete".to_string(),
            "product:read".to_string(),
            "product:write".to_string(),
            "product:delete".to_string(),
            "order:read".to_string(),
            "order:write".to_string(),
            "inventory:read".to_string(),
            "inventory:write".to_string(),
            "logistics:read".to_string(),
            "logistics:write".to_string(),
        ],
        "operator" => vec![
            "product:read".to_string(),
            "product:write".to_string(),
            "order:read".to_string(),
            "order:write".to_string(),
            "inventory:read".to_string(),
            "inventory:write".to_string(),
            "logistics:read".to_string(),
            "logistics:write".to_string(),
        ],
        "viewer" => vec![
            "product:read".to_string(),
            "order:read".to_string(),
            "inventory:read".to_string(),
            "logistics:read".to_string(),
        ],
        _ => vec![],
    };

    Ok(Json(ApiResponse::success(AccessCodesResponse { codes })))
}

/// 验证管理员身份
pub async fn check_admin(state: &AppState, headers: &HeaderMap) -> Result<(), AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::InvalidToken)?;

    let token = extract_token(auth_header).ok_or(AppError::InvalidToken)?;

    if !check_token_exists(state, token).await? {
        return Err(AppError::TokenExpired);
    }

    let claims = validate_token(state, token)?;

    if claims.role != "admin" {
        return Err(AppError::PermissionDenied);
    }

    Ok(())
}
