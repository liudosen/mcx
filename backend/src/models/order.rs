use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ─── DB row structs ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct OrderRow {
    pub id: u64,
    pub order_no: String,
    pub external_order_no: Option<String>,
    pub user_id: u64,
    pub address_id: Option<u64>,
    pub status: i8,
    pub total_amount: i64,
    pub paid_amount: i64,
    pub discount_amount: i64,
    pub remark: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct OrderItemRow {
    pub id: u64,
    pub order_id: u64,
    pub order_no: String,
    pub spu_id: u64,
    pub sku_id: u64,
    pub goods_title: String,
    pub goods_image: String,
    pub spec_info: String, // JSON
    pub unit_price: i64,
    pub quantity: i32,
    pub subtotal: i64,
}

/// 地址快照，内嵌到 OrderResp 中（读取下单时的地址记录）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderAddressSnap {
    pub id: String,
    pub receiver_name: String,
    pub phone: String,
    pub province: String,
    pub city: String,
    pub district: String,
    pub detail_address: String,
    pub label: String,
}

// ─── Order status ─────────────────────────────────────────────────────────────

pub fn status_label(status: i8) -> &'static str {
    match status {
        0 => "待付款",
        1 => "待发货",
        2 => "待收货",
        3 => "已完成",
        4 => "已取消",
        _ => "未知",
    }
}

// ─── Mini-app request types ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrderItemReq {
    pub sku_id: Option<String>,
    pub spu_id: Option<String>,
    pub quantity: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrderRequest {
    pub items: Vec<CreateOrderItemReq>,
    pub address_id: String,
    pub remark: Option<String>,
}

// ─── API response types ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderItemResp {
    pub id: String,
    pub spu_id: String,
    pub sku_id: String,
    pub goods_title: String,
    pub goods_image: String,
    pub spec_info: serde_json::Value,
    pub unit_price: i64,
    pub quantity: i32,
    pub subtotal: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderResp {
    pub id: String,
    pub order_no: String,
    pub external_order_no: Option<String>,
    pub openid: String,
    pub status: i8,
    pub status_label: String,
    pub total_amount: i64,
    pub paid_amount: i64,
    pub discount_amount: i64,
    pub remark: Option<String>,
    pub address: Option<OrderAddressSnap>,
    pub items: Vec<OrderItemResp>,
    pub created_at: String,
    pub updated_at: String,
}

/// 支付订单请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayOrderRequest {
    pub payment_password: String,
}

// ─── Admin request types ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpdateOrderStatusRequest {
    pub status: i8,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

pub fn build_order_item_resp(row: &OrderItemRow) -> OrderItemResp {
    let spec_info =
        serde_json::from_str(&row.spec_info).unwrap_or(serde_json::Value::Array(vec![]));
    OrderItemResp {
        id: row.id.to_string(),
        spu_id: row.spu_id.to_string(),
        sku_id: row.sku_id.to_string(),
        goods_title: row.goods_title.clone(),
        goods_image: row.goods_image.clone(),
        spec_info,
        unit_price: row.unit_price,
        quantity: row.quantity,
        subtotal: row.subtotal,
    }
}

pub fn build_order_resp(
    order: &OrderRow,
    items: Vec<OrderItemRow>,
    address: Option<OrderAddressSnap>,
    openid: String,
) -> OrderResp {
    OrderResp {
        id: order.id.to_string(),
        order_no: order.order_no.clone(),
        external_order_no: order.external_order_no.clone(),
        openid,
        status: order.status,
        status_label: status_label(order.status).to_string(),
        total_amount: order.total_amount,
        paid_amount: order.paid_amount,
        discount_amount: order.discount_amount,
        remark: order.remark.clone(),
        address,
        items: items.iter().map(build_order_item_resp).collect(),
        created_at: order.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        updated_at: order.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
    }
}
