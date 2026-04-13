use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 微信小程序用户
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WechatUser {
    pub id: u64,
    pub openid: String,
    pub real_name: String,
    pub avatar_url: String,
    pub phone: String,
    pub country: String,
    pub province: String,
    pub city: String,
    pub gender: i8,
    pub id_card_number: String,
    pub payment_password: String,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 地址（关联用户信息）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AddressWithUser {
    // 地址信息（LEFT JOIN，可能为 NULL）
    pub id: Option<u64>,
    pub open_id: Option<String>,
    pub receiver_name: Option<String>,
    pub phone: Option<String>,
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub detail_address: Option<String>,
    pub label: Option<String>,
    pub is_default: Option<bool>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    // 关联的用户信息
    pub real_name: String,
    pub avatar_url: String,
    pub user_phone: String,
    pub gender: i8,
    pub id_card_number: String,
}

/// 创建用户请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateWechatUserRequest {
    pub openid: String,
    pub real_name: Option<String>,
    pub avatar_url: Option<String>,
    pub phone: Option<String>,
    pub country: Option<String>,
    pub province: Option<String>,
    pub city: Option<String>,
    pub gender: Option<i8>,
    pub id_card_number: Option<String>,
}

/// 更新用户请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWechatUserRequest {
    pub real_name: Option<String>,
    pub avatar_url: Option<String>,
    pub phone: Option<String>,
    pub country: Option<String>,
    pub province: Option<String>,
    pub city: Option<String>,
    pub gender: Option<i8>,
    pub id_card_number: Option<String>,
}

/// 通过openid更新用户请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWechatUserByOpenidRequest {
    pub real_name: Option<String>,
    pub phone: Option<String>,
    pub id_card_number: Option<String>,
}

/// 用户查询参数
#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct WechatUserQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub openid: Option<String>,
    pub phone: Option<String>,
    pub gender: Option<i8>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// 分页响应
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T> {
    pub list: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: i64,
}

impl<T> PaginatedResponse<T> {
    pub fn new(list: Vec<T>, total: i64, page: u32, page_size: u32) -> Self {
        let total_pages = if page_size > 0 {
            (total as f64 / page_size as f64).ceil() as i64
        } else {
            0
        };
        Self {
            list,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}

/// 支付密码响应
#[derive(Debug, Clone, Serialize)]
pub struct PaymentPasswordResponse {
    pub payment_password: String,
}
