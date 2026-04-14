use crate::error::AppError;
use crate::state::AppState;

pub async fn user_id_by_openid(state: &AppState, openid: &str) -> Result<u64, AppError> {
    let user_id: u64 = sqlx::query_scalar("SELECT id FROM wechat_users WHERE openid = ?")
        .bind(openid)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("用户不存在".to_string()))?;
    Ok(user_id)
}

pub async fn id_card_and_payment_password(
    state: &AppState,
    openid: &str,
) -> Result<(String, String), AppError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id_card_number: String,
        payment_password: String,
    }

    let row = sqlx::query_as::<_, Row>(
        "SELECT id_card_number, payment_password FROM wechat_users WHERE openid = ?",
    )
    .bind(openid)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("用户不存在".to_string()))?;

    Ok((row.id_card_number, row.payment_password))
}

pub async fn current_balance(state: &AppState, openid: &str) -> Result<i64, AppError> {
    let balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE( \
             (SELECT balance FROM balance_accounts WHERE openid = ?), \
             (SELECT balance_after FROM balance_transactions WHERE openid = ? ORDER BY id DESC LIMIT 1), \
             0 \
         )",
    )
    .bind(openid)
    .bind(openid)
    .fetch_one(&state.db)
    .await?;

    Ok(balance)
}

pub async fn recent_balance_transactions(
    state: &AppState,
    openid: &str,
    limit: i64,
) -> Result<Vec<crate::models::subscription::BalanceTransaction>, AppError> {
    let txs = sqlx::query_as::<_, crate::models::subscription::BalanceTransaction>(
        "SELECT id, openid, amount, balance_after, `type`, external_order_no, \
         status, remark, created_at \
         FROM balance_transactions \
         WHERE openid = ? ORDER BY id DESC LIMIT ?",
    )
    .bind(openid)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(txs)
}
