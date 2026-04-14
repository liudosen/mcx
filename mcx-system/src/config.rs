use std::env;

use urlencoding::encode;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    #[allow(dead_code)]
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    pub server_host: String,
    pub server_port: u16,
    #[allow(dead_code)]
    pub bcrypt_cost: u32,
    pub wechat_appid: String,
    pub wechat_secret: String,
    pub dev_wechat_openid: Option<String>,
    pub jk_seller_username: String,
    pub jk_seller_password: String,
    pub oss_endpoint: String,
    pub oss_access_key_id: String,
    pub oss_access_key_secret: String,
    pub oss_bucket: String,
    pub oss_domain: String,
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
}

fn env_required(name: &str) -> Result<String, env::VarError> {
    let value = env::var(name)?;
    if value.trim().is_empty() {
        return Err(env::VarError::NotPresent);
    }
    Ok(value)
}

fn build_mysql_url() -> Result<String, env::VarError> {
    let host = env_or_default("DATABASE_HOST", "127.0.0.1");
    let port = env_or_default("DATABASE_PORT", "3306");
    let database = env_or_default("DATABASE_NAME", "welfare_store");
    let user = env_or_default("DATABASE_USER", "root");
    let password = env::var("DATABASE_PASSWORD").unwrap_or_default();

    let user = encode(&user);
    let password = encode(&password);

    let url = if password.is_empty() {
        format!("mysql://{}@{}:{}/{}", user, host, port, database)
    } else {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            user, password, host, port, database
        )
    };

    Ok(url)
}

fn build_redis_url() -> Result<String, env::VarError> {
    let host = env_or_default("REDIS_HOST", "127.0.0.1");
    let port = env_or_default("REDIS_PORT", "6379");
    let db = env_or_default("REDIS_DB", "0");
    let username = env_or_default("REDIS_USERNAME", "default");
    let password = env::var("REDIS_PASSWORD").unwrap_or_default();

    let url = if password.is_empty() {
        if username == "default" {
            format!("redis://{}:{}/{}", host, port, db)
        } else {
            format!("redis://{}@{}:{}/{}", encode(&username), host, port, db)
        }
    } else {
        format!(
            "redis://{}:{}@{}:{}/{}",
            encode(&username),
            encode(&password),
            host,
            port,
            db
        )
    };

    Ok(url)
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Config {
            database_url: build_mysql_url()?,
            redis_url: build_redis_url()?,
            jwt_secret: env_required("JWT_SECRET")?,
            jwt_expiry_hours: env_or_default("JWT_EXPIRY_HOURS", "24")
                .parse()
                .unwrap_or(24),
            server_host: env_or_default("SERVER_HOST", "127.0.0.1"),
            server_port: env_or_default("SERVER_PORT", "8080")
                .parse()
                .unwrap_or(8080),
            bcrypt_cost: env_or_default("BCRYPT_COST", "12").parse().unwrap_or(12),
            wechat_appid: env_required("WEIXIN_APPID")?,
            wechat_secret: env_required("WEIXIN_SECRET")?,
            dev_wechat_openid: env::var("DEV_WECHAT_OPENID").ok().and_then(|v| {
                let trimmed = v.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            }),
            jk_seller_username: env_required("JK_SELLER_USERNAME")?,
            jk_seller_password: env_required("JK_SELLER_PASSWORD")?,
            oss_endpoint: env_required("OSS_ENDPOINT")?,
            oss_access_key_id: env_required("OSS_ACCESS_KEY_ID")?,
            oss_access_key_secret: env_required("OSS_ACCESS_KEY_SECRET")?,
            oss_bucket: env_required("OSS_BUCKET")?,
            oss_domain: env_required("OSS_DOMAIN")?,
        })
    }
}
