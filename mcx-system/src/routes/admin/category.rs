use crate::error::AppError;
use crate::models::{CreateCategoryRequest, GoodsCategory, UpdateCategoryRequest};
use crate::routes::admin::auth::authorize_admin;
use crate::routes::admin::permissions::CATEGORY_LIST_VIEW;
use crate::routes::ApiResponse;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

pub async fn list_categories(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<Vec<GoodsCategory>>>, AppError> {
    authorize_admin(&state, &headers, &[CATEGORY_LIST_VIEW]).await?;

    let list = sqlx::query_as::<_, GoodsCategory>(
        "SELECT id, name, sort_order, status, (SELECT COUNT(*) FROM goods WHERE category_id = goods_categories.id) AS goods_count FROM goods_categories ORDER BY sort_order ASC, id ASC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ApiResponse::success(list)))
}

pub async fn create_category(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<CreateCategoryRequest>,
) -> Result<Json<ApiResponse<GoodsCategory>>, AppError> {
    authorize_admin(&state, &headers, &[CATEGORY_LIST_VIEW]).await?;

    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("分类名称不能为空".to_string()));
    }

    let sort_order = body.sort_order.unwrap_or(0);

    let mut tx = state.db.begin().await?;
    sqlx::query("INSERT INTO goods_categories (name, sort_order, status) VALUES (?, ?, 1)")
        .bind(&name)
        .bind(sort_order)
        .execute(&mut *tx)
        .await?;
    let id: u64 = sqlx::query_scalar("SELECT LAST_INSERT_ID()")
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;

    let cat = sqlx::query_as::<_, GoodsCategory>(
        "SELECT id, name, sort_order, status, (SELECT COUNT(*) FROM goods WHERE category_id = goods_categories.id) AS goods_count FROM goods_categories WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(ApiResponse::success(cat)))
}

pub async fn update_category(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
    Json(body): Json<UpdateCategoryRequest>,
) -> Result<Json<ApiResponse<GoodsCategory>>, AppError> {
    authorize_admin(&state, &headers, &[CATEGORY_LIST_VIEW]).await?;

    let existing = sqlx::query_as::<_, GoodsCategory>(
        "SELECT id, name, sort_order, status, (SELECT COUNT(*) FROM goods WHERE category_id = goods_categories.id) AS goods_count FROM goods_categories WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("分类不存在".to_string()))?;

    let name = body
        .name
        .map(|s| s.trim().to_string())
        .unwrap_or(existing.name);
    if name.is_empty() {
        return Err(AppError::BadRequest("分类名称不能为空".to_string()));
    }
    let sort_order = body.sort_order.unwrap_or(existing.sort_order);
    let status = body.status.unwrap_or(existing.status);

    sqlx::query("UPDATE goods_categories SET name = ?, sort_order = ?, status = ? WHERE id = ?")
        .bind(&name)
        .bind(sort_order)
        .bind(status)
        .bind(id)
        .execute(&state.db)
        .await?;

    let cat = sqlx::query_as::<_, GoodsCategory>(
        "SELECT id, name, sort_order, status, (SELECT COUNT(*) FROM goods WHERE category_id = goods_categories.id) AS goods_count FROM goods_categories WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(ApiResponse::success(cat)))
}

pub async fn delete_category(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    authorize_admin(&state, &headers, &[CATEGORY_LIST_VIEW]).await?;

    // 检查是否有商品在用
    let in_use: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM goods WHERE category_id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await?;
    if in_use > 0 {
        return Err(AppError::BadRequest(format!(
            "该分类下还有 {} 个商品，请先移除商品后再删除",
            in_use
        )));
    }

    let result = sqlx::query("DELETE FROM goods_categories WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("分类不存在".to_string()));
    }

    Ok(Json(ApiResponse::success(())))
}
