use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use md5::Digest;
use once_cell::sync::OnceCell;
use rand::Rng;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use rsa::{pkcs8::DecodePublicKey, Pkcs1v15Encrypt, RsaPublicKey};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const APP_ID: &str = "9222001";
const SELLER_ID: &str = "248933040709";
const ID_TYPE: &str = "1";
const PAY_CHANNEL: i64 = 6;
const MAX_LOGIN_RETRY: u32 = 10;
/// Redis key for cached wtk token (expires 8 hours)
const TOKEN_REDIS_KEY: &str = "welfare:jk:wtk";
const TOKEN_TTL_SECS: u64 = 8 * 3600;

static HTTP_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();

const RSA_PUBLIC_KEY: &str = concat!(
    "MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDKzDDsrhcP7iRsbbVhn30P/38R",
    "+b4DNmV0bhrxG7lm1kBdhk8+br7g42JCK5m7Vs50FWnSXWSkNoKT+fuzg23x3WpR",
    "xu6s84FSFj9Un6H4eRFSAOKyxTQuNftr4RYDFvkRsHlGGnhiHv7dXgufD7TfaTNr",
    "fI/K4pLZRhfzcqHecwIDAQAB"
);

const ITEM_CODE: &str = "PAJKPOS1169888";
const ITEM_NAME: &str = "中医-其他";
const ITEM_CATEGORY: &str = "D04";
const ITEM_CATEGORY_DESP: &str = "保健服务";
const ITEM_GMT_MODIFIED: &str = "2025-10-29 16:56:44";

// ─── OCR ──────────────────────────────────────────────────────────────────────

static OCR: OnceCell<ddddocr::Ddddocr<'static>> = OnceCell::new();

fn get_http_client() -> Result<&'static reqwest::Client, String> {
    HTTP_CLIENT
        .get_or_try_init(|| {
            reqwest::Client::builder()
                .cookie_store(true)
                .pool_idle_timeout(std::time::Duration::from_secs(90))
                .pool_max_idle_per_host(8)
                .user_agent(
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                     AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36",
                )
                .build()
                .map_err(|e| {
                    tracing::warn!("[JK HTTP] init failed: {}", e);
                    format!("HTTP client error: {e}")
                })
        })
        .map(|client| client)
}

fn get_ocr() -> Option<&'static ddddocr::Ddddocr<'static>> {
    OCR.get_or_try_init(|| {
        ddddocr::ddddocr_classification().map_err(|e| {
            tracing::warn!("[JK OCR] init failed: {}", e);
            e
        })
    })
    .ok()
}

fn recognize_captcha(img_bytes: &[u8]) -> String {
    match get_ocr() {
        Some(ocr) => match ocr.classification(img_bytes) {
            Ok(text) => {
                tracing::info!("[JK OCR] recognized: {}", text);
                text
            }
            Err(e) => {
                tracing::warn!(
                    "[JK OCR] classification failed: {}, using random fallback",
                    e
                );
                random_captcha()
            }
        },
        None => {
            tracing::warn!("[JK OCR] not available, using random fallback");
            random_captcha()
        }
    }
}

fn random_captcha() -> String {
    let mut rng = rand::thread_rng();
    (0..4)
        .map(|_| {
            let charset = b"abcdefghijklmnopqrstuvwxyz0123456789";
            charset[rng.gen_range(0..charset.len())] as char
        })
        .collect()
}

// ─── Crypto ───────────────────────────────────────────────────────────────────

fn md5_hex(s: &str) -> String {
    let mut hasher = md5::Md5::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn calc_sig(params: &BTreeMap<String, String>, wtk: &str) -> String {
    let mut parts = String::new();
    for (k, v) in params.iter() {
        if k == "_sig" {
            continue;
        }
        parts.push_str(k);
        parts.push('=');
        parts.push_str(v);
    }
    if !wtk.is_empty() {
        parts.push_str(wtk);
    } else {
        parts.push_str("jk.pingan.com");
    }
    md5_hex(&parts)
}

fn pwd_hash(password: &str) -> String {
    md5_hex(&format!("{}pajk.cn", password))
}

fn rsa_encrypt(plain: &str) -> Result<String, String> {
    let pem = format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
        RSA_PUBLIC_KEY
            .chars()
            .collect::<Vec<_>>()
            .chunks(64)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    );
    let public_key =
        RsaPublicKey::from_public_key_pem(&pem).map_err(|e| format!("RSA key error: {e}"))?;
    let mut rng = rand::thread_rng();
    let encrypted = public_key
        .encrypt(&mut rng, Pkcs1v15Encrypt, plain.as_bytes())
        .map_err(|e| format!("RSA encrypt error: {e}"))?;
    Ok(BASE64.encode(&encrypted))
}

fn make_card_password(card_no: &str, card_password: &str) -> Result<String, String> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let payload = json!({
        "cardNo": card_no,
        "pd": card_password,
        "timestamp": ts
    });
    let payload_str = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    rsa_encrypt(&payload_str)
}

// ─── HTTP client ──────────────────────────────────────────────────────────────

struct JkClient {
    client: reqwest::Client,
}

impl JkClient {
    fn new() -> Result<Self, String> {
        Ok(Self {
            client: get_http_client()?.clone(),
        })
    }

    async fn api(&self, mt: &str, extra: &[(&str, &str)], wtk: &str) -> Result<Value, String> {
        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("_mt".to_string(), mt.to_string());
        params.insert("_sm".to_string(), "md5".to_string());
        params.insert("_aid".to_string(), APP_ID.to_string());
        for (k, v) in extra {
            params.insert(k.to_string(), v.to_string());
        }
        if !wtk.is_empty() {
            params.insert("_wtk".to_string(), wtk.to_string());
        }
        let sig = calc_sig(&params, wtk);
        params.insert("_sig".to_string(), sig);

        let form: Vec<(String, String)> = params.into_iter().collect();
        let url = format!("https://api.jk.cn/m.api?_mt={mt}");

        let resp = self
            .client
            .post(&url)
            .header("Origin", "https://www.jk.cn")
            .header("Referer", "https://www.jk.cn/")
            .header(
                "Content-Type",
                "application/x-www-form-urlencoded;charset=UTF-8",
            )
            .form(&form)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;

        resp.json::<Value>()
            .await
            .map_err(|e| format!("JSON parse error: {e}"))
    }

    async fn login(&self, username: &str, password: &str) -> Result<String, String> {
        let pwd_hashed = pwd_hash(password);

        for attempt in 1..=MAX_LOGIN_RETRY {
            // Get captcha
            let mut cap_params: BTreeMap<String, String> = BTreeMap::new();
            cap_params.insert("_mt".to_string(), "kylin.requestCaptcha".to_string());
            cap_params.insert("_sm".to_string(), "md5".to_string());
            cap_params.insert("_aid".to_string(), APP_ID.to_string());
            let sig = calc_sig(&cap_params, "");
            cap_params.insert("_sig".to_string(), sig);
            let cap_form: Vec<(String, String)> = cap_params.into_iter().collect();

            let cap_data: Value = self
                .client
                .post("https://api.jk.cn/m.api?_mt=kylin.requestCaptcha")
                .header("Origin", "https://www.jk.cn")
                .header("Referer", "https://www.jk.cn/")
                .header(
                    "Content-Type",
                    "application/x-www-form-urlencoded;charset=UTF-8",
                )
                .form(&cap_form)
                .send()
                .await
                .map_err(|e| format!("captcha request error: {e}"))?
                .json()
                .await
                .map_err(|e| format!("captcha parse error: {e}"))?;

            let content = cap_data
                .get("content")
                .and_then(|c| c.as_array())
                .and_then(|a| a.first())
                .ok_or("no captcha content")?;

            let img_url = content
                .get("imgUrl")
                .and_then(|v| v.as_str())
                .ok_or("no imgUrl")?;
            let cap_key = content
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or("no key")?
                .to_string();

            // Download captcha image
            let img_bytes = self
                .client
                .get(img_url)
                .send()
                .await
                .map_err(|e| format!("img download error: {e}"))?
                .bytes()
                .await
                .map_err(|e| format!("img bytes error: {e}"))?;

            // OCR recognition using ddddocr
            let cap_text = recognize_captcha(&img_bytes);

            tracing::info!(
                "[JK Login] attempt {}/{} captcha={}",
                attempt,
                MAX_LOGIN_RETRY,
                cap_text
            );

            let resp: Value = self
                .client
                .post("https://jk.cn/login/loginname")
                .header("Origin", "https://www.jk.cn")
                .header("Referer", "https://www.jk.cn/")
                .form(&[
                    ("loginName", username),
                    ("password", &pwd_hashed),
                    ("captcha", &cap_text),
                    ("_cap", &cap_key),
                    ("appId", APP_ID),
                ])
                .send()
                .await
                .map_err(|e| format!("login request error: {e}"))?
                .json()
                .await
                .map_err(|e| format!("login parse error: {e}"))?;

            if resp.get("success").and_then(|v| v.as_bool()) == Some(true) {
                let wtk = resp
                    .pointer("/model/_wtk")
                    .and_then(|v| v.as_str())
                    .ok_or("no wtk in response")?
                    .to_string();
                tracing::info!("[JK Login] success wtk={}", wtk);
                return Ok(wtk);
            }

            let err_code = resp
                .get("errorCode")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let err_msg = resp
                .get("errorMessage")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            tracing::warn!("[JK Login] failed: {} {}", err_code, err_msg);
        }
        Err("登录失败，已超过最大重试次数".to_string())
    }
}

// ─── Token management (Redis cache) ──────────────────────────────────────────

fn should_clear_cached_token(err: &str) -> bool {
    let msg = err.to_lowercase();
    msg.contains("wtk")
        || msg.contains("token")
        || msg.contains("登录")
        || msg.contains("登陆")
        || msg.contains("expired")
        || msg.contains("失效")
        || msg.contains("无效")
}

async fn get_token(
    redis: &mut ConnectionManager,
    jk: &JkClient,
    username: &str,
    password: &str,
) -> Result<String, String> {
    // Try cached token first
    let cached: Option<String> = redis
        .get(TOKEN_REDIS_KEY)
        .await
        .map_err(|e| format!("Redis get error: {e}"))?;

    if let Some(wtk) = cached {
        tracing::info!("[JK Token] using cached wtk from Redis (skip verify)");
        return Ok(wtk);
    } else {
        tracing::info!("[JK Token] no cached token, logging in");
    }

    let wtk = jk.login(username, password).await?;

    // Cache in Redis with TTL
    let _: () = redis
        .set_ex(TOKEN_REDIS_KEY, &wtk, TOKEN_TTL_SECS)
        .await
        .map_err(|e| format!("Redis set error: {e}"))?;
    tracing::info!("[JK Token] token cached in Redis for {}s", TOKEN_TTL_SECS);

    Ok(wtk)
}

// ─── Public API ───────────────────────────────────────────────────────────────

pub async fn warmup(
    redis: &mut ConnectionManager,
    seller_username: &str,
    seller_password: &str,
) -> Result<(), String> {
    let jk = JkClient::new()?;
    let _ = get_ocr();
    let _ = get_token(redis, &jk, seller_username, seller_password).await?;
    Ok(())
}

pub struct PayResult {
    pub success: bool,
    pub paid_amount: i64,
    pub order_status: Option<i64>,
    pub external_order_no: Option<String>,
    pub fail_reason: Option<String>,
}

/// Execute the full JK health-card payment flow.
///
/// - `redis`: mutable reference to Redis connection (for token caching)
/// - `seller_username` / `seller_password`: from env
/// - `card_no`: user's ID card number (健康卡号)
/// - `card_password`: user-supplied payment password
/// - `total_amount_fen`: order total in 分; converted to 元 ÷ 0.95 rounded
pub async fn jk_pay(
    redis: &mut ConnectionManager,
    seller_username: &str,
    seller_password: &str,
    card_no: &str,
    card_password: &str,
    total_amount_fen: i64,
) -> PayResult {
    match do_jk_pay(
        redis,
        seller_username,
        seller_password,
        card_no,
        card_password,
        total_amount_fen,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => PayResult {
            success: false,
            paid_amount: 0,
            order_status: None,
            external_order_no: None,
            fail_reason: Some(e),
        },
    }
}

async fn do_jk_pay(
    redis: &mut ConnectionManager,
    seller_username: &str,
    seller_password: &str,
    card_no: &str,
    card_password: &str,
    total_amount_fen: i64,
) -> Result<PayResult, String> {
    // Amount: 分 → 元 ÷ 0.95 → round to 2 decimal places (精确到分)
    let amount_yuan = ((total_amount_fen as f64) / 100.0 / 0.95 * 100.0).round() / 100.0;
    let started = Instant::now();

    tracing::info!(
        "[JK Pay] total_amount_fen={} → amount_yuan={}",
        total_amount_fen,
        amount_yuan
    );

    let jk = JkClient::new()?;

    let line = build_trade_line(amount_yuan);

    // 最多重试一次：缓存 token 失效时清掉重新登录
    for attempt in 0..2u32 {
        let token_started = Instant::now();
        let wtk = get_token(redis, &jk, seller_username, seller_password).await?;
        tracing::info!(
            "[JK Pay] get_token attempt={} elapsed_ms={}",
            attempt + 1,
            token_started.elapsed().as_millis()
        );

        // Step 1: Get jk.cn order number
        let order_no_started = Instant::now();
        let data = fetch_order_no(&jk, &wtk).await?;
        tracing::info!(
            "[JK Pay] getStoreAndOrderNo attempt={} elapsed_ms={}",
            attempt + 1,
            order_no_started.elapsed().as_millis()
        );

        // stat.code < 0 表示 token 失效（如 -360），清缓存后重试
        let stat_code = data
            .pointer("/stat/code")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        if stat_code < 0 {
            let _: () = redis.del(TOKEN_REDIS_KEY).await.unwrap_or(());
            tracing::warn!(
                "[JK Token] stat.code={} on attempt {}, clearing cache and retrying",
                stat_code,
                attempt
            );
            if attempt == 0 {
                continue;
            }
            return Err(format!("获取订单号失败(code={}): {}", stat_code, data));
        }

        let order_no = extract_order_no(&data)?;

        tracing::info!("[JK Pay] orderNo={}", order_no);

        return match try_pay_with_password(
            &jk,
            &wtk,
            card_no,
            card_password,
            &order_no,
            amount_yuan,
            &line,
        )
        .await
        {
            Ok(r) => {
                tracing::info!(
                    "[JK Pay] total elapsed_ms={} attempt={}",
                    started.elapsed().as_millis(),
                    attempt + 1
                );
                Ok(r)
            }
            Err(e) => {
                if should_clear_cached_token(&e) {
                    let _: () = redis.del(TOKEN_REDIS_KEY).await.unwrap_or(());
                    tracing::warn!(
                        "[JK Token] cleared cached token due to auth-like error: {}",
                        e
                    );
                } else {
                    tracing::warn!("[JK Pay] non-token error, keeping cached token: {}", e);
                }
                Err(e)
            }
        };
    }

    Err("获取订单号重试失败".to_string())
}

/// 简化错误提示，提取关键信息给用户
fn simplify_error_message(raw_msg: &str) -> String {
    let msg = raw_msg.to_lowercase();

    // 密码错误相关
    if msg.contains("交易密码错误") || msg.contains("密码错误") {
        if msg.contains("超限") {
            return "密码错误次数过多，请稍后再试".to_string();
        }
        return "支付密码错误".to_string();
    }

    // 余额不足
    if msg.contains("余额不足") || msg.contains("账户余额") {
        return "健康卡余额不足".to_string();
    }

    // 卡状态异常
    if msg.contains("卡状态") || msg.contains("卡片状态") {
        return "健康卡状态异常，请联系客服".to_string();
    }

    // 金额相关
    if msg.contains("金额") && msg.contains("小于") {
        return "支付金额过小".to_string();
    }

    // 验证失败
    if msg.contains("验证失败") {
        return "健康卡验证失败".to_string();
    }

    // 其他情况，去掉技术性前缀
    if let Some(idx) = raw_msg.find("试算失败：") {
        return raw_msg[idx + "试算失败：".len()..].to_string();
    }
    if let Some(idx) = raw_msg.find("预结算失败：") {
        return raw_msg[idx + "预结算失败：".len()..].to_string();
    }
    if let Some(idx) = raw_msg.find("：") {
        return raw_msg[idx + "：".len()..].to_string();
    }

    raw_msg.to_string()
}

fn build_trade_line(amount_yuan: f64) -> Value {
    json!({
        "itemCode": ITEM_CODE,
        "itemName": ITEM_NAME,
        "category": ITEM_CATEGORY,
        "categoryDesp": ITEM_CATEGORY_DESP,
        "spec": "", "brand": "", "itemType": "",
        "itemBarCode": "", "itemApprovalNumber": "",
        "itemSubCategory": "", "itemManufacturer": "",
        "gmtModified": ITEM_GMT_MODIFIED,
        "lastModifier": "",
        "qty": 1,
        "price": amount_yuan,
        "amount": amount_yuan,
        "approvalNumber": "",
        "barcode": "",
        "manufacturer": "",
        "subCategory": "",
    })
}

fn extract_order_no(data: &Value) -> Result<String, String> {
    let content = data
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| format!("获取订单号失败: {}", data))?;

    content
        .get("orderNo")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("获取订单号失败: {}", data))
}

async fn fetch_order_no(jk: &JkClient, wtk: &str) -> Result<Value, String> {
    jk.api(
        "baize.getStoreAndOrderNo",
        &[("req", "{}"), ("sellerId", SELLER_ID)],
        wtk,
    )
    .await
}

async fn query_payment_channel(
    jk: &JkClient,
    wtk: &str,
    card_no: &str,
    card_password: &str,
) -> Result<String, String> {
    let enc_pwd = make_card_password(card_no, card_password)?;
    let channel_req = json!({
        "idType": ID_TYPE,
        "cardNo": card_no,
        "password": enc_pwd,
        "queryBalance": true,
    });
    let channel_req_str =
        serde_json::to_string(&channel_req).map_err(|e| format!("json error: {e}"))?;

    let data = jk
        .api(
            "baize.queryPayChannelByEntityCard",
            &[("req", &channel_req_str), ("sellerId", SELLER_ID)],
            wtk,
        )
        .await?;

    let ch_content = data
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| {
            data.pointer("/stat/stateList")
                .and_then(|v| v.as_str())
                .unwrap_or("健康卡验证失败")
                .to_string()
        })?;

    if ch_content.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let msg = ch_content
            .get("returnMsg")
            .and_then(|v| v.as_str())
            .unwrap_or("健康卡验证失败")
            .to_string();
        return Err(simplify_error_message(&msg));
    }

    Ok(enc_pwd)
}

async fn precalc_payment(
    jk: &JkClient,
    wtk: &str,
    card_no: &str,
    enc_pwd: &str,
    order_no: &str,
    amount_yuan: f64,
    line: &Value,
) -> Result<Value, String> {
    let precalc_req = json!({
        "cardNo": card_no,
        "password": enc_pwd,
        "idType": ID_TYPE,
        "xrefNo": order_no,
        "amount": amount_yuan,
        "lines": [line],
        "payChannel": PAY_CHANNEL,
    });
    let precalc_req_str =
        serde_json::to_string(&precalc_req).map_err(|e| format!("json error: {e}"))?;

    let data = jk
        .api(
            "baize.drugCardPreCalc",
            &[("req", &precalc_req_str), ("sellerId", SELLER_ID)],
            wtk,
        )
        .await?;

    let pr = data
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| format!("预结算失败: {}", data))?;

    if pr.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let msg = pr
            .get("returnMsg")
            .and_then(|v| v.as_str())
            .unwrap_or("预结算失败");
        return Err(simplify_error_message(msg));
    }

    if pr.get("fundAmount").is_none() || pr["fundAmount"].is_null() {
        let msg = pr
            .get("returnMsg")
            .and_then(|v| v.as_str())
            .unwrap_or("预结算业务失败");
        return Err(simplify_error_message(msg));
    }

    Ok(pr.clone())
}

async fn poll_ready_plan_if_needed(jk: &JkClient, wtk: &str, order_no: &str) -> Result<(), String> {
    let poll_req = json!({"xrefNo": order_no, "payChannel": PAY_CHANNEL});
    let poll_req_str = serde_json::to_string(&poll_req).map_err(|e| format!("json error: {e}"))?;

    for i in 0..15 {
        let poll_started = Instant::now();
        let poll_data = jk
            .api(
                "baize.pollReadyPlan",
                &[("req", &poll_req_str), ("sellerId", SELLER_ID)],
                wtk,
            )
            .await?;
        tracing::info!(
            "[JK Pay] pollReadyPlan {} elapsed_ms={}",
            i + 1,
            poll_started.elapsed().as_millis()
        );

        let pr2 = poll_data
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .cloned()
            .unwrap_or(json!({}));
        tracing::info!(
            "[JK Pay] pollReadyPlan {}: finish={:?}",
            i + 1,
            pr2.get("finish")
        );

        if pr2.get("finish").and_then(|v| v.as_bool()) == Some(true) {
            return Ok(());
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    Ok(())
}

async fn execute_payment(
    jk: &JkClient,
    wtk: &str,
    order_no: &str,
    amount_yuan: f64,
    line: &Value,
) -> Result<Value, String> {
    let pay_req = json!({
        "idType": ID_TYPE,
        "xrefNo": order_no,
        "amount": amount_yuan,
        "lines": [line],
        "payChannel": PAY_CHANNEL,
    });
    let pay_req_str = serde_json::to_string(&pay_req).map_err(|e| format!("json error: {e}"))?;

    let pay_data = jk
        .api(
            "baize.drugCardPay",
            &[("req", &pay_req_str), ("sellerId", SELLER_ID)],
            wtk,
        )
        .await?;

    Ok(pay_data)
}

async fn try_pay_with_password(
    jk: &JkClient,
    wtk: &str,
    card_no: &str,
    card_password: &str,
    order_no: &str,
    amount_yuan: f64,
    line: &Value,
) -> Result<PayResult, String> {
    let enc_pwd = query_payment_channel(jk, wtk, card_no, card_password).await?;
    let precalc_started = Instant::now();
    let pr = precalc_payment(jk, wtk, card_no, &enc_pwd, order_no, amount_yuan, line).await?;
    tracing::info!(
        "[JK Pay] drugCardPreCalc elapsed_ms={}",
        precalc_started.elapsed().as_millis()
    );

    if pr.get("pollReadyPlan").and_then(|v| v.as_bool()) == Some(true) {
        poll_ready_plan_if_needed(jk, wtk, order_no).await?;
    }

    let pay_started = Instant::now();
    let pay_data = execute_payment(jk, wtk, order_no, amount_yuan, line).await?;
    tracing::info!(
        "[JK Pay] drugCardPay elapsed_ms={}",
        pay_started.elapsed().as_millis()
    );
    let r = pay_data
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .cloned()
        .unwrap_or(json!({}));
    let pay_success = r.get("success").and_then(|v| v.as_bool()) == Some(true)
        && r.get("deductAmount")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            > 0.0;

    if pay_success {
        // totalAmount from API is in 元; convert to 分 for storage
        let total_amount_yuan = r.get("totalAmount").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let paid_amount_fen = (total_amount_yuan * 100.0).round() as i64;

        Ok(PayResult {
            success: true,
            paid_amount: paid_amount_fen,
            order_status: None,
            external_order_no: Some(order_no.to_string()),
            fail_reason: None,
        })
    } else {
        let msg = r
            .get("returnMsg")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                pay_data
                    .pointer("/stat/stateList")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "支付失败".to_string());

        Err(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    /// 端到端测试：登录 + 健康卡支付 0.01 元
    /// 运行：cargo test test_jk_pay_001 -- --nocapture --ignored
    #[tokio::test]
    #[ignore]
    async fn test_jk_pay_001() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .try_init();

        let redis_host = env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let redis_port = env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
        let redis_username = env::var("REDIS_USERNAME").unwrap_or_else(|_| "default".to_string());
        let redis_password = env::var("REDIS_PASSWORD").unwrap_or_default();
        let redis_url = if redis_password.is_empty() {
            if redis_username == "default" {
                format!("redis://{}:{}/0", redis_host, redis_port)
            } else {
                format!(
                    "redis://{}@{}:{}/0",
                    urlencoding::encode(&redis_username),
                    redis_host,
                    redis_port
                )
            }
        } else {
            format!(
                "redis://{}:{}@{}:{}/0",
                urlencoding::encode(&redis_username),
                urlencoding::encode(&redis_password),
                redis_host,
                redis_port
            )
        };

        let client = redis::Client::open(redis_url).expect("redis client");
        let mut redis = redis::aio::ConnectionManager::new(client)
            .await
            .expect("redis connection");

        // 测试数据来自 jk_order.py 注释
        let result = jk_pay(
            &mut redis,
            &env::var("JK_TEST_SELLER_USERNAME")
                .or_else(|_| env::var("JK_SELLER_USERNAME"))
                .expect("JK_TEST_SELLER_USERNAME or JK_SELLER_USERNAME must be set"),
            &env::var("JK_TEST_SELLER_PASSWORD")
                .or_else(|_| env::var("JK_SELLER_PASSWORD"))
                .expect("JK_TEST_SELLER_PASSWORD or JK_SELLER_PASSWORD must be set"),
            "310115199011060935",
            "093538",
            1, // 1 分 = 0.01 元；内部会 ÷0.95 换算
        )
        .await;

        println!("[test] success={}", result.success);
        println!("[test] paid_amount={} 分", result.paid_amount);
        println!("[test] order_status={:?}", result.order_status);
        println!("[test] external_order_no={:?}", result.external_order_no);
        println!("[test] fail_reason={:?}", result.fail_reason);

        assert!(result.success, "支付失败: {:?}", result.fail_reason);
        assert!(result.paid_amount > 0, "实付金额应大于 0");
        assert!(result.external_order_no.is_some(), "应返回 jk.cn 订单号");
    }
}
