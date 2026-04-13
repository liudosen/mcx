# TDD Workflow Skill

## Description
Test-Driven Development workflow for Rust/Axum projects.

## When to Use
- Adding new business logic
- Complex algorithm implementation
- Database query logic
- API endpoint development

## TDD Cycle

### Phase 1: Red - Write Failing Test

**Step 1**: Create test file or add to existing `#[cfg(test)]` module

**Step 2**: Write test that describes desired behavior:
```rust
#[cfg(test)]
mod product_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_product_success() {
        // Arrange
        let state = create_test_state().await;
        let request = CreateProductRequest {
            name: "Test Product".to_string(),
            description: Some("Description".to_string()),
            price: 1000,
            image_urls: vec!["http://example.com/img.jpg".to_string()],
            category: Some("Electronics".to_string()),
        };

        // Act
        let result = create_product(State(state), Json(request)).await;

        // Assert
        assert!(result.is_ok());
    }
}
```

**Step 3**: Run test, verify it fails with meaningful error

### Phase 2: Green - Write Minimal Code

**Step 1**: Write minimum code to pass the test
- Focus on correctness, not optimization
- Don't add extra features

**Step 2**: Run tests, verify they pass

### Phase 3: Refactor

**Step 1**: Improve code structure
- Remove duplication
- Extract helper functions
- Apply Rust idioms

**Step 2**: Ensure tests still pass

## Project-Specific Patterns

### Testing Route Handlers
```rust
// Use tower's Request builder for testing
use tower::ServiceExt;
use axum::{Router, body::Body};

let app = Router::new()
    .route("/api/products", post(create_product))
    .with_state(state);

let response = app
    .oneshot(Request::builder()
        .method("POST")
        .uri("/api/products")
        .json(&request)
        .unwrap())
    .await?;
```

### Testing Database Logic
```rust
// Use sqlx test database
#[sqlx::test]
async fn test_product_query(pool: MySqlPool) {
    // Insert test data
    sqlx::query("INSERT INTO products...")
        .execute(&pool)
        .await?;

    // Query and verify
    let product = sqlx::query_as::<_, Product>(
        "SELECT * FROM products WHERE name = ?"
    )
    .bind("Test")
    .fetch_one(&pool)
    .await?;

    assert_eq!(product.name, "Test");
}
```

## Verification Commands
```bash
# Run tests
cargo test -- --nocapture

# Run with coverage
cargo tarpaulin --verbose

# Run clippy
cargo clippy -- -D warnings

# Full verification
cargo check && cargo test && cargo clippy -- -D warnings
```
