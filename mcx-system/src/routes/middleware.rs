use axum::{
    extract::{Request, State},
    http::header::HeaderValue,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use uuid::Uuid;

fn should_skip_request_log(path: &str) -> bool {
    path == "/api/admin/logs/recent"
}

fn request_user_label(req: &Request, jwt_secret: &str) -> String {
    let path = req.uri().path();
    if path.starts_with("/api/mini") || path.starts_with("/api/goods") {
        if let Some(auth_header) = req
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
        {
            if let Some(token) = auth_header.strip_prefix("Bearer ") {
                use jsonwebtoken::{decode, DecodingKey, Validation};

                let result = decode::<crate::routes::mini_app::auth::WechatClaims>(
                    token,
                    &DecodingKey::from_secret(jwt_secret.as_bytes()),
                    &Validation::default(),
                );

                if let Ok(data) = result {
                    return data.claims.openid;
                }
            }
        }
    }

    "-".to_string()
}

pub async fn request_log_middleware(
    State(jwt_secret): State<String>,
    req: Request,
    next: Next,
) -> Response {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let user = request_user_label(&req, &jwt_secret);
    let started = Instant::now();

    if should_skip_request_log(&path) {
        let mut response = next.run(req).await;

        if let Ok(value) = HeaderValue::from_str(&request_id) {
            response.headers_mut().insert("x-request-id", value);
        }

        return response;
    }

    tracing::info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        user = %user,
        "request started"
    );
    let mut response = next.run(req).await;

    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", value);
    }

    tracing::info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        user = %user,
        status = response.status().as_u16(),
        elapsed_ms = started.elapsed().as_millis(),
        "request finished"
    );

    response
}

#[cfg(test)]
mod tests {
    use super::should_skip_request_log;

    #[test]
    fn skips_log_query_endpoint() {
        assert!(should_skip_request_log("/api/admin/logs/recent"));
    }

    #[test]
    fn keeps_other_routes_logged() {
        assert!(!should_skip_request_log("/api/admin/logs"));
        assert!(!should_skip_request_log("/api/admin/logs/recent/extra"));
    }
}
