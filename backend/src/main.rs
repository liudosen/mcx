mod config;
mod error;
mod models;
mod routes;
mod services;
mod state;

use axum::{
    routing::{delete, get, post, put},
    Json, Router,
};
use routes::admin::auth::{get_codes, login, logout, refresh};
use routes::admin::subscription as admin_subscription;
use routes::admin::{
    category, dashboard, goods as admin_goods, order as admin_order, product, upload, wechat_user,
};
use routes::mini_app::{
    address, auth as mini_auth, category as mini_category, goods as mini_goods,
    order as mini_order, subscription as mini_subscription,
};
use state::AppState;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Health check response
#[derive(serde::Serialize)]
struct HealthResponse {
    status: String,
    service: String,
}

/// Health check endpoint
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "OK".to_string(),
        service: "welfare-store-api".to_string(),
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,welfare_store_api=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Welfare Store API");

    // Load configuration
    let config = config::Config::from_env()?;
    tracing::info!(
        "Configuration loaded: {}:{}",
        config.server_host,
        config.server_port
    );

    // Initialize database
    let db = sqlx::MySqlPool::connect(&config.database_url).await?;
    tracing::info!("Database connected");

    // Initialize Redis
    let redis_client =
        redis::Client::open(config.redis_url.as_str()).expect("Failed to create Redis client");
    let redis_conn = redis::aio::ConnectionManager::new(redis_client)
        .await
        .expect("Failed to connect to Redis");
    tracing::info!("Redis connected");

    // Run migrations
    sqlx::migrate!("./migrations").run(&db).await?;
    tracing::info!("Migrations completed");

    // Create initial admin user
    if let (Ok(admin_username), Ok(admin_password)) = (
        std::env::var("ADMIN_USERNAME"),
        std::env::var("ADMIN_PASSWORD"),
    ) {
        // Check if user exists first
        let exists: Option<(u64,)> =
            sqlx::query_as("SELECT id FROM admin_users WHERE username = ?")
                .bind(&admin_username)
                .fetch_optional(&db)
                .await?;

        if exists.is_none() {
            let password_hash = bcrypt::hash(&admin_password, config.bcrypt_cost)?;
            sqlx::query(
                "INSERT INTO admin_users (username, password_hash, role, is_active) VALUES (?, ?, 'admin', 1)"
            )
            .bind(&admin_username)
            .bind(&password_hash)
            .execute(&db)
            .await?;
            tracing::info!("Initial admin user created: {}", admin_username);
        } else {
            tracing::info!("Admin user already exists, skipping creation");
        }
    }

    // Create shared application state
    let app_state = Arc::new(AppState {
        db,
        redis: redis_conn,
        jwt_secret: config.jwt_secret.clone(),
        jwt_expiry_hours: config.jwt_expiry_hours,
        bcrypt_cost: config.bcrypt_cost,
        wechat_appid: config.wechat_appid.clone(),
        wechat_secret: config.wechat_secret.clone(),
        jk_seller_username: config.jk_seller_username.clone(),
        jk_seller_password: config.jk_seller_password.clone(),
        oss_endpoint: config.oss_endpoint.clone(),
        oss_access_key_id: config.oss_access_key_id.clone(),
        oss_access_key_secret: config.oss_access_key_secret.clone(),
        oss_bucket: config.oss_bucket.clone(),
        oss_domain: config.oss_domain.clone(),
    });

    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(health))
        // ========== 管理后台 API (admin) ==========
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
        .route("/auth/codes", get(get_codes))
        // 监控大屏
        .route("/api/admin/dashboard", get(dashboard::get_dashboard_data))
        // 产品管理（旧接口，保留兼容）
        .route("/api/admin/products", get(product::list_products))
        .route("/api/admin/products", post(product::create_product))
        .route("/api/admin/products/{id}", get(product::get_product))
        .route("/api/admin/products/{id}", put(product::update_product))
        .route("/api/admin/products/{id}", delete(product::delete_product))
        // 商品分类管理
        .route("/api/admin/categories", get(category::list_categories))
        .route("/api/admin/categories", post(category::create_category))
        .route("/api/admin/categories/{id}", put(category::update_category))
        .route(
            "/api/admin/categories/{id}",
            delete(category::delete_category),
        )
        // 商品管理（新 SPU/SKU 接口）
        .route("/api/admin/goods", get(admin_goods::list_goods))
        .route("/api/admin/goods", post(admin_goods::create_goods))
        .route("/api/admin/goods/{id}", get(admin_goods::get_goods))
        .route("/api/admin/goods/{id}", put(admin_goods::update_goods))
        .route("/api/admin/goods/{id}", delete(admin_goods::delete_goods))
        // 文件上传
        .route(
            "/api/admin/upload/signature",
            get(upload::get_upload_signature),
        )
        // 微信用户管理
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
        // ========== 小程序 API (mini-app) ==========
        // 微信登录
        .route("/api/mini/login", post(mini_auth::wechat_login))
        // 用户信息
        .route("/api/mini/userinfo", get(mini_auth::get_my_userinfo))
        .route("/api/mini/userinfo", put(mini_auth::update_my_userinfo))
        // 身份证号查询
        .route("/api/mini/check-id-card", get(mini_auth::check_my_id_card))
        // 商品查询（小程序）
        .route("/api/goods/list", get(mini_goods::list_goods))
        .route("/api/goods/detail", get(mini_goods::get_goods_detail))
        // 分类查询（小程序）
        .route("/api/mini/categories", get(mini_category::list_categories))
        // 收货地址管理
        .route("/api/mini/addresses", get(address::list_addresses))
        .route("/api/mini/addresses", post(address::create_address))
        .route("/api/mini/addresses/{id}", get(address::get_address))
        .route("/api/mini/addresses/{id}", put(address::update_address))
        .route("/api/mini/addresses/{id}", delete(address::delete_address))
        .route(
            "/api/mini/addresses/{id}/default",
            put(address::set_default_address),
        )
        // 订单管理（小程序）
        .route("/api/mini/orders", post(mini_order::create_order))
        .route("/api/mini/orders", get(mini_order::list_my_orders))
        .route("/api/mini/orders/{id}", get(mini_order::get_my_order))
        .route(
            "/api/mini/orders/{id}/cancel",
            put(mini_order::cancel_my_order),
        )
        .route("/api/mini/orders/{id}/pay", post(mini_order::pay_order))
        // 订单管理（管理后台）
        .route("/api/admin/orders", get(admin_order::list_orders))
        .route("/api/admin/orders/{id}", get(admin_order::get_order))
        .route(
            "/api/admin/orders/{id}/status",
            put(admin_order::update_order_status),
        )
        // ========== 订阅服务 + 储值 ==========
        // 小程序
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
        // 管理后台
        .route(
            "/api/admin/subscription/auto-recharge",
            post(admin_subscription::auto_recharge),
        )
        .route(
            "/api/admin/wechat/users/{openid}/balance",
            get(admin_subscription::get_user_balance),
        )
        .route(
            "/api/admin/wechat/users/{openid}/balance/transactions",
            get(admin_subscription::get_user_transactions),
        )
        .layer(cors)
        .with_state(app_state);

    // Start server
    let addr = format!("{}:{}", config.server_host, config.server_port);
    tracing::info!("API service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
