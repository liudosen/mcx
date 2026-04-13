# Rust Backend Patterns Skill

## Description
Common patterns for building REST APIs with Axum and SQLx in this project.

## Project Architecture

### Layered Structure
```
routes/     # HTTP handlers, request/response types
services/   # Business logic (future)
models/     # Data structures, database types
state.rs    # Application state (DB pool, config)
error.rs    # Unified error handling
```

## Common Patterns

### Handler Pattern
```rust
pub async fn endpoint(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RequestType>,
) -> Result<Json<ApiResponse<ResponseType>>, AppError> {
    // 1. Validate input
    if payload.name.is_empty() {
        return Err(AppError::BadRequest("Name required".to_string()));
    }

    // 2. Execute database operation
    let result = sqlx::query_as::<_, Model>(
        "INSERT INTO ... VALUES (...)"
    )
    .bind(&payload.name)
    .execute(&state.db)
    .await?
    .ok_or(AppError::InternalError("Insert failed".to_string()))?;

    // 3. Return success response
    Ok(Json(ApiResponse::success(result)))
}
```

### Response Pattern
```rust
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub code: u16,
    pub data: Option<T>,
    pub message: String,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 200,
            data: Some(data),
            message: "success".to_string(),
        }
    }
}
```

### Error Handling Pattern
```rust
// Convert sqlx errors to AppError
match result {
    Ok(val) => Ok(val),
    Err(sqlx::Error::RowNotFound) => Err(AppError::NotFound(id)),
    Err(e) => {
        tracing::error!("Database error: {}", e);
        Err(AppError::DatabaseError(e))
    }
}
```

### Authentication Pattern
```rust
pub async fn protected_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<T>>, AppError> {
    // Extract token
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::InvalidToken)?;

    let token = extract_token(auth_header).ok_or(AppError::InvalidToken)?;

    // Validate token
    let claims = validate_token(&state, token)?;

    // Check role if needed
    if claims.role != "admin" {
        return Err(AppError::PermissionDenied);
    }

    // Proceed with handler logic
}
```

### Database Transaction Pattern
```rust
// For operations needing atomicity
let mut tx = pool.begin().await?;

sqlx::query("INSERT INTO orders ...")
    .execute(&mut tx)
    .await?;

sqlx::query("UPDATE inventory ...")
    .execute(&mut tx)
    .await?;

tx.commit().await?;
```

## Best Practices

1. **Use typed queries** - `sqlx::query_as::<_, T>` for type safety
2. **Extract to services** - Keep handlers thin, logic in services
3. **Use constants** - Define repeated values as constants
4. **Log appropriately** - Info for success, error for failures
5. **Validate early** - Fail fast on invalid input
