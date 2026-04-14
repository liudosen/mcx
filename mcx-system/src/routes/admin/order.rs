use crate::error::AppError;
use crate::models::order::{
    build_order_resp, OrderAddressSnap, OrderItemRow, OrderResp, OrderRow, UpdateOrderStatusRequest,
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
pub struct ListOrdersQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<i8>,
    pub user_id: Option<u64>,
    pub order_no: Option<String>,
}

#[derive(serde::Serialize)]
pub struct PagedOrders {
    pub list: Vec<OrderResp>,
    pub total: i64,
    pub page: u64,
    pub page_size: u64,
}

async fn fetch_order_items(state: &AppState, order_id: u64) -> Result<Vec<OrderItemRow>, AppError> {
    let items = sqlx::query_as::<_, OrderItemRow>(
        "SELECT id, order_id, order_no, spu_id, sku_id, goods_title, goods_image, \
         spec_info, unit_price, quantity, subtotal FROM order_items WHERE order_id = ? ORDER BY id",
    )
    .bind(order_id)
    .fetch_all(&state.db)
    .await?;
    Ok(items)
}

#[derive(sqlx::FromRow)]
struct AddressRow {
    id: u64,
    receiver_name: String,
    phone: String,
    province: String,
    city: String,
    district: String,
    detail_address: String,
    label: String,
}

async fn fetch_address_snap(state: &AppState, address_id: Option<u64>) -> Option<OrderAddressSnap> {
    let id = address_id?;
    sqlx::query_as::<_, AddressRow>(
        "SELECT id, receiver_name, phone, province, city, district, detail_address, label \
         FROM addresses WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .map(|r| OrderAddressSnap {
        id: r.id.to_string(),
        receiver_name: r.receiver_name,
        phone: r.phone,
        province: r.province,
        city: r.city,
        district: r.district,
        detail_address: r.detail_address,
        label: r.label,
    })
}

const ORDER_SELECT: &str =
    "SELECT id, order_no, external_order_no, user_id, address_id, status, total_amount, paid_amount, \
     discount_amount, remark, created_at, updated_at FROM orders";

async fn fetch_openid(state: &AppState, user_id: u64) -> String {
    sqlx::query_scalar::<_, String>("SELECT openid FROM wechat_users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// 管理后台：订单列表（支持按状态/用户/订单号筛选）
pub async fn list_orders(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Query(q): Query<ListOrdersQuery>,
) -> Result<Json<ApiResponse<PagedOrders>>, AppError> {
    check_admin(&state, &headers).await?;

    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).min(100);
    let offset = (page - 1) * page_size;

    let mut conditions = vec!["1=1"];
    if q.status.is_some() {
        conditions.push("status = ?");
    }
    if q.user_id.is_some() {
        conditions.push("user_id = ?");
    }
    if q.order_no.is_some() {
        conditions.push("order_no LIKE ?");
    }
    let where_clause = conditions.join(" AND ");

    let count_sql = format!("SELECT COUNT(*) FROM orders WHERE {}", where_clause);
    let list_sql = format!(
        "{} WHERE {} ORDER BY id DESC LIMIT ? OFFSET ?",
        ORDER_SELECT, where_clause
    );

    let q_status = q.status;
    let q_user_id = q.user_id;
    let q_order_no = q.order_no.clone();

    let mut count_q = sqlx::query_scalar(&count_sql);
    if let Some(st) = q_status {
        count_q = count_q.bind(st);
    }
    if let Some(uid) = q_user_id {
        count_q = count_q.bind(uid);
    }
    if let Some(ref no) = q_order_no {
        count_q = count_q.bind(format!("%{}%", no));
    }
    let total: i64 = count_q.fetch_one(&state.db).await?;

    let mut list_q = sqlx::query_as::<_, OrderRow>(&list_sql);
    if let Some(st) = q_status {
        list_q = list_q.bind(st);
    }
    if let Some(uid) = q_user_id {
        list_q = list_q.bind(uid);
    }
    if let Some(ref no) = q_order_no {
        list_q = list_q.bind(format!("%{}%", no));
    }
    let rows: Vec<OrderRow> = list_q
        .bind(page_size)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;

    let mut list = Vec::with_capacity(rows.len());
    for row in &rows {
        let items = fetch_order_items(&state, row.id).await?;
        let address = fetch_address_snap(&state, row.address_id).await;
        let openid = fetch_openid(&state, row.user_id).await;
        list.push(build_order_resp(row, items, address, openid));
    }

    Ok(Json(ApiResponse::success(PagedOrders {
        list,
        total,
        page,
        page_size,
    })))
}

/// 管理后台：订单详情
pub async fn get_order(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<OrderResp>>, AppError> {
    check_admin(&state, &headers).await?;

    let order = sqlx::query_as::<_, OrderRow>(&format!("{} WHERE id = ?", ORDER_SELECT))
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("订单不存在".to_string()))?;

    let items = fetch_order_items(&state, order.id).await?;
    let address = fetch_address_snap(&state, order.address_id).await;
    let openid = fetch_openid(&state, order.user_id).await;
    Ok(Json(ApiResponse::success(build_order_resp(
        &order, items, address, openid,
    ))))
}

/// 管理后台：更新订单状态
pub async fn update_order_status(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
    Json(body): Json<UpdateOrderStatusRequest>,
) -> Result<Json<ApiResponse<OrderResp>>, AppError> {
    check_admin(&state, &headers).await?;

    if !(0..=4).contains(&body.status) {
        return Err(AppError::BadRequest("无效的订单状态（0-4）".to_string()));
    }

    let exists: bool = sqlx::query_scalar("SELECT COUNT(*) > 0 FROM orders WHERE id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    if !exists {
        return Err(AppError::NotFound("订单不存在".to_string()));
    }

    sqlx::query("UPDATE orders SET status = ? WHERE id = ?")
        .bind(body.status)
        .bind(id)
        .execute(&state.db)
        .await?;

    let order = sqlx::query_as::<_, OrderRow>(&format!("{} WHERE id = ?", ORDER_SELECT))
        .bind(id)
        .fetch_one(&state.db)
        .await?;

    let items = fetch_order_items(&state, order.id).await?;
    let address = fetch_address_snap(&state, order.address_id).await;
    let openid = fetch_openid(&state, order.user_id).await;
    Ok(Json(ApiResponse::success(build_order_resp(
        &order, items, address, openid,
    ))))
}
