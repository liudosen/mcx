use crate::error::AppError;
use crate::models::{AccessCodesResponse, AdminUser, LoginResponse};
use crate::routes::admin::permissions::all_permission_codes;
use crate::routes::ApiResponse;
use crate::state::AppState;
use axum::{extract::State, http::HeaderMap, Json};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

#[derive(Debug, Clone)]
pub struct AdminSession {
    pub admin: AdminUser,
    pub codes: Vec<String>,
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

async fn load_admin_session(state: &AppState, token: &str) -> Result<AdminSession, AppError> {
    if !check_token_exists(state, token).await? {
        return Err(AppError::TokenExpired);
    }

    let claims = validate_token(state, token)?;
    let admin = sqlx::query_as::<_, AdminUser>(
        r#"
        SELECT id, username, password_hash, role, COALESCE(permission_codes, '[]') as permission_codes,
               is_active, created_at, updated_at
        FROM admin_users
        WHERE id = ?
        "#,
    )
    .bind(claims.admin_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::InvalidToken)?;

    if !admin.is_active {
        return Err(AppError::PermissionDenied);
    }

    let codes = if admin.role == "admin" {
        all_permission_codes()
    } else {
        serde_json::from_str::<Vec<String>>(&admin.permission_codes).unwrap_or_default()
    };

    Ok(AdminSession { admin, codes })
}

pub async fn authorize_admin(
    state: &AppState,
    headers: &HeaderMap,
    required: &[&str],
) -> Result<AdminSession, AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::InvalidToken)?;

    let token = extract_token(auth_header).ok_or(AppError::InvalidToken)?;
    let session = load_admin_session(state, token).await?;

    if !required.is_empty() {
        let granted: HashSet<&str> = session.codes.iter().map(|code| code.as_str()).collect();
        let missing = required.iter().find(|code| !granted.contains(**code));
        if missing.is_some() {
            return Err(AppError::PermissionDenied);
        }
    }

    Ok(session)
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginBody>,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    tracing::info!("Login attempt for user: {}", body.username);

    let admin = sqlx::query_as::<_, AdminUser>(
        r#"
        SELECT id, username, password_hash, role, COALESCE(permission_codes, '[]') as permission_codes,
               is_active, created_at, updated_at
        FROM admin_users
        WHERE username = ?
        "#,
    )
    .bind(&body.username)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::InvalidCredentials)?;

    if !admin.is_active {
        return Err(AppError::InvalidCredentials);
    }

    let is_valid = bcrypt::verify(&body.password, &admin.password_hash)?;
    if !is_valid {
        tracing::warn!("Invalid password for user: {}", body.username);
        return Err(AppError::InvalidCredentials);
    }

    let token = create_token(&state, admin.id as u64, &body.username, &admin.role)?;
    store_token(&state, &token, admin.id as u64).await?;

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
    let session = load_admin_session(&state, token).await?;

    let new_token = create_token(
        &state,
        session.admin.id as u64,
        &session.admin.username,
        &session.admin.role,
    )?;

    store_token(&state, &new_token, session.admin.id as u64).await?;

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
    let session = load_admin_session(&state, token).await?;
    let username = session.admin.username.clone();
    let role = session.admin.role.clone();
    let is_admin = role == "admin";

    let codes = if is_admin {
        all_permission_codes()
    } else {
        session.codes.clone()
    };

    Ok(Json(ApiResponse::success(AccessCodesResponse {
        username,
        role,
        is_admin,
        codes,
    })))
}
