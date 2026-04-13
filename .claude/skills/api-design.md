# API Design Skill

## 何时使用
- 添加新的 API endpoint
- 设计 REST 资源结构
- 定义 request/response 格式
- 修改现有 API

## Welfare Store API 规范

### Response Format

```rust
#[derive(Serialize)]
struct ApiResponse<T> {
    code: u16,      // 200=成功, 400=客户端错误, 500=服务器错误
    data: Option<T>,
    message: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 200,
            data: Some(data),
            message: "success".to_string(),
        }
    }

    pub fn error(code: u16, message: impl Into<String>) -> Self {
        Self {
            code,
            data: None,
            message: message.into(),
        }
    }
}
```

### Error Codes

| Code | Meaning |
|------|---------|
| 200 | Success |
| 400 | Bad Request - Invalid input |
| 401 | Unauthorized - Missing/invalid token |
| 403 | Forbidden - Insufficient permissions |
| 404 | Not Found - Resource doesn't exist |
| 409 | Conflict - Duplicate resource |
| 500 | Internal Server Error |

### REST Endpoints Pattern

```
GET     /api/products          # List all products
GET     /api/products/:id      # Get single product
POST    /api/products          # Create product
PUT     /api/products/:id      # Update product
DELETE  /api/products/:id      # Delete product
```

### Handler Signature Pattern

```rust
// Good: Async handler with State extraction
pub async fn get_product(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<Product>>, AppError> {
    let product = ProductService::find_by_id(&state.db, id)
        .await?
        .ok_or(AppError::NotFound("Product".to_string()))?;

    Ok(Json(ApiResponse::success(product)))
}
```

### Request Validation Pattern

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub price: i32,  // cents
    #[serde(default)]
    pub image_urls: Vec<String>,
    #[serde(default)]
    pub category: Option<String>,
}

// Validate in handler
impl CreateProductRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.name.is_empty() {
            return Err(AppError::BadRequest("name is required".to_string()));
        }
        if self.name.len() > 255 {
            return Err(AppError::BadRequest("name too long".to_string()));
        }
        if self.price < 0 {
            return Err(AppError::BadRequest("price must be positive".to_string()));
        }
        Ok(())
    }
}
```

### Route Registration Pattern

```rust
// In main.rs or router module
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/products", post(create_product))
        .route("/api/products/:id", get(get_product))
        .with_state(state)
}
```

### JWT Auth Middleware Pattern

```rust
use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;

    let claims = state.jwt.verify(token)?;
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}
```

## 注意事项

1. **使用 Json<T> extractor**: 自动序列化/反序列化
2. **统一错误处理**: 使用 `?` 操作符和 AppError
3. **验证输入**: 在 handler 或 service 层验证
4. **日志敏感数据**: 不要 log 密码、token 等
5. **幂等性**: DELETE 应该 idempotent
