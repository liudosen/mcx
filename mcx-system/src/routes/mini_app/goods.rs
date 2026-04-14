use crate::error::AppError;
use crate::models::{
    build_goods_detail, build_goods_list_item, GoodsDetail, GoodsListItem, GoodsRow, GoodsSkuRow,
};
use crate::routes::ApiResponse;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListGoodsQuery {
    pub page_index: Option<u64>,
    pub page_size: Option<u64>,
    pub category_id: Option<String>,
    pub keyword: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoodsListResponse {
    pub list: Vec<GoodsListItem>,
    pub total: i64,
    pub page_index: u64,
    pub page_size: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailQuery {
    pub spu_id: String,
}

pub async fn list_goods(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListGoodsQuery>,
) -> Result<Json<ApiResponse<GoodsListResponse>>, AppError> {
    let page_index = q.page_index.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).min(100);
    let offset = (page_index - 1) * page_size;

    let category_id = q.category_id.clone();
    let keyword = q.keyword.clone();

    let mut conditions = vec!["status = 1"];
    if category_id.is_some() {
        conditions.push("category_id = ?");
    }
    if keyword.is_some() {
        conditions.push("title LIKE ?");
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

    let list = rows.iter().map(build_goods_list_item).collect();

    Ok(Json(ApiResponse::success(GoodsListResponse {
        list,
        total,
        page_index,
        page_size,
    })))
}

pub async fn get_goods_detail(
    State(state): State<Arc<AppState>>,
    Query(q): Query<DetailQuery>,
) -> Result<Json<ApiResponse<GoodsDetail>>, AppError> {
    let spu_id: u64 = q
        .spu_id
        .parse()
        .map_err(|_| AppError::BadRequest("spuId 格式错误".to_string()))?;

    let row = sqlx::query_as::<_, GoodsRow>(
        "SELECT id, store_id, saas_id, title, primary_image, images, desc_images, spec_list, \
         min_sale_price, max_line_price, spu_tag_list, is_sold_out, spu_stock_quantity, sold_num, \
         category_id, status FROM goods WHERE id = ? AND status = 1",
    )
    .bind(spu_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("商品不存在".to_string()))?;

    let skus: Vec<GoodsSkuRow> = sqlx::query_as::<_, GoodsSkuRow>(
        "SELECT id, spu_id, sku_image, spec_info, sale_price, line_price, stock_quantity \
         FROM goods_skus WHERE spu_id = ? ORDER BY id",
    )
    .bind(spu_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ApiResponse::success(build_goods_detail(&row, skus))))
}
