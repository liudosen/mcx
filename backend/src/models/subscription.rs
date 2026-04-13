use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ─── 充值虚拟商品固定 ID（对应 migration 20260407000003）────────────────────

pub const RECHARGE_SPU_ID: u64 = 1;
pub const RECHARGE_SKU_ID: u64 = 1;
pub const RECHARGE_GOODS_TITLE: &str = "储值充值";

// ─── subscription_records ────────────────────────────────────────────────────

/// DB 行：订阅记录
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct SubscriptionRecord {
    pub id: u64,
    pub openid: String,
    pub action: i8, // 0=关闭, 1=开启
    pub created_at: NaiveDateTime,
}

/// API 响应：当前订阅状态
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionStatusResp {
    pub action: Option<i8>,
    pub created_at: Option<String>,
}

/// 请求体：开启/关闭订阅
#[derive(Debug, Deserialize)]
pub struct SetSubscriptionRequest {
    pub action: i8, // 0=关闭, 1=开启
}

// ─── balance_accounts ────────────────────────────────────────────────────────

/// DB 行：储值账户
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct BalanceAccount {
    pub id: u64,
    pub openid: String,
    pub balance: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

// ─── balance_transactions ────────────────────────────────────────────────────

/// DB 行：流水记录
#[derive(Debug, Clone, FromRow)]
pub struct BalanceTransaction {
    pub id: u64,
    #[allow(dead_code)]
    pub openid: String,
    pub amount: i64,
    pub balance_after: i64,
    #[sqlx(rename = "type")]
    pub tx_type: i8,
    pub external_order_no: Option<String>,
    pub status: i8, // 0=失败, 1=成功
    pub remark: Option<String>,
    pub created_at: NaiveDateTime,
}

/// API 响应：流水条目
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceTransactionResp {
    pub id: String,
    pub amount: i64,
    pub balance_after: i64,
    pub tx_type: i8,
    pub external_order_no: Option<String>,
    pub status: i8,
    pub remark: Option<String>,
    pub created_at: String,
}

impl From<BalanceTransaction> for BalanceTransactionResp {
    fn from(t: BalanceTransaction) -> Self {
        Self {
            id: t.id.to_string(),
            amount: t.amount,
            balance_after: t.balance_after,
            tx_type: t.tx_type,
            external_order_no: t.external_order_no,
            status: t.status,
            remark: t.remark,
            created_at: t.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

/// API 响应：余额 + 流水
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceResp {
    pub balance: i64,
    pub transactions: Vec<BalanceTransactionResp>,
}
