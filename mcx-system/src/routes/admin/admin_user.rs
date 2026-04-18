use crate::error::AppError;
use crate::models::{
    AdminUser, AdminUserListItem, AdminUserListResponse, PermissionCatalogResponse,
    UpdateAdminUserPermissionsRequest,
};
use crate::routes::admin::auth::authorize_admin;
use crate::routes::admin::permissions::{all_permission_codes, permission_catalog, ADMIN_USER_VIEW};
use crate::routes::ApiResponse;
use crate::state::AppState;
use axum::{extract::Path, extract::State, Json};
use std::sync::Arc;

fn parse_permission_codes(raw: &str) -> Vec<String> {
    if raw.trim().is_empty() {
        return Vec::new();
    }

    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

pub async fn list_permissions(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<PermissionCatalogResponse>>, AppError> {
    authorize_admin(&state, &headers, &[ADMIN_USER_VIEW]).await?;
    Ok(Json(ApiResponse::success(permission_catalog())))
}

pub async fn list_admin_users(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<AdminUserListResponse>>, AppError> {
    authorize_admin(&state, &headers, &[ADMIN_USER_VIEW]).await?;

    let users = sqlx::query_as::<_, AdminUser>(
        r#"
        SELECT id, username, password_hash, role, COALESCE(permission_codes, '[]') as permission_codes,
               is_active, created_at, updated_at
        FROM admin_users
        ORDER BY id DESC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let list = users
        .into_iter()
        .map(|user| AdminUserListItem {
            id: user.id,
            username: user.username,
            role: user.role,
            permission_codes: parse_permission_codes(&user.permission_codes),
            is_active: user.is_active,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(AdminUserListResponse { list })))
}

pub async fn update_admin_user_permissions(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<UpdateAdminUserPermissionsRequest>,
) -> Result<Json<ApiResponse<AdminUserListItem>>, AppError> {
    authorize_admin(&state, &headers, &[ADMIN_USER_VIEW]).await?;

    let existing = sqlx::query_as::<_, AdminUser>(
        r#"
        SELECT id, username, password_hash, role, COALESCE(permission_codes, '[]') as permission_codes,
               is_active, created_at, updated_at
        FROM admin_users
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("Admin user".to_string()))?;

    let next_role = payload.role.clone().unwrap_or(existing.role.clone());
    let stored_permissions = if next_role == "admin" {
        serde_json::to_string(&all_permission_codes()).unwrap_or_else(|_| "[]".to_string())
    } else {
        serde_json::to_string(&payload.permission_codes).unwrap_or_else(|_| "[]".to_string())
    };

    let is_active = payload.is_active.unwrap_or(existing.is_active);

    sqlx::query(
        "UPDATE admin_users SET role = ?, is_active = ?, permission_codes = ? WHERE id = ?",
    )
    .bind(&next_role)
    .bind(is_active)
    .bind(&stored_permissions)
    .bind(id)
    .execute(&state.db)
    .await?;

    let updated = sqlx::query_as::<_, AdminUser>(
        r#"
        SELECT id, username, password_hash, role, COALESCE(permission_codes, '[]') as permission_codes,
               is_active, created_at, updated_at
        FROM admin_users
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(ApiResponse::success(AdminUserListItem {
        id: updated.id,
        username: updated.username,
        role: updated.role,
        permission_codes: parse_permission_codes(&updated.permission_codes),
        is_active: updated.is_active,
        created_at: updated.created_at,
        updated_at: updated.updated_at,
    })))
}
