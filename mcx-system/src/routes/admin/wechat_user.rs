use crate::error::AppError;
use crate::models::wechat_user::{
    AddressWithUser, CreateWechatUserRequest, PaginatedResponse, UpdateWechatUserByOpenidRequest,
    UpdateWechatUserRequest, WechatUser, WechatUserQuery,
};
use crate::routes::admin::auth::authorize_admin;
use crate::routes::admin::permissions::{WECHAT_USER_LIST_VIEW, WECHAT_USER_PASSWORD_VIEW};
use crate::routes::ApiResponse;
use crate::state::AppState;
use axum::{extract::Path, extract::State, Json};
use sqlx::{MySql, QueryBuilder};
use std::sync::Arc;

fn append_wechat_user_filters(builder: &mut QueryBuilder<MySql>, query: &WechatUserQuery) {
    if let Some(ref openid) = query.openid {
        if !openid.is_empty() {
            builder.push(" AND w.openid = ");
            builder.push_bind(openid.clone());
        }
    }

    if let Some(ref phone) = query.phone {
        if !phone.is_empty() {
            let phone_like = format!("%{}%", phone);
            builder.push(" AND (w.phone LIKE ");
            builder.push_bind(phone_like.clone());
            builder.push(" OR a.phone LIKE ");
            builder.push_bind(phone_like);
            builder.push(")");
        }
    }

    if let Some(gender) = query.gender {
        builder.push(" AND w.gender = ");
        builder.push_bind(gender);
    }

    if let Some(ref start_date) = query.start_date {
        if !start_date.is_empty() {
            builder.push(" AND w.created_at >= ");
            builder.push_bind(start_date.clone());
        }
    }

    if let Some(ref end_date) = query.end_date {
        if !end_date.is_empty() {
            builder.push(" AND w.created_at <= ");
            builder.push_bind(end_date.clone());
        }
    }
}

/// Get user list. Addresses are the primary table and wechat_users fills in extra fields.
pub async fn list_wechat_users(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<WechatUserQuery>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<PaginatedResponse<AddressWithUser>>>, AppError> {
    authorize_admin(&state, &headers, &[WECHAT_USER_LIST_VIEW]).await?;

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100);
    let offset = (page - 1) * page_size;

    let mut count_builder = QueryBuilder::<MySql>::new(
        "SELECT COUNT(*) FROM wechat_users w \
         LEFT JOIN addresses a ON w.openid = a.open_id AND a.is_default = 1 \
         WHERE 1=1",
    );
    append_wechat_user_filters(&mut count_builder, &query);
    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(&state.db)
        .await?;

    let mut select_builder = QueryBuilder::<MySql>::new(
        r#"
        SELECT a.id, a.open_id, a.receiver_name, a.phone, a.province, a.city, a.district,
               a.detail_address, a.label, a.is_default, a.created_at, a.updated_at,
               COALESCE(w.real_name, '') as real_name,
               COALESCE(w.avatar_url, '') as avatar_url,
               COALESCE(a.phone, '') as user_phone,
               COALESCE(w.gender, 0) as gender,
               COALESCE(w.id_card_number, '') as id_card_number
        FROM wechat_users w
        LEFT JOIN addresses a ON w.openid = a.open_id AND a.is_default = 1
        WHERE 1=1
        "#,
    );
    append_wechat_user_filters(&mut select_builder, &query);
    select_builder.push(" ORDER BY w.id DESC LIMIT ");
    select_builder.push_bind(page_size as i64);
    select_builder.push(" OFFSET ");
    select_builder.push_bind(offset as i64);

    let addresses = select_builder
        .build_query_as::<AddressWithUser>()
        .fetch_all(&state.db)
        .await?;

    let response = PaginatedResponse::new(addresses, total, page, page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// Get a single user.
pub async fn get_wechat_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<AddressWithUser>>, AppError> {
    authorize_admin(&state, &headers, &[WECHAT_USER_LIST_VIEW]).await?;

    let address = sqlx::query_as::<_, AddressWithUser>(
        r#"
        SELECT a.id, a.open_id, a.receiver_name, a.phone, a.province, a.city, a.district,
               a.detail_address, a.label, a.is_default, a.created_at, a.updated_at,
               COALESCE(w.real_name, '') as real_name,
               COALESCE(w.avatar_url, '') as avatar_url,
               COALESCE(a.phone, '') as user_phone,
               COALESCE(w.gender, 0) as gender,
               COALESCE(w.id_card_number, '') as id_card_number
        FROM wechat_users w
        LEFT JOIN addresses a ON w.openid = a.open_id AND a.is_default = 1
        WHERE w.id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("User".to_string()))?;

    Ok(Json(ApiResponse::success(address)))
}

/// Create a user.
pub async fn create_wechat_user(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CreateWechatUserRequest>,
) -> Result<Json<ApiResponse<WechatUser>>, AppError> {
    authorize_admin(&state, &headers, &[WECHAT_USER_LIST_VIEW]).await?;

    sqlx::query(
        r#"
        INSERT INTO wechat_users (openid, real_name, avatar_url, phone, country, province, city, gender, id_card_number)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&payload.openid)
    .bind(payload.real_name.as_deref().unwrap_or(""))
    .bind(payload.avatar_url.as_deref().unwrap_or(""))
    .bind(payload.phone.as_deref().unwrap_or(""))
    .bind(payload.country.as_deref().unwrap_or(""))
    .bind(payload.province.as_deref().unwrap_or(""))
    .bind(payload.city.as_deref().unwrap_or(""))
    .bind(payload.gender.unwrap_or(0))
    .bind(payload.id_card_number.as_deref().unwrap_or(""))
    .execute(&state.db)
    .await?;

    let user = sqlx::query_as::<_, WechatUser>(
        "SELECT * FROM wechat_users WHERE openid = ? ORDER BY id DESC LIMIT 1",
    )
    .bind(&payload.openid)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(ApiResponse::success(user)))
}

/// Update a user by id.
pub async fn update_wechat_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<UpdateWechatUserRequest>,
) -> Result<Json<ApiResponse<WechatUser>>, AppError> {
    authorize_admin(&state, &headers, &[WECHAT_USER_LIST_VIEW]).await?;

    let existing = sqlx::query_as::<_, WechatUser>("SELECT * FROM wechat_users WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("User".to_string()))?;

    let mut updates: Vec<&str> = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(ref v) = payload.real_name {
        updates.push("real_name = ?");
        values.push(v.clone());
    }
    if let Some(ref v) = payload.avatar_url {
        updates.push("avatar_url = ?");
        values.push(v.clone());
    }
    if let Some(ref v) = payload.phone {
        updates.push("phone = ?");
        values.push(v.clone());
    }
    if let Some(ref v) = payload.country {
        updates.push("country = ?");
        values.push(v.clone());
    }
    if let Some(ref v) = payload.province {
        updates.push("province = ?");
        values.push(v.clone());
    }
    if let Some(ref v) = payload.city {
        updates.push("city = ?");
        values.push(v.clone());
    }
    if let Some(v) = payload.gender {
        updates.push("gender = ?");
        values.push(v.to_string());
    }
    if let Some(ref v) = payload.id_card_number {
        updates.push("id_card_number = ?");
        values.push(v.clone());
    }

    if updates.is_empty() {
        return Ok(Json(ApiResponse::success(existing)));
    }

    let query = format!(
        "UPDATE wechat_users SET {} WHERE id = ?",
        updates.join(", ")
    );

    let mut q = sqlx::query(&query);
    for v in &values {
        q = q.bind(v);
    }
    q = q.bind(id);
    q.execute(&state.db).await?;

    let user = sqlx::query_as::<_, WechatUser>("SELECT * FROM wechat_users WHERE id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(ApiResponse::success(user)))
}

/// Delete a user.
pub async fn delete_wechat_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    authorize_admin(&state, &headers, &[WECHAT_USER_LIST_VIEW]).await?;

    let result = sqlx::query("DELETE FROM wechat_users WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("User".to_string()));
    }

    Ok(Json(ApiResponse::success("deleted".to_string())))
}

/// Update a user by openid.
pub async fn update_wechat_user_by_openid(
    State(state): State<Arc<AppState>>,
    Path(openid): Path<String>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<UpdateWechatUserByOpenidRequest>,
) -> Result<Json<ApiResponse<WechatUser>>, AppError> {
    authorize_admin(&state, &headers, &[WECHAT_USER_LIST_VIEW]).await?;

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

/// Check if a card number exists.
pub async fn check_id_card_number_exists(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CheckIdCardRequest>,
) -> Result<Json<ApiResponse<bool>>, AppError> {
    authorize_admin(&state, &headers, &[WECHAT_USER_LIST_VIEW]).await?;

    if payload.id_card_number.is_empty() {
        return Ok(Json(ApiResponse::success(false)));
    }

    let exists: Option<(u64,)> =
        sqlx::query_as("SELECT id FROM wechat_users WHERE id_card_number = ?")
            .bind(&payload.id_card_number)
            .fetch_optional(&state.db)
            .await?;

    Ok(Json(ApiResponse::success(exists.is_some())))
}

/// Request body for card-number check.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CheckIdCardRequest {
    pub id_card_number: String,
}

/// Get a user's payment password.
pub async fn get_payment_password(
    State(state): State<Arc<AppState>>,
    Path(openid): Path<String>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<crate::models::wechat_user::PaymentPasswordResponse>>, AppError> {
    authorize_admin(&state, &headers, &[WECHAT_USER_PASSWORD_VIEW]).await?;

    let result: Option<(String,)> =
        sqlx::query_as("SELECT payment_password FROM wechat_users WHERE openid = ?")
            .bind(&openid)
            .fetch_optional(&state.db)
            .await?;

    match result {
        Some((password,)) if !password.is_empty() => Ok(Json(ApiResponse::success(
            crate::models::wechat_user::PaymentPasswordResponse {
                payment_password: password,
            },
        ))),
        Some(_) => Ok(Json(ApiResponse::success(
            crate::models::wechat_user::PaymentPasswordResponse {
                payment_password: String::new(),
            },
        ))),
        None => Err(AppError::NotFound("User".to_string())),
    }
}
