use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

fn deserialize_optional_i8<'de, D>(deserializer: D) -> Result<Option<i8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<String>::deserialize(deserializer)?;
    match raw.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(value) => value.parse::<i8>().map(Some).map_err(serde::de::Error::custom),
    }
}

#[allow(dead_code)]
pub fn subscription_action_label(action: i8) -> &'static str {
    match action {
        1 => "寮€鍚?",
        0 => "鍏抽棴",
        _ => "鏈煡",
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SubscriptionRecordQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub openid: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_i8")]
    pub action: Option<i8>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionRecordListItem {
    pub id: u64,
    pub openid: String,
    pub real_name: String,
    pub phone: String,
    pub action: i8,
    pub action_label: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionRecordListResponse {
    pub list: Vec<SubscriptionRecordListItem>,
    pub total: i64,
    pub page: u64,
    pub page_size: u64,
}

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

#[cfg(test)]
mod tests {
    use super::SubscriptionRecordQuery;

    #[test]
    fn empty_action_is_treated_as_none() {
        let query: SubscriptionRecordQuery = serde_json::from_str(
            r#"{"page":1,"page_size":20,"action":""}"#,
        )
        .expect("query should deserialize");

        assert_eq!(query.action, None);
    }

    #[test]
    fn numeric_action_is_parsed() {
        let query: SubscriptionRecordQuery = serde_json::from_str(
            r#"{"page":1,"page_size":20,"action":"1"}"#,
        )
        .expect("query should deserialize");

        assert_eq!(query.action, Some(1));
    }
}
