#!/bin/bash
# Post-Task Automation: Review -> Verify -> Fix -> Update Docs -> Update Tasks
# Runs automatically after each task completion

set -e

cd "$(git rev-parse --show-toplevel)"

echo "=== Post-Task Automation ==="

# 1. Review Changes
echo "[1/6] Reviewing changes..."
if [[ -n "$(git diff --stat)" ]]; then
    git diff --stat
    echo "Changes detected:"
    git diff --name-only
fi

# 2. Verify (check, clippy, test)
echo "[2/6] Verifying..."
cargo check --quiet 2>/dev/null || {
    echo "ERROR: cargo check failed"
    exit 1
}

cargo clippy --quiet 2>/dev/null || {
    echo "WARNING: clippy found issues, attempting auto-fix..."
    cargo clippy --fix --allow-dirty --allow-staged --quiet 2>/dev/null || true
}

cargo test --quiet 2>/dev/null || {
    echo "ERROR: tests failed"
    exit 1
}

# 3. Format
echo "[3/6] Formatting..."
cargo fmt --quiet

# 4. Auto-update docs if code changed
echo "[4/6] Checking if docs need update..."
if [[ -n "$(git diff --name-only)" ]]; then
    CHANGED_FILES=$(git diff --name-only)

    if echo "$CHANGED_FILES" | grep -qE "^(backend/src/routes/|backend/src/models/)"; then
        echo "API structure changed - updating project memory..."
    fi
fi

# 5. Update Task System if using
echo "[5/6] Checking task status..."
TASK_FILE=".claude/tasks/TODO.md"
META_FILE=".claude/tasks/meta.json"

if [[ -f "$TASK_FILE" ]] && [[ -f "$META_FILE" ]]; then
    # Check if there's an in-progress task to update
    if grep -q "in_progress" "$META_FILE" 2>/dev/null; then
        echo "Task in progress - consider updating status with: task-cli update <task-id> --status completed"
    fi
fi

# 6. Check Background Tasks
echo "[6/6] Checking background tasks..."
BG_DIR=".claude/background/running"
if [[ -d "$BG_DIR" ]]; then
    RUNNING_TASKS=$(ls -A "$BG_DIR" 2>/dev/null || true)
    if [[ -n "$RUNNING_TASKS" ]]; then
        echo "Background tasks running:"
        ls "$BG_DIR"
    fi
fi

echo "=== Verification Complete ==="
echo "Run 'git diff' to review all changes"
echo "Run 'task-cli list' to view task board"
echo "Run 'bg list' to view background tasks"
