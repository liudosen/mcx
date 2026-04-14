use crate::error::AppError;
use crate::routes::mini_app::auth::validate_wechat_user;
use crate::routes::ApiResponse;
use crate::state::AppState;
use axum::{extract::Path, extract::State, Json};
use std::sync::Arc;

use crate::models::address::{Address, CreateAddressRequest, UpdateAddressRequest};

/// 获取用户的所有地址
pub async fn list_addresses(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<Vec<Address>>>, AppError> {
    let user_id = validate_wechat_user(&state, &headers).await?;

    let addresses = sqlx::query_as::<_, Address>(
        "SELECT * FROM addresses WHERE open_id = ? ORDER BY is_default DESC, id DESC",
    )
    .bind(&user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ApiResponse::success(addresses)))
}

/// 获取单个地址
pub async fn get_address(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<Address>>, AppError> {
    let user_id = validate_wechat_user(&state, &headers).await?;

    let address = sqlx::query_as::<_, Address>("SELECT * FROM addresses WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Address".to_string()))?;

    // 验证地址属于当前用户
    if address.open_id != user_id {
        return Err(AppError::PermissionDenied);
    }

    Ok(Json(ApiResponse::success(address)))
}

/// 创建地址
pub async fn create_address(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CreateAddressRequest>,
) -> Result<Json<ApiResponse<Address>>, AppError> {
    let user_id = validate_wechat_user(&state, &headers).await?;

    let label = payload.label.unwrap_or_else(|| "其他".to_string());
    let is_default = payload.is_default.unwrap_or(false);

    // 如果设为默认，先取消该用户的其他默认地址
    if is_default {
        sqlx::query("UPDATE addresses SET is_default = 0 WHERE open_id = ?")
            .bind(&user_id)
            .execute(&state.db)
            .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO addresses (open_id, receiver_name, phone, province, city, district, detail_address, label, is_default)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&user_id)
    .bind(&payload.receiver_name)
    .bind(&payload.phone)
    .bind(&payload.province)
    .bind(&payload.city)
    .bind(&payload.district)
    .bind(&payload.detail_address)
    .bind(&label)
    .bind(is_default)
    .execute(&state.db)
    .await?;

    let address = sqlx::query_as::<_, Address>(
        "SELECT * FROM addresses WHERE open_id = ? ORDER BY id DESC LIMIT 1",
    )
    .bind(&user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(ApiResponse::success(address)))
}

/// 更新地址
pub async fn update_address(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<UpdateAddressRequest>,
) -> Result<Json<ApiResponse<Address>>, AppError> {
    let user_id = validate_wechat_user(&state, &headers).await?;

    // 检查地址存在且属于当前用户
    let existing = sqlx::query_as::<_, Address>("SELECT * FROM addresses WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Address".to_string()))?;

    if existing.open_id != user_id {
        return Err(AppError::PermissionDenied);
    }

    // 如果设为默认，先取消该用户的其他默认地址
    if payload.is_default == Some(true) {
        sqlx::query("UPDATE addresses SET is_default = 0 WHERE open_id = ? AND id != ?")
            .bind(user_id)
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    let receiver_name = payload.receiver_name.unwrap_or(existing.receiver_name);
    let phone = payload.phone.unwrap_or(existing.phone);
    let province = payload.province.unwrap_or(existing.province);
    let city = payload.city.unwrap_or(existing.city);
    let district = payload.district.unwrap_or(existing.district);
    let detail_address = payload.detail_address.unwrap_or(existing.detail_address);
    let label = payload.label.unwrap_or(existing.label);
    let is_default = payload.is_default.unwrap_or(existing.is_default);

    sqlx::query(
        r#"
        UPDATE addresses
        SET receiver_name = ?, phone = ?, province = ?, city = ?, district = ?, detail_address = ?, label = ?, is_default = ?
        WHERE id = ?
        "#,
    )
    .bind(&receiver_name)
    .bind(&phone)
    .bind(&province)
    .bind(&city)
    .bind(&district)
    .bind(&detail_address)
    .bind(&label)
    .bind(is_default)
    .bind(id)
    .execute(&state.db)
    .await?;

    let address = sqlx::query_as::<_, Address>("SELECT * FROM addresses WHERE id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(ApiResponse::success(address)))
}

/// 删除地址
pub async fn delete_address(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let user_id = validate_wechat_user(&state, &headers).await?;

    // 检查地址存在且属于当前用户
    let existing = sqlx::query_as::<_, Address>("SELECT * FROM addresses WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Address".to_string()))?;

    if existing.open_id != user_id {
        return Err(AppError::PermissionDenied);
    }

    sqlx::query("DELETE FROM addresses WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(ApiResponse::success("deleted".to_string())))
}

/// 设置默认地址
pub async fn set_default_address(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<crate::models::address::SetDefaultRequest>,
) -> Result<Json<ApiResponse<Address>>, AppError> {
    let user_id = validate_wechat_user(&state, &headers).await?;

    // 检查地址存在且属于当前用户
    let existing = sqlx::query_as::<_, Address>("SELECT * FROM addresses WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Address".to_string()))?;

    if existing.open_id != user_id {
        return Err(AppError::PermissionDenied);
    }

    if payload.is_default {
        // 取消该用户的其他默认地址
        sqlx::query("UPDATE addresses SET is_default = 0 WHERE open_id = ?")
            .bind(user_id)
            .execute(&state.db)
            .await?;
    }

    sqlx::query("UPDATE addresses SET is_default = ? WHERE id = ?")
        .bind(payload.is_default)
        .bind(id)
        .execute(&state.db)
        .await?;

    let address = sqlx::query_as::<_, Address>("SELECT * FROM addresses WHERE id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(ApiResponse::success(address)))
}
