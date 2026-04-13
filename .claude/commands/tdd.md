# TDD Command - Test Driven Development

## Description
Execute a TDD workflow: Red -> Green -> Refactor

## When to Use
- Adding new business logic
- Complex algorithm implementations
- Database query logic
- Authentication/authorization logic

## TDD Cycle

### 1. Red - Write Failing Test
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_feature_behavior() {
        // Write test that describes expected behavior
        // Run: cargo test -- --nocapture
        // Verify it fails with clear error
    }
}
```

### 2. Green - Write Minimal Code
- Write only enough code to make the test pass
- Don't optimize, don't add features
- Focus on correctness

### 3. Refactor - Improve Code
- Remove duplication
- Extract functions/modules
- Apply Rust idioms
- Ensure all tests still pass

## Verification
After TDD cycle:
1. `cargo test`
2. `cargo clippy -- -D warnings`
3. Review the diff

## Project-Specific Notes
- Use `sqlx::query_as` for typed database queries
- Use `AppError` for all error types
- Mock database in unit tests using sqlx test helpers
