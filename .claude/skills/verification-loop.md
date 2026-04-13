# Verification Loop Skill

## Description
Continuous verification workflow for ensuring code quality after every change.

## When to Use
- After every code change
- Before committing
- After implementing a feature
- During code review

## Verification Steps

### 1. Type Check
```bash
cd backend && cargo check
```
Ensures no type errors, import issues, or compilation problems.

### 2. Lint
```bash
cd backend && cargo clippy -- -D warnings
```
Checks for common mistakes, unused code, and Rust idioms.

### 3. Tests
```bash
cd backend && cargo test
```
Runs all unit and integration tests.

### 4. Format Check
```bash
cd backend && cargo fmt --check
```
Ensures consistent code formatting.

### 5. Security Audit
```bash
cd backend && cargo audit
```
Checks for known vulnerabilities in dependencies.

## Full Verification Sequence
```bash
cd backend && cargo fmt && cargo check && cargo clippy -- -D warnings && cargo test
```

## Project-Specific Verification

### For API Changes
1. Start the server: `cd backend && cargo run`
2. Test endpoint: `curl http://localhost:8081/health`
3. Check logs for errors

### For Database Changes
1. Verify migration syntax
2. Test rollback scenario
3. Check data integrity

## Pre-Commit Checklist
- [ ] `cd backend && cargo check` passes
- [ ] `cd backend && cargo clippy` passes
- [ ] `cd backend && cargo test` passes
- [ ] No `TODO` or `FIXME` left in code
- [ ] No debug prints (`println!`, `dbg!`)
- [ ] No hardcoded secrets
- [ ] Tests cover new functionality

## Continuous Integration
For CI/CD, run:
```bash
cd backend
cargo fmt --check || exit 1
cargo check --all-targets || exit 1
cargo clippy -- -D warnings || exit 1
cargo test --all-targets || exit 1
cargo audit || exit 1
```
