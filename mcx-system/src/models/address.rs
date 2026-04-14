use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 收货地址
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Address {
    pub id: u64,
    pub open_id: String,
    pub receiver_name: String,
    pub phone: String,
    pub province: String,
    pub city: String,
    pub district: String,
    pub detail_address: String,
    pub label: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建地址请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAddressRequest {
    pub receiver_name: String,
    pub phone: String,
    pub province: String,
    pub city: String,
    pub district: String,
    pub detail_address: String,
    pub label: Option<String>,
    pub is_default: Option<bool>,
}

/// 更新地址请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAddressRequest {
    pub receiver_name: Option<String>,
    pub phone: Option<String>,
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub detail_address: Option<String>,
    pub label: Option<String>,
    pub is_default: Option<bool>,
}

/// 设置默认地址请求
#[derive(Debug, Clone, Deserialize)]
pub struct SetDefaultRequest {
    pub is_default: bool,
}
