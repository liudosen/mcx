# Rust Backend Context

## System Prompt Injection
This context is automatically loaded when working on the welfare-store backend.

## Current Project State
- **Type**: REST API backend
- **Status**: Functional with auth and product endpoints
- **Database**: MySQL with auto-migration

## Active Patterns

### Route Handler Pattern
```rust
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Request>,
) -> Result<Json<ApiResponse<T>>, AppError> {
    // Business logic
}
```

### Error Conversion
All errors should be converted to `AppError` before returning.

### Authentication Flow
1. Extract Bearer token from Authorization header
2. Validate JWT signature and expiration
3. Extract claims (admin_id, role)
4. Check role permissions for protected endpoints

## Available Skills
- `tdd-workflow` — For new feature development
- `verification-loop` — For code quality checks
- `rust-patterns` — For API patterns and best practices

## Conventions
- Use `ApiResponse::success(data)` for successful responses
- Return appropriate `AppError` variants for errors
- Log with `tracing::info!`, `tracing::warn!`, `tracing::error!`
- Never log sensitive data (passwords, tokens)
