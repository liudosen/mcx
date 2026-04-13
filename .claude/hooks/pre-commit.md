# Pre-Commit Hook

## Description
Runs verification before allowing git commit.

## Trigger
Executes automatically before `git commit` via git hooks.

## Checks Performed

### 1. Format Check
```bash
cargo fmt --check
```
Ensures consistent code formatting.

### 2. Type Check
```bash
cargo check --all-targets
```
Verifies no compilation errors.

### 3. Lint Check
```bash
cargo clippy -- -D warnings
```
Catches common mistakes and style issues.

### 4. Test Check
```bash
cargo test --all-targets
```
Ensures all tests pass.

## Failure Handling
If any check fails:
1. Hook prints which check failed
2. Commit is aborted
3. User must fix issues and retry

## Installation
The hook is installed automatically when cloning the repo if using the project's setup script. To install manually:
```bash
cp .claude/hooks/pre-commit.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

## Configuration
Skip pre-commit checks with:
```bash
git commit --no-verify
```
Use sparingly - only for emergency hotfixes.
