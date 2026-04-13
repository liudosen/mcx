use crate::error::AppError;
use crate::models::order::{
    build_order_resp, CreateOrderRequest, OrderAddressSnap, OrderItemRow, OrderResp, OrderRow,
    PayOrderRequest,
};
use crate::routes::mini_app::auth::validate_wechat_user;
use crate::routes::ApiResponse;
use crate::services::jk_pay;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

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

async fn get_user_id_by_openid(state: &AppState, openid: &str) -> Result<u64, AppError> {
    let user_id: u64 = sqlx::query_scalar("SELECT id FROM wechat_users WHERE openid = ?")
        .bind(openid)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("用户不存在".to_string()))?;
    Ok(user_id)
}

/// 小程序：提交订单
pub async fn create_order(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<CreateOrderRequest>,
) -> Result<Json<ApiResponse<OrderResp>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;
    let user_id = get_user_id_by_openid(&state, &openid).await?;

    if body.items.is_empty() {
        return Err(AppError::BadRequest("订单商品不能为空".to_string()));
    }

    // 校验收货地址归属当前用户
    let address_id: u64 = body
        .address_id
        .parse()
        .map_err(|_| AppError::BadRequest("addressId 格式错误".to_string()))?;

    let addr_open_id: Option<String> =
        sqlx::query_scalar("SELECT open_id FROM addresses WHERE id = ?")
            .bind(address_id)
            .fetch_optional(&state.db)
            .await?;

    match addr_open_id {
        None => return Err(AppError::NotFound("收货地址不存在".to_string())),
        Some(oid) if oid != openid => return Err(AppError::PermissionDenied),
        _ => {}
    }

    // 生成订单号：时间戳 + user_id 尾4位
    let order_no = format!(
        "{}{:04}",
        chrono::Utc::now().format("%Y%m%d%H%M%S%3f"),
        user_id % 10000
    );

    // 查询每个 SKU 的价格和商品信息，计算金额
    let mut total_amount: i64 = 0;
    struct ItemData {
        sku_id: u64,
        spu_id: u64,
        goods_title: String,
        goods_image: String,
        spec_info: String,
        unit_price: i64,
        quantity: i32,
        subtotal: i64,
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

    let mut item_data_list: Vec<ItemData> = Vec::with_capacity(body.items.len());

    for item_req in &body.items {
        if item_req.quantity <= 0 {
            return Err(AppError::BadRequest("商品数量必须大于0".to_string()));
        }
        // 支持按 skuId 或 spuId 下单；spuId 下单时自动取第一个 SKU
        let sku_row: SkuLookup = if let Some(ref sku_id_str) = item_req.sku_id {
            let sku_id: u64 = sku_id_str
                .parse()
                .map_err(|_| AppError::BadRequest("skuId 格式错误".to_string()))?;
            sqlx::query_as::<_, SkuLookup>(
                "SELECT gs.id, gs.spu_id, gs.sale_price, gs.spec_info, \
                 g.title, g.primary_image \
                 FROM goods_skus gs JOIN goods g ON g.id = gs.spu_id \
                 WHERE gs.id = ? AND g.status = 1",
            )
            .bind(sku_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound(format!(
                "SKU {} 不存在或商品已下架",
                sku_id
            )))?
        } else if let Some(ref spu_id_str) = item_req.spu_id {
            let spu_id: u64 = spu_id_str
                .parse()
                .map_err(|_| AppError::BadRequest("spuId 格式错误".to_string()))?;
            sqlx::query_as::<_, SkuLookup>(
                "SELECT gs.id, gs.spu_id, gs.sale_price, gs.spec_info, \
                 g.title, g.primary_image \
                 FROM goods_skus gs JOIN goods g ON g.id = gs.spu_id \
                 WHERE gs.spu_id = ? AND g.status = 1 ORDER BY gs.id LIMIT 1",
            )
            .bind(spu_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound(format!(
                "SPU {} 下无可用SKU或商品已下架",
                spu_id
            )))?
        } else {
            return Err(AppError::BadRequest(
                "每个商品必须提供 skuId 或 spuId".to_string(),
            ));
        };

        let sku_id = sku_row.id;

        let subtotal = sku_row.sale_price * item_req.quantity as i64;
        total_amount += subtotal;

        item_data_list.push(ItemData {
            sku_id,
            spu_id: sku_row.spu_id,
            goods_title: sku_row.title,
            goods_image: sku_row.primary_image,
            spec_info: sku_row.spec_info,
            unit_price: sku_row.sale_price,
            quantity: item_req.quantity,
            subtotal,
        });
    }

    let mut tx = state.db.begin().await?;

    let order_insert = sqlx::query(
        "INSERT INTO orders (order_no, user_id, address_id, status, total_amount, paid_amount, \
         discount_amount, remark) VALUES (?, ?, ?, 0, ?, 0, 0, ?)",
    )
    .bind(&order_no)
    .bind(user_id)
    .bind(address_id)
    .bind(total_amount)
    .bind(&body.remark)
    .execute(&mut *tx)
    .await?;

    let order_id: u64 = order_insert.last_insert_id();

    for item in &item_data_list {
        sqlx::query(
            "INSERT INTO order_items (order_id, order_no, spu_id, sku_id, goods_title, \
             goods_image, spec_info, unit_price, quantity, subtotal) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(order_id)
        .bind(&order_no)
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

    let order = sqlx::query_as::<_, OrderRow>(
        "SELECT id, order_no, external_order_no, user_id, address_id, status, total_amount, paid_amount, \
         discount_amount, remark, created_at, updated_at FROM orders WHERE id = ?",
    )
    .bind(order_id)
    .fetch_one(&state.db)
    .await?;

    let items = fetch_order_items(&state, order_id).await?;
    let address = fetch_address_snap(&state, order.address_id).await;
    Ok(Json(ApiResponse::success(build_order_resp(
        &order, items, address, openid,
    ))))
}

/// 小程序：我的订单列表
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
            "SELECT id, order_no, external_order_no, user_id, address_id, status, total_amount, paid_amount, \
             discount_amount, remark, created_at, updated_at \
             FROM orders WHERE user_id = ? AND status = ? ORDER BY id DESC LIMIT ? OFFSET ?"
                .to_string(),
        )
    } else {
        (
            "SELECT COUNT(*) FROM orders WHERE user_id = ?".to_string(),
            "SELECT id, order_no, external_order_no, user_id, address_id, status, total_amount, paid_amount, \
             discount_amount, remark, created_at, updated_at \
             FROM orders WHERE user_id = ? ORDER BY id DESC LIMIT ? OFFSET ?"
                .to_string(),
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
        let items = fetch_order_items(&state, row.id).await?;
        let address = fetch_address_snap(&state, row.address_id).await;
        list.push(build_order_resp(row, items, address, openid.clone()));
    }

    Ok(Json(ApiResponse::success(PagedOrders {
        list,
        total,
        page,
        page_size,
    })))
}

/// 小程序：订单详情
pub async fn get_my_order(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<OrderResp>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;
    let user_id = get_user_id_by_openid(&state, &openid).await?;

    let order = sqlx::query_as::<_, OrderRow>(
        "SELECT id, order_no, external_order_no, user_id, address_id, status, total_amount, paid_amount, \
         discount_amount, remark, created_at, updated_at FROM orders WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("订单不存在".to_string()))?;

    if order.user_id != user_id {
        return Err(AppError::PermissionDenied);
    }

    let items = fetch_order_items(&state, order.id).await?;
    let address = fetch_address_snap(&state, order.address_id).await;
    Ok(Json(ApiResponse::success(build_order_resp(
        &order, items, address, openid,
    ))))
}

/// 小程序：取消订单（仅限待付款状态）
pub async fn cancel_my_order(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<ApiResponse<OrderResp>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;
    let user_id = get_user_id_by_openid(&state, &openid).await?;

    let order = sqlx::query_as::<_, OrderRow>(
        "SELECT id, order_no, external_order_no, user_id, address_id, status, total_amount, paid_amount, \
         discount_amount, remark, created_at, updated_at FROM orders WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("订单不存在".to_string()))?;

    if order.user_id != user_id {
        return Err(AppError::PermissionDenied);
    }

    if order.status != 0 {
        return Err(AppError::BadRequest("只有待付款的订单才能取消".to_string()));
    }

    sqlx::query("UPDATE orders SET status = 4 WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    let updated = sqlx::query_as::<_, OrderRow>(
        "SELECT id, order_no, external_order_no, user_id, address_id, status, total_amount, paid_amount, \
         discount_amount, remark, created_at, updated_at FROM orders WHERE id = ?",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    let items = fetch_order_items(&state, updated.id).await?;
    let address = fetch_address_snap(&state, updated.address_id).await;
    Ok(Json(ApiResponse::success(build_order_resp(
        &updated, items, address, openid,
    ))))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayOrderResp {
    pub success: bool,
    pub paid_amount: i64,
    pub order_status: Option<i64>,
    pub message: String,
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

    // Fetch order
    let order = sqlx::query_as::<_, OrderRow>(
        "SELECT id, order_no, external_order_no, user_id, address_id, status, total_amount, paid_amount, \
         discount_amount, remark, created_at, updated_at FROM orders WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("订单不存在".to_string()))?;

    if order.user_id != user_id {
        return Err(AppError::PermissionDenied);
    }

    if order.status != 0 {
        return Err(AppError::BadRequest("只有待付款的订单才能支付".to_string()));
    }

    // Fetch user id_card_number
    #[derive(sqlx::FromRow)]
    struct UserIdCard {
        id_card_number: String,
    }
    let user_info =
        sqlx::query_as::<_, UserIdCard>("SELECT id_card_number FROM wechat_users WHERE openid = ?")
            .bind(&openid)
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound("用户不存在".to_string()))?;

    if user_info.id_card_number.is_empty() {
        // Write failure reason to order remark
        let fail_msg = "支付失败：用户未绑定身份证号";
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

    let card_no = &user_info.id_card_number;
    let card_password = &body.payment_password;

    let result = jk_pay::jk_pay(
        &mut state.redis.clone(),
        &state.jk_seller_username,
        &state.jk_seller_password,
        card_no,
        card_password,
        order.total_amount,
    )
    .await;

    if result.success {
        // Update order: status=1 (待发货), paid_amount, external_order_no
        sqlx::query(
            "UPDATE orders SET status = 1, paid_amount = ?, external_order_no = ? WHERE id = ?",
        )
        .bind(result.paid_amount)
        .bind(&result.external_order_no)
        .bind(id)
        .execute(&state.db)
        .await?;

        // 回填支付密码到用户表
        sqlx::query("UPDATE wechat_users SET payment_password = ? WHERE openid = ?")
            .bind(card_password)
            .bind(&openid)
            .execute(&state.db)
            .await?;

        Ok(Json(ApiResponse::success(PayOrderResp {
            success: true,
            paid_amount: result.paid_amount,
            order_status: result.order_status,
            message: "支付成功".to_string(),
        })))
    } else {
        let fail_msg = result.fail_reason.unwrap_or_else(|| "支付失败".to_string());
        // Write failure reason to order remark
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
