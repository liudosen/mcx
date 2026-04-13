# Rust Rules - Welfare Store Backend

## Rust Idioms

### Error Handling
```rust
// Good: Use ? operator with AppError
async fn handler() -> Result<Json<Response>, AppError> {
    let result = database_call().await?;
    Ok(Json(Response::success(result)))
}

// Good: Convert errors at boundary
Err(e) => Err(AppError::DatabaseError(e))

// Bad: unwrap in async code
let result = database_call().await.unwrap();
```

### Async Patterns
```rust
// Good: Async handler with State extraction
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Request>,
) -> Result<Json<Response>, AppError> {
    // implementation
}

// Good: Shared state via Arc
let app_state = Arc::new(AppState { ... });
```

### Ownership
- Avoid unnecessary clones
- Use references when data doesn't need ownership
- Use `Arc<T>` for shared ownership across async tasks
- Use `&T` for read-only references

## Project Conventions

### Module Structure
```rust
// In lib.rs or main.rs
mod module_name;

// Public API in lib.rs
pub use routes::{auth, product};
pub mod error;
pub mod models;
pub mod state;
```

### Route Handlers
```rust
// Standard handler signature
pub async fn handler(
    State(state): State<Arc<AppState>>,
    // Other extractors...
) -> Result<Json<ApiResponse<T>>, AppError> {
    // 1. Extract and validate input
    // 2. Business logic
    // 3. Return success or error
}
```

### Database Queries (sqlx)
```rust
// Good: Type-safe query with FromRow
let result = sqlx::query_as::<_, Model>(
    "SELECT * FROM table WHERE id = ?"
)
.bind(id)
.fetch_optional(&state.db)
.await?
.ok_or(AppError::NotFound("Item".to_string()))?;
```

### Logging
```rust
// Info for normal operations
tracing::info!("User logged in: {}", username);

// Warn for recoverable issues
tracing::warn!("Invalid attempt for user: {}", username);

// Error for failures (includes error details)
tracing::error!("Database error: {}", err);
```

## Clippy Rules
- `cargo clippy -- -D warnings` should pass
- Common warnings to avoid:
  - `clippy::unwrap_used`
  - `clippy::expect_used`
  - `clippy::todo`
  - `clippy::dbg_macro`

## Testing Conventions
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_feature() {
        // Arrange
        let input = test_input();

        // Act
        let result = feature_function(input).await;

        // Assert
        assert!(result.is_ok());
    }
}
```
