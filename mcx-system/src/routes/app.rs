use super::admin::auth::{get_codes, login, logout, refresh};
use super::admin::{
    admin_user, category, dashboard, goods as admin_goods, logs as admin_logs,
    order as admin_order, product, subscription as admin_subscription, upload, wechat_user,
};
use super::mini_app::{
    address, auth as mini_auth, category as mini_category, goods as mini_goods,
    order as mini_order, subscription as mini_subscription,
};
use crate::state::AppState;
use axum::{
    middleware,
    routing::{delete, get, post, put},
    Json, Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

#[derive(serde::Serialize)]
struct HealthResponse {
    status: String,
    service: String,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "OK".to_string(),
        service: "welfare-store-api".to_string(),
    })
}

pub fn build_app(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
        .route("/auth/codes", get(get_codes))
        .route("/api/admin/permissions", get(admin_user::list_permissions))
        .route("/api/admin/admin-users", get(admin_user::list_admin_users))
        .route(
            "/api/admin/admin-users/{id}/permissions",
            put(admin_user::update_admin_user_permissions),
        )
        .route("/api/admin/dashboard", get(dashboard::get_dashboard_data))
        .route("/api/admin/logs/recent", get(admin_logs::get_recent_logs))
        .route("/api/admin/products", get(product::list_products))
        .route("/api/admin/products", post(product::create_product))
        .route("/api/admin/products/{id}", get(product::get_product))
        .route("/api/admin/products/{id}", put(product::update_product))
        .route("/api/admin/products/{id}", delete(product::delete_product))
        .route("/api/admin/categories", get(category::list_categories))
        .route("/api/admin/categories", post(category::create_category))
        .route("/api/admin/categories/{id}", put(category::update_category))
        .route(
            "/api/admin/categories/{id}",
            delete(category::delete_category),
        )
        .route("/api/admin/goods", get(admin_goods::list_goods))
        .route("/api/admin/goods", post(admin_goods::create_goods))
        .route("/api/admin/goods/{id}", get(admin_goods::get_goods))
        .route("/api/admin/goods/{id}", put(admin_goods::update_goods))
        .route("/api/admin/goods/{id}", delete(admin_goods::delete_goods))
        .route(
            "/api/admin/upload/signature",
            get(upload::get_upload_signature),
        )
        .route(
            "/api/admin/wechat/users",
            get(wechat_user::list_wechat_users),
        )
        .route(
            "/api/admin/wechat/users",
            post(wechat_user::create_wechat_user),
        )
        .route(
            "/api/admin/wechat/users/{id}",
            get(wechat_user::get_wechat_user),
        )
        .route(
            "/api/admin/wechat/users/{id}",
            put(wechat_user::update_wechat_user),
        )
        .route(
            "/api/admin/wechat/users/{id}",
            delete(wechat_user::delete_wechat_user),
        )
        .route(
            "/api/admin/wechat/users/by-openid/{openid}",
            put(wechat_user::update_wechat_user_by_openid),
        )
        .route(
            "/api/admin/wechat/users/check-id-card",
            post(wechat_user::check_id_card_number_exists),
        )
        .route(
            "/api/admin/wechat/users/{openid}/payment-password",
            get(wechat_user::get_payment_password),
        )
        .route("/api/mini/login", post(mini_auth::wechat_login))
        .route("/api/mini/userinfo", get(mini_auth::get_my_userinfo))
        .route("/api/mini/userinfo", put(mini_auth::update_my_userinfo))
        .route("/api/mini/check-id-card", get(mini_auth::check_my_id_card))
        .route("/api/goods/list", get(mini_goods::list_goods))
        .route("/api/goods/detail", get(mini_goods::get_goods_detail))
        .route("/api/mini/categories", get(mini_category::list_categories))
        .route("/api/mini/addresses", get(address::list_addresses))
        .route("/api/mini/addresses", post(address::create_address))
        .route("/api/mini/addresses/{id}", get(address::get_address))
        .route("/api/mini/addresses/{id}", put(address::update_address))
        .route("/api/mini/addresses/{id}", delete(address::delete_address))
        .route(
            "/api/mini/addresses/{id}/default",
            put(address::set_default_address),
        )
        .route("/api/mini/orders", post(mini_order::create_order))
        .route("/api/mini/orders", get(mini_order::list_my_orders))
        .route("/api/mini/orders/{id}", get(mini_order::get_my_order))
        .route(
            "/api/mini/orders/{id}/cancel",
            put(mini_order::cancel_my_order),
        )
        .route(
            "/api/mini/orders/{id}/receive",
            put(mini_order::confirm_my_order_received),
        )
        .route("/api/mini/orders/{id}/pay", post(mini_order::pay_order))
        .route(
            "/api/mini/orders/{id}/balance-pay",
            post(mini_order::pay_order_with_balance),
        )
        .route("/api/admin/orders", get(admin_order::list_orders))
        .route("/api/admin/orders/{id}", get(admin_order::get_order))
        .route(
            "/api/admin/orders/{id}/status",
            put(admin_order::update_order_status),
        )
        .route(
            "/api/mini/subscription",
            post(mini_subscription::set_subscription),
        )
        .route(
            "/api/mini/subscription",
            get(mini_subscription::get_subscription),
        )
        .route(
            "/api/mini/subscription/ability",
            get(mini_subscription::check_subscription_ability),
        )
        .route("/api/mini/balance", get(mini_subscription::get_balance))
        .route("/api/mini/recharge", post(mini_subscription::recharge))
        .route(
            "/api/admin/subscription/auto-recharge",
            post(admin_subscription::auto_recharge),
        )
        .route(
            "/api/admin/subscription/records",
            get(admin_subscription::list_subscription_records),
        )
        .route(
            "/api/admin/wechat/users/{openid}/balance",
            get(admin_subscription::get_user_balance),
        )
        .route(
            "/api/admin/wechat/users/{openid}/balance/transactions",
            get(admin_subscription::get_user_transactions),
        )
        .layer(middleware::from_fn_with_state(
            state.jwt_secret.clone(),
            crate::routes::middleware::request_log_middleware,
        ))
        .layer(cors)
        .with_state(state)
}
