use crate::error::AppError;
use crate::models::subscription::{
    BalanceResp, BalanceTransactionResp, RECHARGE_GOODS_TITLE, RECHARGE_SKU_ID, RECHARGE_SPU_ID,
};
use crate::routes::admin::auth::check_admin;
use crate::routes::ApiResponse;
use crate::services::account;
use crate::services::jk_pay;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Serialize;
use std::sync::Arc;

const AUTO_RECHARGE_AMOUNT: i64 = 200_000; // 200000 分 = 2000 元

/// POST /api/admin/subscription/auto-recharge 的响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoRechargeResp {
    pub total: usize,
    pub success_count: usize,
    pub fail_count: usize,
    pub skipped_count: usize,
    pub results: Vec<AutoRechargeUserResult>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoRechargeUserResult {
    pub openid: String,
    pub success: bool,
    pub fail_reason: Option<String>,
    pub external_order_no: Option<String>,
}

/// POST /api/admin/subscription/auto-recharge
/// 由外部 cron 在每年 6 月 1 日调用，对所有开启订阅的用户自动扣款 2000 元
pub async fn auto_recharge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<AutoRechargeResp>>, AppError> {
    check_admin(&state, &headers).await?;

    tracing::info!("[AutoRecharge] starting auto-recharge job");

    #[derive(sqlx::FromRow)]
    struct UserRow {
        openid: String,
        id_card_number: String,
        payment_password: String,
    }

    let users = sqlx::query_as::<_, UserRow>(
        "SELECT openid, id_card_number, payment_password \
         FROM wechat_users \
         WHERE id_card_number != '' AND payment_password != ''",
    )
    .fetch_all(&state.db)
    .await?;

    tracing::info!("[AutoRecharge] found {} eligible users", users.len());

    let total = users.len();
    let mut results: Vec<AutoRechargeUserResult> = Vec::with_capacity(total);
    let mut success_count = 0usize;
    let mut fail_count = 0usize;
    let mut skipped_count = 0usize;

    for user in users {
        // 查询该用户最新订阅记录
        let latest_action: Option<i8> = sqlx::query_scalar(
            "SELECT action FROM subscription_records WHERE openid = ? ORDER BY id DESC LIMIT 1",
        )
        .bind(&user.openid)
        .fetch_optional(&state.db)
        .await?;

        // 无订阅记录或最新 action=0 则跳过
        if latest_action != Some(1) {
            tracing::info!(
                "[AutoRecharge] openid={} skipped (action={:?})",
                user.openid,
                latest_action
            );
            skipped_count += 1;
            continue;
        }

        // 调用 jk_pay 扣款
        let pay_result = jk_pay::jk_pay(
            &mut state.redis.clone(),
            &state.jk_seller_username,
            &state.jk_seller_password,
            &user.id_card_number,
            &user.payment_password,
            AUTO_RECHARGE_AMOUNT,
        )
        .await;

        // 先创建充值订单（status=0 待付款）
        let user_id = account::user_id_by_openid(&state, &user.openid).await?;
        let order_no = format!(
            "RC{}{:04}",
            chrono::Utc::now().format("%Y%m%d%H%M%S%3f"),
            user_id % 10000
        );
        let amount_yuan = AUTO_RECHARGE_AMOUNT as f64 / 100.0;
        let recharge_remark = format!("储值充值 {:.2} 元", amount_yuan);
        let spec_info = format!(
            "[{{\"name\":\"充值金额\",\"value\":\"{:.2}元\"}}]",
            amount_yuan
        );

        let goods_image: String =
            sqlx::query_scalar("SELECT primary_image FROM goods WHERE id = ?")
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
        .bind(user_id)
        .bind(AUTO_RECHARGE_AMOUNT)
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
        .bind(AUTO_RECHARGE_AMOUNT)
        .bind(AUTO_RECHARGE_AMOUNT)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

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
            .bind(&user.openid)
            .bind(AUTO_RECHARGE_AMOUNT)
            .bind(AUTO_RECHARGE_AMOUNT)
            .execute(&state.db)
            .await?;

            // 读取更新后的余额
            let balance_after = account::current_balance(&state, &user.openid).await?;

            // 写成功流水
            sqlx::query(
                "INSERT INTO balance_transactions \
                 (openid, amount, balance_after, `type`, external_order_no, status, remark) \
                 VALUES (?, ?, ?, 1, ?, 1, '自动充值成功')",
            )
            .bind(&user.openid)
            .bind(AUTO_RECHARGE_AMOUNT)
            .bind(balance_after)
            .bind(&pay_result.external_order_no)
            .execute(&state.db)
            .await?;

            tracing::info!(
                "[AutoRecharge] success openid={} order_no={} balance_after={}",
                user.openid,
                order_no,
                balance_after
            );

            success_count += 1;
            results.push(AutoRechargeUserResult {
                openid: user.openid,
                success: true,
                fail_reason: None,
                external_order_no: pay_result.external_order_no,
            });
        } else {
            let reason = pay_result
                .fail_reason
                .unwrap_or_else(|| "扣款失败".to_string());

            // 订单改为已取消（status=4），remark 写失败原因
            sqlx::query("UPDATE orders SET status = 4, remark = ? WHERE id = ?")
                .bind(&reason)
                .bind(order_id)
                .execute(&state.db)
                .await?;

            // 当前余额不变，读取用于记录流水
            let balance_now = account::current_balance(&state, &user.openid).await?;

            // 写失败流水
            sqlx::query(
                "INSERT INTO balance_transactions \
                 (openid, amount, balance_after, `type`, external_order_no, status, remark) \
                 VALUES (?, ?, ?, 1, NULL, 0, ?)",
            )
            .bind(&user.openid)
            .bind(AUTO_RECHARGE_AMOUNT)
            .bind(balance_now)
            .bind(&reason)
            .execute(&state.db)
            .await?;

            tracing::warn!(
                "[AutoRecharge] failed openid={} order_no={} reason={}",
                user.openid,
                order_no,
                reason
            );

            fail_count += 1;
            results.push(AutoRechargeUserResult {
                openid: user.openid,
                success: false,
                fail_reason: Some(reason),
                external_order_no: None,
            });
        }
    }

    tracing::info!(
        "[AutoRecharge] done total={} success={} fail={} skipped={}",
        total,
        success_count,
        fail_count,
        skipped_count
    );

    Ok(Json(ApiResponse::success(AutoRechargeResp {
        total,
        success_count,
        fail_count,
        skipped_count,
        results,
    })))
}

/// GET /api/admin/wechat/users/{openid}/balance
pub async fn get_user_balance(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(openid): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    check_admin(&state, &headers).await?;

    let balance = account::current_balance(&state, &openid).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "openid": openid,
        "balance": balance
    }))))
}

/// GET /api/admin/wechat/users/{openid}/balance/transactions
pub async fn get_user_transactions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(openid): Path<String>,
) -> Result<Json<ApiResponse<BalanceResp>>, AppError> {
    check_admin(&state, &headers).await?;

    let balance = account::current_balance(&state, &openid).await?;
    let txs = account::recent_balance_transactions(&state, &openid, 200).await?;

    Ok(Json(ApiResponse::success(BalanceResp {
        balance,
        transactions: txs.into_iter().map(BalanceTransactionResp::from).collect(),
    })))
}
