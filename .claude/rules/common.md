# Common Rules - All Languages

## Core Principles

### Security-First
- Never commit secrets, API keys, or credentials
- Use environment variables for all sensitive config
- Never log passwords, tokens, or personal data
- Validate all user input before processing

### Error Handling
- Always handle errors explicitly with `?` or `match`
- Never use `unwrap()` in production code unless absolutely certain
- Use `expect()` only with clear panic messages for truly unrecoverable states
- Return user-friendly error messages without leaking internals

### Code Quality
- Write self-documenting code with clear names
- Keep functions small and focused (single responsibility)
- DRY - Don't Repeat Yourself
- YAGNI - You Aren't Gonna Need It

### Testing
- Test business logic thoroughly
- Aim for meaningful test coverage, not 100% vanity metrics
- Unit tests should be fast and isolated
- Integration tests verify real behavior

## Workflow Rules

### Before Writing Code
1. Understand the full requirement
2. Plan the implementation
3. Consider the API contract first

### After Writing Code
1. Run tests: `cargo test`
2. Run linter: `cargo clippy -- -D warnings`
3. Check types: `cargo check`
4. Review your diff for unintended changes

### Before Committing
1. Verify all tests pass
2. No clippy warnings
3. Code follows project conventions
4. No debug code left behind
