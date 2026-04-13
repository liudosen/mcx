# Automatic Post-Task Verification

## Trigger
After completing ANY code change (editing files, creating functions, fixing bugs, etc.)

## Automatic Execution Sequence

### 1. Self-Review
Review the changes you just made:
- Logic correctness
- Error handling
- Security implications
- Follows project patterns

### 2. Verification Loop
Run in sequence until all pass:
```bash
cargo check      # 1. Type check
cargo clippy -- -D warnings  # 2. Lint
cargo test       # 3. Tests
cargo fmt       # 4. Format
```

### 3. Auto-Fix if Issues Found
If clippy or tests fail:
1. Read the error messages
2. Fix the issues automatically
3. Re-run verification
4. Repeat until clean

### 4. Documentation Update
If code changed:
- Updated function signatures → Update doc comments
- New API endpoints → Update CLAUDE.md API section
- New patterns → Update relevant skill files
- Breaking changes → Update CHANGELOG

## Success Criteria
All commands pass with zero errors:
```
cargo check ✓
cargo clippy ✓
cargo test ✓
cargo fmt ✓
```

## On Failure
- Do NOT report "task complete" until fixed
- Fix the issues first
- Re-verify
- Only then report completion

## Exception
Skip for:
- Pure documentation changes (READMEs, comments)
- Configuration-only changes
- Debugging/troubleshooting sessions
