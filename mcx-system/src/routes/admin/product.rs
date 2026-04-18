use crate::error::AppError;
use crate::routes::admin::auth::authorize_admin;
use crate::routes::admin::permissions::PRODUCT_LIST_VIEW;
use crate::routes::ApiResponse;
use crate::state::AppState;
use axum::{extract::Path, extract::State, Json};
use std::sync::Arc;

use crate::models::{CreateProductRequest, Product, UpdateProductRequest};

pub async fn list_products(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<Vec<Product>>>, AppError> {
    authorize_admin(&state, &headers, &[PRODUCT_LIST_VIEW]).await?;

    let products = sqlx::query_as::<_, Product>("SELECT * FROM products ORDER BY id")
        .fetch_all(&state.db)
        .await?;

    Ok(Json(ApiResponse::success(products)))
}

pub async fn get_product(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<Product>>, AppError> {
    authorize_admin(&state, &headers, &[PRODUCT_LIST_VIEW]).await?;

    let product = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Product not found".to_string()))?;

    Ok(Json(ApiResponse::success(product)))
}

pub async fn create_product(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<CreateProductRequest>,
) -> Result<Json<ApiResponse<Product>>, AppError> {
    authorize_admin(&state, &headers, &[PRODUCT_LIST_VIEW]).await?;

    if body.name.is_empty() {
        return Err(AppError::BadRequest(
            "Product name cannot be empty".to_string(),
        ));
    }
    if body.price < 0 {
        return Err(AppError::BadRequest("Price cannot be negative".to_string()));
    }

    let image_urls_json =
        serde_json::to_string(&body.image_urls).unwrap_or_else(|_| "[]".to_string());

    tracing::info!(
        "Creating product: name={}, price={}, image_urls={}",
        body.name,
        body.price,
        image_urls_json
    );

    // Default to 上架 (on sale) if status not provided
    let status: i32 = 1; // 上架 by default

    // Use transaction to ensure LAST_INSERT_ID() works correctly
    let mut tx = state.db.begin().await?;

    // Insert without RETURNING (MySQL doesn't support it)
    let result = sqlx::query(
        "INSERT INTO products (name, description, price, image_urls, category, status) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&body.name)
    .bind(&body.description)
    .bind(body.price)
    .bind(&image_urls_json)
    .bind(&body.category)
    .bind(status)
    .execute(&mut *tx)
    .await?;

    tracing::info!(
        "Product inserted, rows affected: {}",
        result.rows_affected()
    );

    // Get last inserted id - this must be in the same transaction
    let last_id: u64 = sqlx::query_scalar("SELECT LAST_INSERT_ID()")
        .fetch_one(&mut *tx)
        .await?;

    tracing::info!("Last insert id: {}", last_id);

    // Commit transaction
    tx.commit().await?;

    // Fetch the inserted product
    let product: Product = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = ?")
        .bind(last_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::InternalError(
            "Failed to fetch created product".to_string(),
        ))?;

    Ok(Json(ApiResponse::success(product)))
}

pub async fn update_product(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
    Json(body): Json<UpdateProductRequest>,
) -> Result<Json<ApiResponse<Product>>, AppError> {
    authorize_admin(&state, &headers, &[PRODUCT_LIST_VIEW]).await?;

    let existing = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("Product not found".to_string()))?;

    let new_price = body.price.unwrap_or(existing.price);
    if new_price < 0 {
        return Err(AppError::BadRequest("Price cannot be negative".to_string()));
    }

    // MySQL doesn't support COALESCE for JSON, so we need to handle it in code
    let name = body.name.as_ref().unwrap_or(&existing.name).clone();
    let description = body.description.clone().or(existing.description.clone());
    let image_urls = match &body.image_urls {
        Some(urls) => serde_json::to_string(urls).unwrap_or_else(|_| "[]".to_string()),
        None => existing.image_urls.clone(),
    };
    let category = body.category.clone().or(existing.category.clone());
    let status = body.status.unwrap_or(existing.status);

    sqlx::query(
        "UPDATE products SET name = ?, description = ?, price = ?, image_urls = ?, category = ?, status = ? WHERE id = ?"
    )
    .bind(&name)
    .bind(&description)
    .bind(new_price)
    .bind(&image_urls)
    .bind(&category)
    .bind(status)
    .bind(id)
    .execute(&state.db)
    .await?;

    // Fetch updated product
    let product = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(ApiResponse::success(product)))
}

pub async fn delete_product(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    authorize_admin(&state, &headers, &[PRODUCT_LIST_VIEW]).await?;

    let result = sqlx::query("DELETE FROM products WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Product not found".to_string()));
    }

    Ok(Json(ApiResponse::success(())))
}
