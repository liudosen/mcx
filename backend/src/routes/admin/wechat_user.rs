use crate::error::AppError;
use crate::routes::admin::auth::check_admin;
use crate::routes::ApiResponse;
use crate::state::AppState;
use axum::{extract::Path, extract::State, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::wechat_user::{
    AddressWithUser, CreateWechatUserRequest, PaginatedResponse, UpdateWechatUserByOpenidRequest,
    UpdateWechatUserRequest, WechatUser, WechatUserQuery,
};

/// 获取用户列表（以微信用户表为主，地址表填充用户信息）
pub async fn list_wechat_users(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<WechatUserQuery>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<PaginatedResponse<AddressWithUser>>>, AppError> {
    check_admin(&state, &headers).await?;

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100);
    let offset = (page - 1) * page_size;

    // 构建动态查询 - 从微信用户表关联地址表
    let mut conditions = Vec::new();

    if let Some(ref phone) = query.phone {
        if !phone.is_empty() {
            // 手机号匹配地址表或用户表
            conditions.push(format!(
                " AND (w.phone LIKE '%{}%' OR a.phone LIKE '%{}%')",
                phone, phone
            ));
        }
    }
    if let Some(ref start_date) = query.start_date {
        if !start_date.is_empty() {
            conditions.push(format!(" AND w.created_at >= '{}'", start_date));
        }
    }
    if let Some(ref end_date) = query.end_date {
        if !end_date.is_empty() {
            conditions.push(format!(" AND w.created_at <= '{}'", end_date));
        }
    }

    let where_clause = conditions.join("");

    // 统计总数
    let count_sql = format!(
        "SELECT COUNT(*) FROM wechat_users w WHERE 1=1{}",
        where_clause
    );
    let total: i64 = sqlx::query_scalar(&count_sql).fetch_one(&state.db).await?;

    // 查询用户列表，关联地址表填充信息
    let select_sql = format!(
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
        WHERE 1=1 {}
        ORDER BY w.id DESC
        LIMIT {} OFFSET {}
        "#,
        where_clause, page_size, offset
    );

    let addresses = sqlx::query_as::<_, AddressWithUser>(&select_sql)
        .fetch_all(&state.db)
        .await?;

    let response = PaginatedResponse::new(addresses, total, page, page_size);
    Ok(Json(ApiResponse::success(response)))
}

/// 获取单个用户（微信用户表为主，地址表填充信息）
pub async fn get_wechat_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<AddressWithUser>>, AppError> {
    check_admin(&state, &headers).await?;

    // 查询用户，关联地址表填充信息
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

/// 创建用户
pub async fn create_wechat_user(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CreateWechatUserRequest>,
) -> Result<Json<ApiResponse<WechatUser>>, AppError> {
    check_admin(&state, &headers).await?;

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

/// 更新用户
pub async fn update_wechat_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<UpdateWechatUserRequest>,
) -> Result<Json<ApiResponse<WechatUser>>, AppError> {
    check_admin(&state, &headers).await?;

    // 检查用户存在
    let existing = sqlx::query_as::<_, WechatUser>("SELECT * FROM wechat_users WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("User".to_string()))?;

    // 只更新传入的字段（Some表示要更新，None表示不更新）
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

/// 删除用户
pub async fn delete_wechat_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    check_admin(&state, &headers).await?;

    let result = sqlx::query("DELETE FROM wechat_users WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("User".to_string()));
    }

    Ok(Json(ApiResponse::success("deleted".to_string())))
}

/// 通过openid更新用户（主要用于更新身份证号等字段）
pub async fn update_wechat_user_by_openid(
    State(state): State<Arc<AppState>>,
    Path(openid): Path<String>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<UpdateWechatUserByOpenidRequest>,
) -> Result<Json<ApiResponse<WechatUser>>, AppError> {
    check_admin(&state, &headers).await?;

    // 检查用户存在
    let existing = sqlx::query_as::<_, WechatUser>("SELECT * FROM wechat_users WHERE openid = ?")
        .bind(&openid)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("User".to_string()))?;

    // 只更新传入的字段（Some表示要更新，None表示不更新）
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

/// 检查身份证号是否已存在
pub async fn check_id_card_number_exists(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CheckIdCardRequest>,
) -> Result<Json<ApiResponse<bool>>, AppError> {
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

/// 检查身份证号请求
#[derive(Debug, Clone, Deserialize)]
pub struct CheckIdCardRequest {
    pub id_card_number: String,
}

/// 获取用户支付密码
pub async fn get_payment_password(
    State(state): State<Arc<AppState>>,
    Path(openid): Path<String>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<crate::models::wechat_user::PaymentPasswordResponse>>, AppError> {
    check_admin(&state, &headers).await?;

    let result: Option<(String,)> =
        sqlx::query_as("SELECT payment_password FROM wechat_users WHERE openid = ?")
            .bind(&openid)
            .fetch_optional(&state.db)
            .await?;

    match result {
        Some((password,)) => Ok(Json(ApiResponse::success(
            crate::models::wechat_user::PaymentPasswordResponse {
                payment_password: password,
            },
        ))),
        None => Err(AppError::NotFound("User".to_string())),
    }
}
