# Security Rules - Welfare Store Backend

## Authentication Security

### JWT Handling
- JWT secret MUST come from environment variable, never hardcoded
- Token expiration should be reasonable (default: 24 hours)
- Always validate token signature and claims
- Refresh tokens should invalidate old tokens

### Password Security
- Hash passwords with bcrypt (cost: 10-12 in production)
- Never log or expose password hashes
- Never accept plaintext passwords over non-HTTPS
- Verify password complexity requirements

### Credential Handling
```
// Environment variables for secrets
JWT_SECRET=...
ADMIN_PASSWORD=...
DATABASE_URL=...
```

## Authorization Security

### Role Checks
- Admin: Full access to all resources
- Operator: Manage products, orders, inventory, logistics
- Viewer: Read-only access

### Permission Enforcement
```rust
// Check role before sensitive operations
if claims.role != "admin" {
    return Err(AppError::PermissionDenied);
}
```

## API Security

### Input Validation
- Validate all request parameters
- Use type-safe deserialization with serde
- Check string lengths and formats
- Sanitize file paths if handling uploads

### SQL Injection Prevention
```rust
// GOOD: Parameterized query
sqlx::query("SELECT * FROM users WHERE username = ?")
    .bind(&username)
    .fetch_one(&pool)
    .await?

// BAD: String concatenation - NEVER DO THIS
let query = format!("SELECT * FROM users WHERE username = '{}'", username);
```

### CORS Configuration
- Configure allowed origins explicitly
- Limit allowed methods and headers
- Don't use `allow_origin(Any)` in production

## Data Protection

### Logging Rules
- NEVER log: passwords, tokens, personal data
- DO log: user IDs (not emails), operation types, timestamps
- Sanitize error messages before logging

### Error Messages
```rust
// Good: Generic message doesn't leak internals
Err(AppError::InvalidCredentials)

// Bad: Reveals which field is invalid
Err(AppError::BadRequest("Invalid password format".to_string()))
```

## Dependencies

### Dependency Security
- Run `cargo audit` regularly to check for vulnerabilities
- Keep dependencies updated
- Review new dependencies before adding

### Allowed Crates
This project uses these approved crates:
- axum, tokio, tower - Web framework
- sqlx - Database (parameterized queries prevent SQL injection)
- jsonwebtoken - JWT handling
- bcrypt - Password hashing
- serde - Serialization
- tracing - Logging
