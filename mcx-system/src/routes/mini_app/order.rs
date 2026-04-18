use crate::error::AppError;
use crate::models::order::{
    build_order_resp, BalancePayRequest, BalancePayResp, CreateOrderRequest, OrderAddressSnap,
    OrderItemRow, OrderResp, OrderRow, PayOrderRequest,
};
use crate::routes::mini_app::auth::validate_wechat_user;
use crate::routes::ApiResponse;
use crate::services::jk_pay;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::Executor;
use std::sync::Arc;

const ORDER_SELECT_SQL: &str = "SELECT id, order_no, external_order_no, user_id, address_id, status, total_amount, paid_amount, discount_amount, remark, created_at, updated_at FROM orders WHERE id = ?";
const ORDER_ITEM_SELECT_SQL: &str = "SELECT id, order_id, order_no, spu_id, sku_id, goods_title, goods_image, spec_info, unit_price, quantity, subtotal FROM order_items WHERE order_id = ? ORDER BY id";
const ADDRESS_SELECT_SQL: &str = "SELECT id, receiver_name, phone, province, city, district, detail_address, label FROM addresses WHERE id = ?";
const WECHAT_USER_ID_SQL: &str = "SELECT id FROM wechat_users WHERE openid = ?";
const WECHAT_ID_CARD_SQL: &str =
    "SELECT COALESCE(id_card_number, '') FROM wechat_users WHERE openid = ?";
const ADDRESS_OWNER_SQL: &str = "SELECT open_id FROM addresses WHERE id = ?";
const ORDER_INSERT_SQL: &str = "INSERT INTO orders (order_no, user_id, address_id, status, total_amount, paid_amount, discount_amount, remark) VALUES (?, ?, ?, 0, ?, 0, 0, ?)";
const ORDER_ITEM_INSERT_SQL: &str = "INSERT INTO order_items (order_id, order_no, spu_id, sku_id, goods_title, goods_image, spec_info, unit_price, quantity, subtotal) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOrdersQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<i8>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedOrders {
    pub list: Vec<OrderResp>,
    pub total: i64,
    pub page: u64,
    pub page_size: u64,
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

#[derive(sqlx::FromRow)]
struct SkuLookup {
    id: u64,
    spu_id: u64,
    sale_price: i64,
    spec_info: String,
    title: String,
    primary_image: String,
}

#[derive(sqlx::FromRow)]
struct UserIdCardRow {
    id_card_number: String,
}

struct ResolvedOrderItem {
    sku_id: u64,
    spu_id: u64,
    goods_title: String,
    goods_image: String,
    spec_info: String,
    unit_price: i64,
    quantity: i32,
    subtotal: i64,
}

async fn fetch_current_balance_on<'e, E>(executor: E, openid: &str) -> Result<i64, AppError>
where
    E: Executor<'e, Database = sqlx::MySql>,
{
    let balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE((SELECT balance FROM balance_accounts WHERE openid = ?), (SELECT balance_after FROM balance_transactions WHERE openid = ? ORDER BY id DESC LIMIT 1), 0)",
    )
    .bind(openid)
    .bind(openid)
    .fetch_one(executor)
    .await?;

    Ok(balance)
}

async fn fetch_order_row(state: &AppState, order_id: u64) -> Result<OrderRow, AppError> {
    sqlx::query_as::<_, OrderRow>(ORDER_SELECT_SQL)
        .bind(order_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("order not found".to_string()))
}

async fn fetch_owned_order(
    state: &AppState,
    order_id: u64,
    user_id: u64,
) -> Result<OrderRow, AppError> {
    let order = fetch_order_row(state, order_id).await?;
    if order.user_id != user_id {
        return Err(AppError::PermissionDenied);
    }
    Ok(order)
}

async fn fetch_order_items(state: &AppState, order_id: u64) -> Result<Vec<OrderItemRow>, AppError> {
    let items = sqlx::query_as::<_, OrderItemRow>(ORDER_ITEM_SELECT_SQL)
        .bind(order_id)
        .fetch_all(&state.db)
        .await?;
    Ok(items)
}

async fn fetch_address_snap(state: &AppState, address_id: Option<u64>) -> Option<OrderAddressSnap> {
    let id = address_id?;
    sqlx::query_as::<_, AddressRow>(ADDRESS_SELECT_SQL)
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .map(|row| OrderAddressSnap {
            id: row.id.to_string(),
            receiver_name: row.receiver_name,
            phone: row.phone,
            province: row.province,
            city: row.city,
            district: row.district,
            detail_address: row.detail_address,
            label: row.label,
        })
}

async fn load_order_resp(
    state: &AppState,
    order: &OrderRow,
    openid: &str,
) -> Result<OrderResp, AppError> {
    let items = fetch_order_items(state, order.id).await?;
    let address = fetch_address_snap(state, order.address_id).await;
    Ok(build_order_resp(order, items, address, openid.to_string()))
}

async fn get_user_id_by_openid(state: &AppState, openid: &str) -> Result<u64, AppError> {
    let user_id: u64 = sqlx::query_scalar(WECHAT_USER_ID_SQL)
        .bind(openid)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("user not found".to_string()))?;
    Ok(user_id)
}

async fn fetch_user_id_card_number(state: &AppState, openid: &str) -> Result<String, AppError> {
    let row = sqlx::query_as::<_, UserIdCardRow>(WECHAT_ID_CARD_SQL)
        .bind(openid)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("user not found".to_string()))?;
    Ok(row.id_card_number)
}

async fn ensure_address_owned_by_user(
    state: &AppState,
    openid: &str,
    address_id: u64,
) -> Result<(), AppError> {
    let owner: Option<String> = sqlx::query_scalar(ADDRESS_OWNER_SQL)
        .bind(address_id)
        .fetch_optional(&state.db)
        .await?;

    match owner {
        None => Err(AppError::NotFound("address not found".to_string())),
        Some(owner_openid) if owner_openid != openid => Err(AppError::PermissionDenied),
        _ => Ok(()),
    }
}

async fn resolve_order_item(
    state: &AppState,
    item_req: &crate::models::order::CreateOrderItemReq,
) -> Result<ResolvedOrderItem, AppError> {
    if item_req.quantity <= 0 {
        return Err(AppError::BadRequest("鍟嗗搧鏁伴噺蹇呴』澶т簬0".to_string()));
    }

    let sku_row: SkuLookup = if let Some(ref sku_id_str) = item_req.sku_id {
        let sku_id: u64 = sku_id_str
            .parse()
            .map_err(|_| AppError::BadRequest("skuId 鏍煎紡閿欒".to_string()))?;
        sqlx::query_as::<_, SkuLookup>(
            "SELECT gs.id, gs.spu_id, gs.sale_price, gs.spec_info, g.title, g.primary_image FROM goods_skus gs JOIN goods g ON g.id = gs.spu_id WHERE gs.id = ? AND g.status = 1",
        )
        .bind(sku_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound(format!(
            "SKU {} not found or product offline",
            sku_id
        )))?
    } else if let Some(ref spu_id_str) = item_req.spu_id {
        let spu_id: u64 = spu_id_str
            .parse()
            .map_err(|_| AppError::BadRequest("spuId 鏍煎紡閿欒".to_string()))?;
        sqlx::query_as::<_, SkuLookup>(
            "SELECT gs.id, gs.spu_id, gs.sale_price, gs.spec_info, g.title, g.primary_image FROM goods_skus gs JOIN goods g ON g.id = gs.spu_id WHERE gs.spu_id = ? AND g.status = 1 ORDER BY gs.id LIMIT 1",
        )
        .bind(spu_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound(format!(
            "SPU {} has no available SKU or product is offline",
            spu_id
        )))?
    } else {
        return Err(AppError::BadRequest(
            "each item must provide skuId or spuId".to_string(),
        ));
    };

    let subtotal = sku_row.sale_price * item_req.quantity as i64;
    Ok(ResolvedOrderItem {
        sku_id: sku_row.id,
        spu_id: sku_row.spu_id,
        goods_title: sku_row.title,
        goods_image: sku_row.primary_image,
        spec_info: sku_row.spec_info,
        unit_price: sku_row.sale_price,
        quantity: item_req.quantity,
        subtotal,
    })
}

fn build_order_no(user_id: u64) -> String {
    format!(
        "{}{:04}",
        Utc::now().format("%Y%m%d%H%M%S%3f"),
        user_id % 10000
    )
}

fn build_balance_trade_no(order_id: u64) -> String {
    format!("BAL{}{}", Utc::now().format("%Y%m%d%H%M%S%3f"), order_id)
}

async fn insert_order_with_items(
    state: &AppState,
    order_no: &str,
    user_id: u64,
    address_id: u64,
    remark: Option<&str>,
    items: &[ResolvedOrderItem],
    total_amount: i64,
) -> Result<u64, AppError> {
    let mut tx = state.db.begin().await?;

    let order_insert = sqlx::query(ORDER_INSERT_SQL)
        .bind(order_no)
        .bind(user_id)
        .bind(address_id)
        .bind(total_amount)
        .bind(remark)
        .execute(&mut *tx)
        .await?;

    let order_id = order_insert.last_insert_id();

    for item in items {
        sqlx::query(ORDER_ITEM_INSERT_SQL)
            .bind(order_id)
            .bind(order_no)
            .bind(item.spu_id)
            .bind(item.sku_id)
            .bind(&item.goods_title)
            .bind(&item.goods_image)
            .bind(&item.spec_info)
            .bind(item.unit_price)
            .bind(item.quantity)
            .bind(item.subtotal)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(order_id)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayOrderResp {
    pub success: bool,
    pub paid_amount: i64,
    pub order_status: Option<i64>,
    pub message: String,
}

/// 灏忕▼搴忥細浣欓鏀粯璁㈠崟
pub async fn pay_order_with_balance(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
    Json(_body): Json<BalancePayRequest>,
) -> Result<Json<ApiResponse<BalancePayResp>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;
    let user_id = get_user_id_by_openid(&state, &openid).await?;
    let lock_key = format!("balance_pay:{}", openid);
    let mut tx = state.db.begin().await?;

    let lock_acquired: Option<i32> = sqlx::query_scalar("SELECT GET_LOCK(?, 5)")
        .bind(&lock_key)
        .fetch_one(&mut *tx)
        .await?;
    if lock_acquired != Some(1) {
        tx.rollback().await?;
        return Err(AppError::BadRequest(
            "payment lock busy, please retry".to_string(),
        ));
    }

    let result = async {
        let order = fetch_owned_order(&state, id, user_id).await?;
        let current_balance = fetch_current_balance_on(&mut *tx, &openid).await?;

        if order.status != 0 {
            return Ok(Json(ApiResponse::success(BalancePayResp {
                success: false,
                paid_amount: 0,
                balance_after: current_balance,
                order_status: Some(order.status as i64),
                message: "only pending orders can be paid".to_string(),
            })));
        }

        if current_balance < order.total_amount {
            return Ok(Json(ApiResponse::success(BalancePayResp {
                success: false,
                paid_amount: 0,
                balance_after: current_balance,
                order_status: Some(order.status as i64),
                message: "insufficient balance".to_string(),
            })));
        }

        let balance_after = current_balance - order.total_amount;
        let balance_trade_no = build_balance_trade_no(order.id);

        let updated = sqlx::query(
            "UPDATE orders SET status = 1, paid_amount = ?, external_order_no = ? WHERE id = ? AND status = 0",
        )
        .bind(order.total_amount)
        .bind(&balance_trade_no)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() != 1 {
            return Err(AppError::BadRequest(
                "order status changed, please retry".to_string(),
            ));
        }

        sqlx::query(
            "INSERT INTO balance_accounts (openid, balance) VALUES (?, ?) ON DUPLICATE KEY UPDATE balance = VALUES(balance), updated_at = NOW()",
        )
        .bind(&openid)
        .bind(balance_after)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO balance_transactions (openid, amount, balance_after, `type`, external_order_no, status, remark) VALUES (?, ?, ?, 2, ?, 1, 'order balance payment')",
        )
        .bind(&openid)
        .bind(order.total_amount)
        .bind(balance_after)
        .bind(&balance_trade_no)
        .execute(&mut *tx)
        .await?;

        Ok(Json(ApiResponse::success(BalancePayResp {
            success: true,
            paid_amount: order.total_amount,
            balance_after,
            order_status: Some(1),
            message: "payment successful".to_string(),
        })))
    }
    .await;

    let _ = sqlx::query_scalar::<_, Option<i32>>("SELECT RELEASE_LOCK(?)")
        .bind(&lock_key)
        .fetch_one(&mut *tx)
        .await;

    match result {
        Ok(resp) => {
            tx.commit().await?;
            Ok(resp)
        }
        Err(err) => {
            tx.rollback().await?;
            Err(err)
        }
    }
}

/// 灏忕▼搴忥細纭鏀惰揣
pub async fn confirm_my_order_received(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<OrderResp>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;
    let user_id = get_user_id_by_openid(&state, &openid).await?;
    let order = fetch_owned_order(&state, id, user_id).await?;

    if order.status != 2 {
        return Err(AppError::BadRequest(
            "鍙湁寰呮敹璐х殑璁㈠崟鎵嶈兘纭鏀惰揣".to_string(),
        ));
    }

    sqlx::query("UPDATE orders SET status = 3 WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    let updated = fetch_order_row(&state, id).await?;
    let resp = load_order_resp(&state, &updated, &openid).await?;
    Ok(Json(ApiResponse::success(resp)))
}

/// 灏忕▼搴忥細鎻愪氦璁㈠崟
pub async fn create_order(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<CreateOrderRequest>,
) -> Result<Json<ApiResponse<OrderResp>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;
    let user_id = get_user_id_by_openid(&state, &openid).await?;

    if body.items.is_empty() {
        return Err(AppError::BadRequest("璁㈠崟鍟嗗搧涓嶈兘涓虹┖".to_string()));
    }

    let address_id: u64 = body
        .address_id
        .parse()
        .map_err(|_| AppError::BadRequest("addressId 鏍煎紡閿欒".to_string()))?;

    ensure_address_owned_by_user(&state, &openid, address_id).await?;

    let order_no = build_order_no(user_id);

    let mut total_amount: i64 = 0;
    let mut resolved_items = Vec::with_capacity(body.items.len());
    for item_req in &body.items {
        let item = resolve_order_item(&state, item_req).await?;
        total_amount += item.subtotal;
        resolved_items.push(item);
    }

    let order_id = insert_order_with_items(
        &state,
        &order_no,
        user_id,
        address_id,
        body.remark.as_deref(),
        &resolved_items,
        total_amount,
    )
    .await?;

    let order = fetch_order_row(&state, order_id).await?;
    let resp = load_order_resp(&state, &order, &openid).await?;
    Ok(Json(ApiResponse::success(resp)))
}

/// 灏忕▼搴忥細鎴戠殑璁㈠崟鍒楄〃
pub async fn list_my_orders(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Query(q): Query<ListOrdersQuery>,
) -> Result<Json<ApiResponse<PagedOrders>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;
    let user_id = get_user_id_by_openid(&state, &openid).await?;

    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(10).min(50);
    let offset = (page - 1) * page_size;

    let (count_sql, list_sql) = if q.status.is_some() {
        (
            "SELECT COUNT(*) FROM orders WHERE user_id = ? AND status = ?".to_string(),
            "SELECT id, order_no, external_order_no, user_id, address_id, status, total_amount, paid_amount, discount_amount, remark, created_at, updated_at FROM orders WHERE user_id = ? AND status = ? ORDER BY id DESC LIMIT ? OFFSET ?".to_string(),
        )
    } else {
        (
            "SELECT COUNT(*) FROM orders WHERE user_id = ?".to_string(),
            "SELECT id, order_no, external_order_no, user_id, address_id, status, total_amount, paid_amount, discount_amount, remark, created_at, updated_at FROM orders WHERE user_id = ? ORDER BY id DESC LIMIT ? OFFSET ?".to_string(),
        )
    };

    let total: i64 = if let Some(st) = q.status {
        sqlx::query_scalar(&count_sql)
            .bind(user_id)
            .bind(st)
            .fetch_one(&state.db)
            .await?
    } else {
        sqlx::query_scalar(&count_sql)
            .bind(user_id)
            .fetch_one(&state.db)
            .await?
    };

    let rows: Vec<OrderRow> = if let Some(st) = q.status {
        sqlx::query_as::<_, OrderRow>(&list_sql)
            .bind(user_id)
            .bind(st)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.db)
            .await?
    } else {
        sqlx::query_as::<_, OrderRow>(&list_sql)
            .bind(user_id)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.db)
            .await?
    };

    let mut list = Vec::with_capacity(rows.len());
    for row in &rows {
        list.push(load_order_resp(&state, row, &openid).await?);
    }

    Ok(Json(ApiResponse::success(PagedOrders {
        list,
        total,
        page,
        page_size,
    })))
}

/// 灏忕▼搴忥細璁㈠崟璇︽儏
pub async fn get_my_order(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<OrderResp>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;
    let user_id = get_user_id_by_openid(&state, &openid).await?;
    let order = fetch_owned_order(&state, id, user_id).await?;
    let resp = load_order_resp(&state, &order, &openid).await?;
    Ok(Json(ApiResponse::success(resp)))
}

/// 灏忕▼搴忥細鍙栨秷璁㈠崟锛堜粎闄愬緟浠樻鐘舵€侊級
pub async fn cancel_my_order(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<OrderResp>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;
    let user_id = get_user_id_by_openid(&state, &openid).await?;
    let order = fetch_owned_order(&state, id, user_id).await?;

    if order.status != 0 {
        return Err(AppError::BadRequest(
            "鍙湁寰呬粯娆剧殑璁㈠崟鎵嶈兘鍙栨秷".to_string(),
        ));
    }

    sqlx::query("UPDATE orders SET status = 4 WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    let updated = fetch_order_row(&state, id).await?;
    let resp = load_order_resp(&state, &updated, &openid).await?;
    Ok(Json(ApiResponse::success(resp)))
}

/// 小程序：支付订单（健康卡支付）
pub async fn pay_order(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
    Json(body): Json<PayOrderRequest>,
) -> Result<Json<ApiResponse<PayOrderResp>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;
    let user_id = get_user_id_by_openid(&state, &openid).await?;
    let order = fetch_owned_order(&state, id, user_id).await?;

    if order.status != 0 {
        return Err(AppError::BadRequest(
            "鍙湁寰呬粯娆剧殑璁㈠崟鎵嶈兘鏀粯".to_string(),
        ));
    }

    let id_card_number = fetch_user_id_card_number(&state, &openid).await?;
    if id_card_number.is_empty() {
        let fail_msg = "鏀粯澶辫触锛氱敤鎴锋湭缁戝畾韬唤璇佸彿";
        sqlx::query("UPDATE orders SET remark = ? WHERE id = ?")
            .bind(fail_msg)
            .bind(id)
            .execute(&state.db)
            .await?;
        return Ok(Json(ApiResponse::success(PayOrderResp {
            success: false,
            paid_amount: 0,
            order_status: None,
            message: fail_msg.to_string(),
        })));
    }

    let result = jk_pay::jk_pay(
        &mut state.redis.clone(),
        &state.jk_seller_username,
        &state.jk_seller_password,
        &id_card_number,
        &body.payment_password,
        order.total_amount,
    )
    .await;

    if result.success {
        sqlx::query(
            "UPDATE orders SET status = 1, paid_amount = ?, external_order_no = ? WHERE id = ?",
        )
        .bind(result.paid_amount)
        .bind(&result.external_order_no)
        .bind(id)
        .execute(&state.db)
        .await?;

        sqlx::query("UPDATE wechat_users SET payment_password = ? WHERE openid = ?")
            .bind(&body.payment_password)
            .bind(&openid)
            .execute(&state.db)
            .await?;

        Ok(Json(ApiResponse::success(PayOrderResp {
            success: true,
            paid_amount: result.paid_amount,
            order_status: result.order_status,
            message: "鏀粯鎴愬姛".to_string(),
        })))
    } else {
        let fail_msg = result
            .fail_reason
            .unwrap_or_else(|| "鏀粯澶辫触".to_string());
        sqlx::query("UPDATE orders SET remark = ? WHERE id = ?")
            .bind(&fail_msg)
            .bind(id)
            .execute(&state.db)
            .await?;

        Ok(Json(ApiResponse::success(PayOrderResp {
            success: false,
            paid_amount: 0,
            order_status: result.order_status,
            message: fail_msg,
        })))
    }
}
