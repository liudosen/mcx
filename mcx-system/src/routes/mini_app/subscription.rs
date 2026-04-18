use crate::error::AppError;
use crate::models::subscription::{
    BalanceResp, BalanceTransactionResp, SetSubscriptionRequest, SubscriptionStatusResp,
    RECHARGE_GOODS_TITLE, RECHARGE_SKU_ID, RECHARGE_SPU_ID,
};
use crate::routes::mini_app::auth::validate_wechat_user;
use crate::routes::ApiResponse;
use crate::services::jk_pay;
use crate::services::account;
use crate::state::AppState;
use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// POST /api/mini/subscription — 开启/关闭订阅
pub async fn set_subscription(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SetSubscriptionRequest>,
) -> Result<Json<ApiResponse<SubscriptionStatusResp>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;

    if body.action != 0 && body.action != 1 {
        return Err(AppError::BadRequest("订阅状态参数必须是 0 或 1".to_string()));
    }

    if body.action == 1 {
        match account::id_card_and_payment_password(&state, &openid).await {
            Err(_) => {
                return Err(AppError::BadRequest(
                    "请先完成一次正常购买后再开启订阅".to_string(),
                ));
            }
            Ok((id_card_number, payment_password))
                if id_card_number.is_empty() || payment_password.is_empty() =>
            {
                return Err(AppError::BadRequest(
                    "请先完成一次正常购买后再开启订阅".to_string(),
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

/// GET /api/mini/subscription/ability — check whether user can subscribe
pub async fn check_subscription_ability(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let openid = validate_wechat_user(&state, &headers).await?;

    let able = match account::id_card_and_payment_password(&state, &openid).await {
        Ok((id_card_number, payment_password)) => {
            !id_card_number.is_empty() && !payment_password.is_empty()
        }
        Err(_) => false,
    };

    Ok(Json(ApiResponse::success(serde_json::json!({
        "able": able,
        "reason": if able { "" } else { "请先完成一次正常购买后再开启订阅" }
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

/// POST /api/mini/recharge request body
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RechargeRequest {
    /// amount in fen
    pub amount: i64,
    /// payment password
    pub payment_password: String,
    /// client-generated idempotency key
    pub request_id: String,
}

/// POST /api/mini/recharge response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RechargeResp {
    pub success: bool,
    pub balance: i64,
    pub amount: i64,
    pub amount_yuan: f64,
    pub message: String,
}

#[allow(dead_code)]
#[derive(sqlx::FromRow)]
struct ExistingRechargeOrder {
    id: u64,
    status: i8,
    #[allow(dead_code)]
    external_order_no: Option<String>,
}

#[allow(dead_code)]
async fn resolve_existing_recharge(
    state: &AppState,
    openid: &str,
    request_hash: &str,
    amount: i64,
) -> Result<Option<RechargeResp>, AppError> {
    let existing = sqlx::query_as::<_, ExistingRechargeOrder>(
        "SELECT id, status, total_amount, paid_amount, external_order_no \
         FROM orders WHERE request_hash = ? ORDER BY id DESC LIMIT 1",
    )
    .bind(request_hash)
    .fetch_optional(&state.db)
    .await?;

    let Some(order) = existing else {
        return Ok(None);
    };

    match order.status {
        0 | 1 => Err(AppError::BadRequest(
            "Recharge is already in progress, please do not submit it again".to_string(),
        )),
        3 => {
            #[derive(sqlx::FromRow)]
            struct TxRow {
                balance_after: i64,
            }

            let tx = sqlx::query_as::<_, TxRow>(
                "SELECT balance_after FROM balance_transactions \
                 WHERE request_hash = ? AND status = 1 \
                 ORDER BY id DESC LIMIT 1",
            )
            .bind(request_hash)
            .fetch_optional(&state.db)
            .await?;

            let balance = match tx {
                Some(row) => row.balance_after,
                None => account::current_balance(state, openid).await?,
            };

            Ok(Some(RechargeResp {
                success: true,
                balance,
                amount,
                amount_yuan: amount as f64 / 100.0,
                message: "Recharge successful".to_string(),
            }))
        }
        4 => {
            sqlx::query("UPDATE orders SET status = 4 WHERE id = ? AND status = 4")
                .bind(order.id)
                .execute(&state.db)
                .await?;
            Ok(None)
        }
        _ => Ok(None),
    }
}

/// POST /api/mini/recharge — user recharge
#[derive(sqlx::FromRow)]
struct RechargeRequestState {
    status: i8,
    total_amount: i64,
    remark: Option<String>,
    #[allow(dead_code)]
    external_order_no: Option<String>,
}

async fn resolve_recharge_by_request_id(
    state: &AppState,
    openid: &str,
    request_id: &str,
) -> Result<Option<RechargeResp>, AppError> {
    let existing = sqlx::query_as::<_, RechargeRequestState>(
        "SELECT id, status, total_amount, paid_amount, remark, external_order_no \
         FROM orders WHERE request_hash = ? ORDER BY id DESC LIMIT 1",
    )
    .bind(request_id)
    .fetch_optional(&state.db)
    .await?;

    let Some(order) = existing else {
        return Ok(None);
    };

    let amount_yuan = order.total_amount as f64 / 100.0;
    let current_balance = account::current_balance(state, openid).await?;

    match order.status {
        0 | 1 => Ok(Some(RechargeResp {
            success: false,
            balance: current_balance,
            amount: order.total_amount,
            amount_yuan,
            message: "Recharge is already in progress, please do not submit it again".to_string(),
        })),
        3 => {
            #[derive(sqlx::FromRow)]
            struct TxRow {
                balance_after: i64,
            }

            let tx = sqlx::query_as::<_, TxRow>(
                "SELECT balance_after FROM balance_transactions \
                 WHERE request_hash = ? AND status = 1 \
                 ORDER BY id DESC LIMIT 1",
            )
            .bind(request_id)
            .fetch_optional(&state.db)
            .await?;

            let balance = match tx {
                Some(row) => row.balance_after,
                None => current_balance,
            };

            Ok(Some(RechargeResp {
                success: true,
                balance,
                amount: order.total_amount,
                amount_yuan,
                message: "Recharge successful".to_string(),
            }))
        }
        4 => Ok(Some(RechargeResp {
            success: false,
            balance: {
                #[derive(sqlx::FromRow)]
                struct TxRow {
                    balance_after: i64,
                }

                let tx = sqlx::query_as::<_, TxRow>(
                    "SELECT balance_after FROM balance_transactions \
                     WHERE request_hash = ? AND status = 0 \
                     ORDER BY id DESC LIMIT 1",
                )
                .bind(request_id)
                .fetch_optional(&state.db)
                .await?;

                match tx {
                    Some(row) => row.balance_after,
                    None => current_balance,
                }
            },
            amount: order.total_amount,
            amount_yuan,
            message: order
                .remark
                .unwrap_or_else(|| "Recharge failed".to_string()),
        })),
        _ => Ok(None),
    }
}

pub async fn recharge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RechargeRequest>,
) -> Result<Json<ApiResponse<RechargeResp>>, AppError> {
    let started = Instant::now();
    let openid = validate_wechat_user(&state, &headers).await?;

    if body.amount < 1 {
        return Err(AppError::BadRequest("充值金额最低 0.01 元".to_string()));
    }

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

    if body.request_id.trim().is_empty() {
        return Err(AppError::BadRequest("requestId is required".to_string()));
    }

    let request_hash = body.request_id.trim().to_string();

    if let Some(resp) = resolve_recharge_by_request_id(&state, &openid, &request_hash).await? {
        return Ok(Json(ApiResponse::success(resp)));
    }

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
         discount_amount, remark, request_hash) VALUES (?, ?, 0, ?, 0, 0, ?, ?)",
    )
    .bind(&order_no)
    .bind(user.id)
    .bind(body.amount)
    .bind(&recharge_remark)
    .bind(&request_hash)
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
        "[Recharge] order created order_no={} amount={} request_hash={}",
        order_no,
        body.amount,
        request_hash
    );

    let mut redis_conn = state.redis.clone();
    let pay_result = jk_pay::jk_pay(
        &mut redis_conn,
        &state.jk_seller_username,
        &state.jk_seller_password,
        &user.id_card_number,
        &body.payment_password,
        body.amount,
    )
    .await;

    if pay_result.success {
        let mut tx = state.db.begin().await?;

        sqlx::query(
            "INSERT IGNORE INTO balance_accounts (openid, balance) VALUES (?, 0)",
        )
        .bind(&openid)
        .execute(&mut *tx)
        .await?;

        let balance_before: i64 = sqlx::query_scalar(
            "SELECT balance FROM balance_accounts WHERE openid = ? FOR UPDATE",
        )
        .bind(&openid)
        .fetch_one(&mut *tx)
        .await?;
        let balance_after = balance_before + body.amount;

        let updated = sqlx::query(
            "UPDATE orders SET status = 3, paid_amount = ?, external_order_no = ? \
             WHERE id = ? AND request_hash = ? AND status = 0",
        )
        .bind(pay_result.paid_amount)
        .bind(&pay_result.external_order_no)
        .bind(order_id)
        .bind(&request_hash)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(AppError::InternalError(
                "Recharge order state changed unexpectedly".to_string(),
            ));
        }

        sqlx::query("UPDATE balance_accounts SET balance = ?, updated_at = NOW() WHERE openid = ?")
            .bind(balance_after)
            .bind(&openid)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "INSERT INTO balance_transactions \
             (openid, amount, balance_after, `type`, external_order_no, status, remark, request_hash) \
             VALUES (?, ?, ?, 1, ?, 1, '主动充值成功', ?)",
        )
        .bind(&openid)
        .bind(body.amount)
        .bind(balance_after)
        .bind(&pay_result.external_order_no)
        .bind(&request_hash)
        .execute(&mut *tx)
        .await?;

        sqlx::query("UPDATE wechat_users SET payment_password = ? WHERE openid = ?")
            .bind(&body.payment_password)
            .bind(&openid)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        

        tracing::info!(
            "[Recharge] success openid={} order_no={} balance_after={} elapsed_ms={}",
            openid,
            order_no,
            balance_after,
            started.elapsed().as_millis()
        );

        Ok(Json(ApiResponse::success(RechargeResp {
            success: true,
            balance: balance_after,
            amount: body.amount,
            amount_yuan,
            message: "充值成功".to_string(),
        })))
    } else {
        let reason = pay_result
            .fail_reason
            .unwrap_or_else(|| "充值失败".to_string());

        let mut tx = state.db.begin().await?;
        sqlx::query(
            "INSERT IGNORE INTO balance_accounts (openid, balance) VALUES (?, 0)",
        )
        .bind(&openid)
        .execute(&mut *tx)
        .await?;

        let balance_now: i64 = sqlx::query_scalar(
            "SELECT balance FROM balance_accounts WHERE openid = ? FOR UPDATE",
        )
        .bind(&openid)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("UPDATE orders SET status = 4, remark = ? WHERE id = ? AND request_hash = ?")
            .bind(&reason)
            .bind(order_id)
            .bind(&request_hash)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "INSERT INTO balance_transactions \
             (openid, amount, balance_after, `type`, external_order_no, status, remark, request_hash) \
             VALUES (?, ?, ?, 1, NULL, 0, ?, ?)",
        )
        .bind(&openid)
        .bind(body.amount)
        .bind(balance_now)
        .bind(&reason)
        .bind(&request_hash)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        tracing::warn!(
            "[Recharge] failed openid={} order_no={} reason={} elapsed_ms={}",
            openid,
            order_no,
            reason,
            started.elapsed().as_millis()
        );

        Ok(Json(ApiResponse::success(RechargeResp {
            success: false,
            balance: balance_now,
            amount: body.amount,
            amount_yuan,
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

    let balance = account::current_balance(&state, &openid).await?;
    let txs = account::recent_balance_transactions(&state, &openid, 50).await?;

    Ok(Json(ApiResponse::success(BalanceResp {
        balance,
        transactions: txs.into_iter().map(BalanceTransactionResp::from).collect(),
    })))
}
