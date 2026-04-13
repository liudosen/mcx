# Code Review Command

## Description
Perform a comprehensive code review of changes.

## When to Use
- Before committing changes
- After implementing a feature
- When requested by team member

## Review Checklist

### Correctness
- [ ] Logic is correct for the intended purpose
- [ ] Edge cases are handled
- [ ] Error cases return appropriate errors
- [ ] No unwrap/expect without good reason

### Security
- [ ] No sensitive data logged
- [ ] Input validation present
- [ ] SQL injection prevented (use parameterized queries)
- [ ] JWT token handling is secure
- [ ] Password handling follows best practices

### Rust Best Practices
- [ ] Uses `?` for error propagation
- [ ] Async functions return `Result<T, AppError>`
- [ ] Appropriate use of `Arc<>` for shared state
- [ ] No unnecessary clones
- [ ] Clippy warnings addressed

### API Design
- [ ] REST conventions followed
- [ ] Proper HTTP status codes
- [ ] Consistent response format
- [ ] Proper error messages

### Testing
- [ ] Unit tests for business logic
- [ ] Tests cover happy path and errors
- [ ] Tests are maintainable

## Usage
`/review` - Review all unstaged changes
`/review <files>` - Review specific files
