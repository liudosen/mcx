use crate::error::AppError;
use crate::models::WechatUser;
use crate::routes::ApiResponse;
use crate::state::AppState;
use axum::{extract::State, Json};
use chrono::{Duration, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

static DEV_TOKEN_STORE: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Deserialize)]
pub struct WechatLoginRequest {
    pub code: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WechatCode2SessionResponse {
    pub openid: Option<String>,
    pub session_key: Option<String>,
    pub unionid: Option<String>,
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WechatLoginResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub token_type: String,
    pub user: WechatUserInfo,
}

#[derive(Debug, Serialize)]
pub struct WechatUserInfo {
    pub id: u64,
    pub openid: String,
    pub real_name: String,
    pub avatar_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WechatClaims {
    pub sub: String,
    pub wechat_id: u64,
    pub openid: String,
    pub exp: usize,
    pub iat: usize,
}

pub async fn wechat_login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<WechatLoginRequest>,
) -> Result<Json<ApiResponse<WechatLoginResponse>>, AppError> {
    tracing::info!("Wechat login attempt with code: {}", payload.code);

    let openid = if let Some(dev_openid) = state.dev_wechat_openid.clone() {
        tracing::warn!(
            "DEV_WECHAT_OPENID is set, skipping WeChat code exchange and using openid: {}",
            dev_openid
        );
        dev_openid
    } else {
        let wechat_url = format!(
            "https://api.weixin.qq.com/sns/jscode2session?appid={}&secret={}&js_code={}&grant_type=authorization_code",
            state.wechat_appid, state.wechat_secret, payload.code
        );

        let client = reqwest::Client::new();
        let response = client.get(&wechat_url).send().await.map_err(|e| {
            tracing::error!("Failed to call wechat api: {}", e);
            AppError::InternalError("Failed to call wechat api".to_string())
        })?;

        let wechat_resp: WechatCode2SessionResponse = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse wechat response: {}", e);
            AppError::InternalError("Failed to parse wechat response".to_string())
        })?;

        if let Some(errcode) = wechat_resp.errcode {
            if errcode != 0 {
                tracing::error!(
                    "Wechat api error: {} - {}",
                    errcode,
                    wechat_resp.errmsg.clone().unwrap_or_default()
                );
                return Err(AppError::BadRequest(format!(
                    "Wechat api error: {}",
                    wechat_resp.errmsg.unwrap_or_default()
                )));
            }
        }

        wechat_resp.openid.ok_or_else(|| {
            tracing::error!("Wechat response missing openid");
            AppError::InternalError("Wechat response missing openid".to_string())
        })?
    };

    tracing::info!("Wechat login success for openid: {}", openid);

    let user = find_or_create_wechat_user(&state, &openid).await?;

    sqlx::query("UPDATE wechat_users SET last_login_at = NOW() WHERE id = ?")
        .bind(user.id)
        .execute(&state.db)
        .await?;

    let token = create_wechat_token(&state, user.id, &openid)?;

    if state.dev_wechat_openid.is_some() {
        DEV_TOKEN_STORE
            .lock()
            .expect("dev token store poisoned")
            .insert(token.clone(), openid.clone());
    } else {
        let wechat_key = format!("welfare:wechat:token:{}", token);
        let expiry_seconds = 30 * 24 * 3600;
        redis::cmd("SETEX")
            .arg(&wechat_key)
            .arg(expiry_seconds)
            .arg(user.id.to_string())
            .query_async::<_, ()>(&mut state.redis.clone())
            .await
            .map_err(|e| {
                tracing::error!("Redis error storing wechat token: {}", e);
                AppError::InternalError("Failed to store session".to_string())
            })?;
    }

    let user_info = WechatUserInfo {
        id: user.id,
        openid: user.openid,
        real_name: user.real_name,
        avatar_url: user.avatar_url,
    };

    Ok(Json(ApiResponse::success(WechatLoginResponse {
        access_token: token,
        expires_in: 30 * 24 * 3600,
        token_type: "Bearer".to_string(),
        user: user_info,
    })))
}

async fn find_or_create_wechat_user(
    state: &AppState,
    openid: &str,
) -> Result<WechatUser, AppError> {
    let existing = sqlx::query_as::<_, WechatUser>("SELECT * FROM wechat_users WHERE openid = ?")
        .bind(openid)
        .fetch_optional(&state.db)
        .await?;

    if let Some(user) = existing {
        return Ok(user);
    }

    sqlx::query(
        r#"
        INSERT INTO wechat_users (openid, real_name, avatar_url, phone, country, province, city, gender, status)
        VALUES (?, '', '', '', '', '', '', 0, 1)
        "#,
    )
    .bind(openid)
    .execute(&state.db)
    .await?;

    let user = sqlx::query_as::<_, WechatUser>(
        "SELECT * FROM wechat_users WHERE openid = ? ORDER BY id DESC LIMIT 1",
    )
    .bind(openid)
    .fetch_one(&state.db)
    .await?;

    tracing::info!("Created new wechat user with id: {}", user.id);
    Ok(user)
}

fn create_wechat_token(state: &AppState, wechat_id: u64, openid: &str) -> Result<String, AppError> {
    use jsonwebtoken::{encode, EncodingKey, Header};

    let now = Utc::now();
    let exp = now + Duration::days(30);

    let claims = WechatClaims {
        sub: openid.to_string(),
        wechat_id,
        openid: openid.to_string(),
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

pub async fn get_openid_from_token(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Option<String> {
    use jsonwebtoken::{decode, DecodingKey, Validation};

    let auth_header = headers.get("Authorization")?.to_str().ok()?;
    let token = auth_header.strip_prefix("Bearer ")?;

    if state.dev_wechat_openid.is_some() {
        let store = DEV_TOKEN_STORE.lock().ok()?;
        if !store.contains_key(token) {
            return None;
        }

        return decode::<WechatClaims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .ok()
        .map(|data| data.claims.openid);
    }

    let wechat_key = format!("welfare:wechat:token:{}", token);
    let exists: bool = redis::cmd("EXISTS")
        .arg(&wechat_key)
        .query_async(&mut state.redis.clone())
        .await
        .ok()?;

    if !exists {
        return None;
    }

    match decode::<WechatClaims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(data) => Some(data.claims.openid),
        Err(_) => None,
    }
}

pub async fn validate_wechat_user(
    state: &AppState,
    headers: &axum::http::HeaderMap,
) -> Result<String, AppError> {
    use jsonwebtoken::{decode, DecodingKey, Validation};

    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::InvalidToken)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::InvalidToken)?;

    if state.dev_wechat_openid.is_some() {
        let store = DEV_TOKEN_STORE
            .lock()
            .map_err(|_| AppError::InternalError("Failed to check session".to_string()))?;
        if !store.contains_key(token) {
            return Err(AppError::WechatTokenExpired);
        }

        let token_data = decode::<WechatClaims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::InvalidToken)?;

        return Ok(token_data.claims.openid);
    }

    let wechat_key = format!("welfare:wechat:token:{}", token);
    let exists: bool = redis::cmd("EXISTS")
        .arg(&wechat_key)
        .query_async(&mut state.redis.clone())
        .await
        .map_err(|e| {
            tracing::error!("Redis error checking wechat token: {}", e);
            AppError::InternalError("Failed to check session".to_string())
        })?;

    if !exists {
        return Err(AppError::WechatTokenExpired);
    }

    let token_data = decode::<WechatClaims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::InvalidToken)?;

    Ok(token_data.claims.openid)
}

pub async fn check_my_id_card(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<Option<String>>>, AppError> {
    let openid = get_openid_from_token(&state, &headers)
        .await
        .ok_or(AppError::WechatTokenExpired)?;

    let id_card: Option<String> =
        sqlx::query_scalar("SELECT id_card_number FROM wechat_users WHERE openid = ?")
            .bind(&openid)
            .fetch_optional(&state.db)
            .await?;

    let result = id_card.filter(|s| !s.is_empty());

    Ok(Json(ApiResponse::success(result)))
}

pub async fn get_my_userinfo(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<WechatUser>>, AppError> {
    let openid = get_openid_from_token(&state, &headers)
        .await
        .ok_or(AppError::WechatTokenExpired)?;

    let user = sqlx::query_as::<_, WechatUser>("SELECT * FROM wechat_users WHERE openid = ?")
        .bind(&openid)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("User".to_string()))?;

    Ok(Json(ApiResponse::success(user)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateMyUserRequest {
    pub real_name: Option<String>,
    pub phone: Option<String>,
    pub id_card_number: Option<String>,
}

pub async fn update_my_userinfo(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<UpdateMyUserRequest>,
) -> Result<Json<ApiResponse<WechatUser>>, AppError> {
    let openid = get_openid_from_token(&state, &headers)
        .await
        .ok_or(AppError::WechatTokenExpired)?;

    let existing = sqlx::query_as::<_, WechatUser>("SELECT * FROM wechat_users WHERE openid = ?")
        .bind(&openid)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("User".to_string()))?;

    let mut updates: Vec<&str> = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(ref v) = payload.real_name {
        updates.push("real_name = ?");
        values.push(v.clone());
    }
    if let Some(ref v) = payload.phone {
        updates.push("phone = ?");
        values.push(v.clone());
    }
    if let Some(ref v) = payload.id_card_number {
        updates.push("id_card_number = ?");
        values.push(v.clone());
    }

    if updates.is_empty() {
        return Ok(Json(ApiResponse::success(existing)));
    }

    let query = format!(
        "UPDATE wechat_users SET {} WHERE openid = ?",
        updates.join(", ")
    );

    let mut q = sqlx::query(&query);
    for v in &values {
        q = q.bind(v);
    }
    q = q.bind(&openid);
    q.execute(&state.db).await?;

    let user = sqlx::query_as::<_, WechatUser>("SELECT * FROM wechat_users WHERE openid = ?")
        .bind(&openid)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(ApiResponse::success(user)))
}
