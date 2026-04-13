use crate::error::AppError;
use crate::routes::admin::auth::{check_token_exists, extract_token, validate_token};
use crate::routes::ApiResponse;
use crate::services::oss::UploadSignature;
use crate::state::AppState;
use axum::{extract::State, http::HeaderMap, Json};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct SignatureRequest {
    pub filename: String,
}

/// GET /api/admin/upload/signature
/// 获取 OSS 上传签名（前端直传用）
pub async fn get_upload_signature(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<SignatureRequest>,
) -> Result<Json<ApiResponse<UploadSignature>>, AppError> {
    // 验证管理员身份
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::InvalidToken)?;

    let token = extract_token(auth_header).ok_or(AppError::InvalidToken)?;

    if !check_token_exists(&state, token).await? {
        return Err(AppError::TokenExpired);
    }

    let _claims = validate_token(&state, token)?;

    // 创建 OSS 服务实例
    let oss_service = crate::services::oss::OssService::new(
        state.oss_endpoint.clone(),
        state.oss_access_key_id.clone(),
        state.oss_access_key_secret.clone(),
        state.oss_bucket.clone(),
        state.oss_domain.clone(),
    );

    // 生成上传签名
    let signature = oss_service.generate_upload_signature(&params.filename);

    tracing::info!("Generated upload signature for file: {}", params.filename);

    Ok(Json(ApiResponse::success(signature)))
}
