use crate::error::AppError;
use crate::models::{
    build_admin_goods_detail, AdminGoodsDetail, CreateGoodsRequest, GoodsRow, GoodsSkuRow,
    SkuRequest, UpdateGoodsRequest,
};
use crate::routes::admin::auth::check_admin;
use crate::routes::ApiResponse;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct ListGoodsQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub category_id: Option<String>,
    pub keyword: Option<String>,
    pub status: Option<i32>,
}

#[derive(serde::Serialize)]
pub struct PagedGoods {
    pub list: Vec<AdminGoodsDetail>,
    pub total: i64,
    pub page: u64,
    pub page_size: u64,
}

async fn fetch_skus(state: &AppState, spu_id: u64) -> Result<Vec<GoodsSkuRow>, AppError> {
    let skus = sqlx::query_as::<_, GoodsSkuRow>(
        "SELECT id, spu_id, sku_image, spec_info, sale_price, line_price, stock_quantity \
         FROM goods_skus WHERE spu_id = ? ORDER BY id",
    )
    .bind(spu_id)
    .fetch_all(&state.db)
    .await?;
    Ok(skus)
}

fn compute_stock_stats(skus: &[SkuRequest]) -> (i64, i64, i32, bool) {
    if skus.is_empty() {
        return (0, 0, 0, true);
    }
    let min_sale = skus.iter().map(|s| s.sale_price).min().unwrap_or(0);
    let max_line = skus.iter().map(|s| s.line_price).max().unwrap_or(0);
    let total_stock: i32 = skus.iter().map(|s| s.stock_quantity).sum();
    let is_sold_out = total_stock == 0;
    (min_sale, max_line, total_stock, is_sold_out)
}

pub async fn list_goods(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Query(q): Query<ListGoodsQuery>,
) -> Result<Json<ApiResponse<PagedGoods>>, AppError> {
    check_admin(&state, &headers).await?;

    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).min(100);
    let offset = (page - 1) * page_size;

    // Build dynamic WHERE clauses
    let mut conditions = vec!["1=1"];
    let category_id = q.category_id.clone();
    let keyword = q.keyword.clone();
    let status = q.status;

    if category_id.is_some() {
        conditions.push("category_id = ?");
    }
    if keyword.is_some() {
        conditions.push("title LIKE ?");
    }
    if status.is_some() {
        conditions.push("status = ?");
    }

    let where_clause = conditions.join(" AND ");

    let count_sql = format!("SELECT COUNT(*) FROM goods WHERE {}", where_clause);
    let list_sql = format!(
        "SELECT id, store_id, saas_id, title, primary_image, images, desc_images, spec_list, \
         min_sale_price, max_line_price, spu_tag_list, is_sold_out, spu_stock_quantity, sold_num, \
         category_id, status FROM goods WHERE {} ORDER BY id DESC LIMIT ? OFFSET ?",
        where_clause
    );

    macro_rules! bind_filters {
        ($q:expr) => {{
            let mut q = $q;
            if let Some(ref cid) = category_id {
                q = q.bind(cid);
            }
            if let Some(ref kw) = keyword {
                q = q.bind(format!("%{}%", kw));
            }
            if let Some(st) = status {
                q = q.bind(st);
            }
            q
        }};
    }

    let total: i64 = bind_filters!(sqlx::query_scalar(&count_sql))
        .fetch_one(&state.db)
        .await?;

    let rows: Vec<GoodsRow> = bind_filters!(sqlx::query_as::<_, GoodsRow>(&list_sql))
        .bind(page_size)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;

    let mut list = Vec::with_capacity(rows.len());
    for row in &rows {
        let skus = fetch_skus(&state, row.id).await?;
        list.push(build_admin_goods_detail(row, skus));
    }

    Ok(Json(ApiResponse::success(PagedGoods {
        list,
        total,
        page,
        page_size,
    })))
}

pub async fn get_goods(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<AdminGoodsDetail>>, AppError> {
    check_admin(&state, &headers).await?;

    let row = sqlx::query_as::<_, GoodsRow>(
        "SELECT id, store_id, saas_id, title, primary_image, images, desc_images, spec_list, \
         min_sale_price, max_line_price, spu_tag_list, is_sold_out, spu_stock_quantity, sold_num, \
         category_id, status FROM goods WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("商品不存在".to_string()))?;

    let skus = fetch_skus(&state, id).await?;
    Ok(Json(ApiResponse::success(build_admin_goods_detail(
        &row, skus,
    ))))
}

pub async fn create_goods(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<CreateGoodsRequest>,
) -> Result<Json<ApiResponse<AdminGoodsDetail>>, AppError> {
    check_admin(&state, &headers).await?;

    if body.title.is_empty() {
        return Err(AppError::BadRequest("商品名称不能为空".to_string()));
    }
    if body.primary_image.is_empty() {
        return Err(AppError::BadRequest("主图不能为空".to_string()));
    }

    let (min_sale, max_line, total_stock, is_sold_out) = compute_stock_stats(&body.skus);

    let images_json = serde_json::to_string(&body.images).unwrap_or_else(|_| "[]".to_string());
    let desc_json = serde_json::to_string(&body.desc_images).unwrap_or_else(|_| "[]".to_string());
    let spec_json = serde_json::to_string(&body.spec_list).unwrap_or_else(|_| "[]".to_string());
    let tag_json = serde_json::to_string(&body.spu_tag_list).unwrap_or_else(|_| "[]".to_string());

    let store_id = body.store_id.unwrap_or_default();
    let saas_id = body.saas_id.unwrap_or_default();

    let mut tx = state.db.begin().await?;

    sqlx::query(
        "INSERT INTO goods (store_id, saas_id, title, primary_image, images, desc_images, \
         spec_list, min_sale_price, max_line_price, spu_tag_list, is_sold_out, \
         spu_stock_quantity, sold_num, category_id, status) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, 1)",
    )
    .bind(&store_id)
    .bind(&saas_id)
    .bind(&body.title)
    .bind(&body.primary_image)
    .bind(&images_json)
    .bind(&desc_json)
    .bind(&spec_json)
    .bind(min_sale)
    .bind(max_line)
    .bind(&tag_json)
    .bind(is_sold_out)
    .bind(total_stock)
    .bind(&body.category_id)
    .execute(&mut *tx)
    .await?;

    let spu_id: u64 = sqlx::query_scalar("SELECT LAST_INSERT_ID()")
        .fetch_one(&mut *tx)
        .await?;

    insert_skus(&mut tx, spu_id, &body.skus).await?;
    tx.commit().await?;

    let row = sqlx::query_as::<_, GoodsRow>(
        "SELECT id, store_id, saas_id, title, primary_image, images, desc_images, spec_list, \
         min_sale_price, max_line_price, spu_tag_list, is_sold_out, spu_stock_quantity, sold_num, \
         category_id, status FROM goods WHERE id = ?",
    )
    .bind(spu_id)
    .fetch_one(&state.db)
    .await?;

    let skus = fetch_skus(&state, spu_id).await?;
    Ok(Json(ApiResponse::success(build_admin_goods_detail(
        &row, skus,
    ))))
}

pub async fn update_goods(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
    Json(body): Json<UpdateGoodsRequest>,
) -> Result<Json<ApiResponse<AdminGoodsDetail>>, AppError> {
    check_admin(&state, &headers).await?;

    let existing = sqlx::query_as::<_, GoodsRow>(
        "SELECT id, store_id, saas_id, title, primary_image, images, desc_images, spec_list, \
         min_sale_price, max_line_price, spu_tag_list, is_sold_out, spu_stock_quantity, sold_num, \
         category_id, status FROM goods WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("商品不存在".to_string()))?;

    let store_id = body.store_id.unwrap_or(existing.store_id.clone());
    let saas_id = body.saas_id.unwrap_or(existing.saas_id.clone());
    let title = body.title.unwrap_or(existing.title.clone());
    let primary_image = body.primary_image.unwrap_or(existing.primary_image.clone());
    let images_json = match body.images {
        Some(v) => serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string()),
        None => existing.images.clone(),
    };
    let desc_json = match body.desc_images {
        Some(v) => serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string()),
        None => existing.desc_images.clone(),
    };
    let spec_json = match body.spec_list {
        Some(v) => serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string()),
        None => existing.spec_list.clone(),
    };
    let tag_json = match body.spu_tag_list {
        Some(v) => serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string()),
        None => existing.spu_tag_list.clone(),
    };
    let category_id = body.category_id.or(existing.category_id.clone());
    let status = body.status.unwrap_or(existing.status);

    // Recompute stock stats if SKUs are being replaced
    let (min_sale, max_line, total_stock, is_sold_out) = if let Some(ref skus) = body.skus {
        compute_stock_stats(skus)
    } else {
        (
            existing.min_sale_price,
            existing.max_line_price,
            existing.spu_stock_quantity,
            existing.is_sold_out,
        )
    };

    let mut tx = state.db.begin().await?;

    sqlx::query(
        "UPDATE goods SET store_id=?, saas_id=?, title=?, primary_image=?, images=?, \
         desc_images=?, spec_list=?, min_sale_price=?, max_line_price=?, spu_tag_list=?, \
         is_sold_out=?, spu_stock_quantity=?, category_id=?, status=? WHERE id=?",
    )
    .bind(&store_id)
    .bind(&saas_id)
    .bind(&title)
    .bind(&primary_image)
    .bind(&images_json)
    .bind(&desc_json)
    .bind(&spec_json)
    .bind(min_sale)
    .bind(max_line)
    .bind(&tag_json)
    .bind(is_sold_out)
    .bind(total_stock)
    .bind(&category_id)
    .bind(status)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    if let Some(ref skus) = body.skus {
        sqlx::query("DELETE FROM goods_skus WHERE spu_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        insert_skus(&mut tx, id, skus).await?;
    }

    tx.commit().await?;

    let row = sqlx::query_as::<_, GoodsRow>(
        "SELECT id, store_id, saas_id, title, primary_image, images, desc_images, spec_list, \
         min_sale_price, max_line_price, spu_tag_list, is_sold_out, spu_stock_quantity, sold_num, \
         category_id, status FROM goods WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    let skus = fetch_skus(&state, id).await?;
    Ok(Json(ApiResponse::success(build_admin_goods_detail(
        &row, skus,
    ))))
}

pub async fn delete_goods(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    check_admin(&state, &headers).await?;

    let result = sqlx::query("DELETE FROM goods WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("商品不存在".to_string()));
    }

    Ok(Json(ApiResponse::success(())))
}

async fn insert_skus(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    spu_id: u64,
    skus: &[SkuRequest],
) -> Result<(), AppError> {
    for sku in skus {
        let spec_json = serde_json::to_string(&sku.spec_info).unwrap_or_else(|_| "[]".to_string());
        sqlx::query(
            "INSERT INTO goods_skus (spu_id, sku_image, spec_info, sale_price, line_price, stock_quantity) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(spu_id)
        .bind(&sku.sku_image)
        .bind(&spec_json)
        .bind(sku.sale_price)
        .bind(sku.line_price)
        .bind(sku.stock_quantity)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}
