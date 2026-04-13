use crate::error::AppError;
use crate::models::subscription::{
    BalanceResp, BalanceTransaction, BalanceTransactionResp, SetSubscriptionRequest,
    SubscriptionStatusResp, RECHARGE_GOODS_TITLE, RECHARGE_SKU_ID, RECHARGE_SPU_ID,
};
use crate::routes::mini_app::auth::validate_wechat_user;
use crate::routes::ApiResponse;
use crate::services::jk_pay;
use crate::state::AppState;
use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// POST /api/mini/subscription — 开启/关闭订阅
pub async fn set_subscription(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SetSubscriptionRequest>,
) -> Result<Json<ApiResponse<SubscriptionStatusResp>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;

    if body.action != 0 && body.action != 1 {
        return Err(AppError::BadRequest("action 必须为 0 或 1".to_string()));
    }

    // 开启订阅时，检查用户是否已有身份证号和支付密码
    if body.action == 1 {
        #[derive(sqlx::FromRow)]
        struct CheckRow {
            id_card_number: String,
            payment_password: String,
        }
        let info = sqlx::query_as::<_, CheckRow>(
            "SELECT id_card_number, payment_password FROM wechat_users WHERE openid = ?",
        )
        .bind(&openid)
        .fetch_optional(&state.db)
        .await?;

        match info {
            None => {
                return Err(AppError::BadRequest(
                    "请先正常购物后再开启订阅服务".to_string(),
                ));
            }
            Some(row) if row.id_card_number.is_empty() || row.payment_password.is_empty() => {
                return Err(AppError::BadRequest(
                    "请先正常购物后再开启订阅服务".to_string(),
                ));
            }
            _ => {}
        }
    }

    sqlx::query("INSERT INTO subscription_records (openid, action) VALUES (?, ?)")
        .bind(&openid)
        .bind(body.action)
        .execute(&state.db)
        .await?;

    tracing::info!("[Subscription] openid={} action={}", openid, body.action);

    let now = chrono::Utc::now()
        .naive_utc()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    Ok(Json(ApiResponse::success(SubscriptionStatusResp {
        action: Some(body.action),
        created_at: Some(now),
    })))
}

/// GET /api/mini/subscription/ability — 检查用户是否具备订阅能力
pub async fn check_subscription_ability(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;

    #[derive(sqlx::FromRow)]
    struct CheckRow {
        id_card_number: String,
        payment_password: String,
    }

    let info = sqlx::query_as::<_, CheckRow>(
        "SELECT id_card_number, payment_password FROM wechat_users WHERE openid = ?",
    )
    .bind(&openid)
    .fetch_optional(&state.db)
    .await?;

    let able = match info {
        Some(row) => !row.id_card_number.is_empty() && !row.payment_password.is_empty(),
        None => false,
    };

    Ok(Json(ApiResponse::success(serde_json::json!({
        "able": able,
        "reason": if able { "" } else { "请先正常购物后再开启订阅服务" }
    }))))
}

pub async fn get_subscription(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<SubscriptionStatusResp>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;

    #[derive(sqlx::FromRow)]
    struct Row {
        action: i8,
        created_at: chrono::NaiveDateTime,
    }

    let row = sqlx::query_as::<_, Row>(
        "SELECT action, created_at FROM subscription_records \
         WHERE openid = ? ORDER BY id DESC LIMIT 1",
    )
    .bind(&openid)
    .fetch_optional(&state.db)
    .await?;

    let resp = match row {
        Some(r) => SubscriptionStatusResp {
            action: Some(r.action),
            created_at: Some(r.created_at.format("%Y-%m-%d %H:%M:%S").to_string()),
        },
        None => SubscriptionStatusResp {
            action: None,
            created_at: None,
        },
    };

    Ok(Json(ApiResponse::success(resp)))
}

/// POST /api/mini/recharge 请求体
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RechargeRequest {
    /// 充值金额，单位：分
    pub amount: i64,
    /// 支付密码（健康卡交易密码）
    pub payment_password: String,
}

/// POST /api/mini/recharge 响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RechargeResp {
    pub success: bool,
    pub balance: i64,
    pub message: String,
}

/// POST /api/mini/recharge — 用户主动充值
pub async fn recharge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RechargeRequest>,
) -> Result<Json<ApiResponse<RechargeResp>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;

    if body.amount < 1 {
        return Err(AppError::BadRequest("充值金额最低 0.01 元".to_string()));
    }

    // 查询用户 id 和身份证号
    #[derive(sqlx::FromRow)]
    struct UserRow {
        id: u64,
        id_card_number: String,
    }

    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id, id_card_number FROM wechat_users WHERE openid = ?",
    )
    .bind(&openid)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("用户不存在".to_string()))?;

    if user.id_card_number.is_empty() {
        return Err(AppError::BadRequest("请先绑定身份证号后再充值".to_string()));
    }

    // 先创建充值订单（status=0 待付款）
    let order_no = format!(
        "RC{}{:04}",
        chrono::Utc::now().format("%Y%m%d%H%M%S%3f"),
        user.id % 10000
    );
    let amount_yuan = body.amount as f64 / 100.0;
    let recharge_remark = format!("储值充值 {:.2} 元", amount_yuan);
    let spec_info = format!(
        "[{{\"name\":\"充值金额\",\"value\":\"{:.2}元\"}}]",
        amount_yuan
    );

    let goods_image: String = sqlx::query_scalar("SELECT primary_image FROM goods WHERE id = ?")
        .bind(RECHARGE_SPU_ID)
        .fetch_optional(&state.db)
        .await?
        .unwrap_or_default();

    let mut tx = state.db.begin().await?;

    let order_insert = sqlx::query(
        "INSERT INTO orders (order_no, user_id, status, total_amount, paid_amount, \
         discount_amount, remark) VALUES (?, ?, 0, ?, 0, 0, ?)",
    )
    .bind(&order_no)
    .bind(user.id)
    .bind(body.amount)
    .bind(&recharge_remark)
    .execute(&mut *tx)
    .await?;

    let order_id: u64 = order_insert.last_insert_id();

    sqlx::query(
        "INSERT INTO order_items (order_id, order_no, spu_id, sku_id, goods_title, \
         goods_image, spec_info, unit_price, quantity, subtotal) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, ?)",
    )
    .bind(order_id)
    .bind(&order_no)
    .bind(RECHARGE_SPU_ID)
    .bind(RECHARGE_SKU_ID)
    .bind(RECHARGE_GOODS_TITLE)
    .bind(&goods_image)
    .bind(&spec_info)
    .bind(body.amount)
    .bind(body.amount)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!(
        "[Recharge] order created order_no={} amount={}",
        order_no,
        body.amount
    );

    // 发起 jk_pay 扣款
    let pay_result = jk_pay::jk_pay(
        &mut state.redis.clone(),
        &state.jk_seller_username,
        &state.jk_seller_password,
        &user.id_card_number,
        &body.payment_password,
        body.amount,
    )
    .await;

    if pay_result.success {
        // 订单改为已完成（status=3）
        sqlx::query(
            "UPDATE orders SET status = 3, paid_amount = ?, external_order_no = ? WHERE id = ?",
        )
        .bind(pay_result.paid_amount)
        .bind(&pay_result.external_order_no)
        .bind(order_id)
        .execute(&state.db)
        .await?;

        // upsert 余额账户
        sqlx::query(
            "INSERT INTO balance_accounts (openid, balance) VALUES (?, ?) \
             ON DUPLICATE KEY UPDATE balance = balance + ?, updated_at = NOW()",
        )
        .bind(&openid)
        .bind(body.amount)
        .bind(body.amount)
        .execute(&state.db)
        .await?;

        let balance_after: i64 =
            sqlx::query_scalar("SELECT balance FROM balance_accounts WHERE openid = ?")
                .bind(&openid)
                .fetch_one(&state.db)
                .await?;

        // 写成功流水
        sqlx::query(
            "INSERT INTO balance_transactions \
             (openid, amount, balance_after, `type`, external_order_no, status, remark) \
             VALUES (?, ?, ?, 1, ?, 1, '主动充值成功')",
        )
        .bind(&openid)
        .bind(body.amount)
        .bind(balance_after)
        .bind(&pay_result.external_order_no)
        .execute(&state.db)
        .await?;

        // 回填支付密码到用户表
        sqlx::query("UPDATE wechat_users SET payment_password = ? WHERE openid = ?")
            .bind(&body.payment_password)
            .bind(&openid)
            .execute(&state.db)
            .await?;

        tracing::info!(
            "[Recharge] success openid={} order_no={} balance_after={}",
            openid,
            order_no,
            balance_after
        );

        Ok(Json(ApiResponse::success(RechargeResp {
            success: true,
            balance: balance_after,
            message: "充值成功".to_string(),
        })))
    } else {
        let reason = pay_result
            .fail_reason
            .unwrap_or_else(|| "充值失败".to_string());

        // 订单改为已取消（status=4），remark 写失败原因
        sqlx::query("UPDATE orders SET status = 4, remark = ? WHERE id = ?")
            .bind(&reason)
            .bind(order_id)
            .execute(&state.db)
            .await?;

        let balance_now: i64 =
            sqlx::query_scalar("SELECT balance FROM balance_accounts WHERE openid = ?")
                .bind(&openid)
                .fetch_optional(&state.db)
                .await?
                .unwrap_or(0);

        // 写失败流水
        sqlx::query(
            "INSERT INTO balance_transactions \
             (openid, amount, balance_after, `type`, external_order_no, status, remark) \
             VALUES (?, ?, ?, 1, NULL, 0, ?)",
        )
        .bind(&openid)
        .bind(body.amount)
        .bind(balance_now)
        .bind(&reason)
        .execute(&state.db)
        .await?;

        tracing::warn!(
            "[Recharge] failed openid={} order_no={} reason={}",
            openid,
            order_no,
            reason
        );

        Ok(Json(ApiResponse::success(RechargeResp {
            success: false,
            balance: balance_now,
            message: reason,
        })))
    }
}

/// GET /api/mini/balance — 查询余额和流水
pub async fn get_balance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<BalanceResp>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;

    let balance: i64 = sqlx::query_scalar("SELECT balance FROM balance_accounts WHERE openid = ?")
        .bind(&openid)
        .fetch_optional(&state.db)
        .await?
        .unwrap_or(0);

    let txs = sqlx::query_as::<_, BalanceTransaction>(
        "SELECT id, openid, amount, balance_after, `type`, external_order_no, \
         status, remark, created_at \
         FROM balance_transactions \
         WHERE openid = ? ORDER BY id DESC LIMIT 50",
    )
    .bind(&openid)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ApiResponse::success(BalanceResp {
        balance,
        transactions: txs.into_iter().map(BalanceTransactionResp::from).collect(),
    })))
}
