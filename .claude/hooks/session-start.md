# Session Start Hook

## Description
Sets up context at the beginning of a Claude Code session.

## Trigger
Executes when starting a new Claude Code session.

## Actions

### 1. Load Project Context
- Read `CLAUDE.md` for project overview
- Load `.claude/rules/*.md` for coding standards
- Load relevant `.claude/skills/*.md` based on detected work

### 2. Detect Work Type
Based on what the user asks to do:
- **New feature** → Load TDD workflow skill
- **Bug fix** → Load verification loop skill
- **Security issue** → Load security rules
- **API change** → Load Rust backend patterns skill
- **Long task** → Load task system (s07)
- **Background work** → Load background tasks (s08)

### 3. Environment Check
```bash
# Verify Rust toolchain
rustc --version
cargo --version

# Verify backend builds (run from backend directory)
cd backend && cargo check 2>/dev/null || echo "Backend has build errors"
```

### 4. Check Task System
```bash
# If returning user, check for in-progress tasks
if [[ -f ".claude/tasks/TODO.md" ]]; then
    echo "=== Current Tasks ==="
    cat .claude/tasks/TODO.md
fi

# Check subagent status
if [[ -f ".claude/subagents/_registry.json" ]]; then
    echo "=== Subagent Status ==="
    cat .claude/subagents/_registry.json
fi

# Check background tasks
if [[ -d ".claude/background/running" ]]; then
    RUNNING=$(ls .claude/background/running 2>/dev/null || true)
    if [[ -n "$RUNNING" ]]; then
        echo "=== Background Tasks Running ==="
        echo "$RUNNING"
    fi
fi
```

### 5. Context Recovery
If previous session had uncompleted tasks:
```bash
# Load last session context
cat .claude/context/last-session.md 2>/dev/null || echo "No previous session"

# Check for pending background tasks
ls .claude/background/completed/ 2>/dev/null || true
```

### 6. Display Session Info
```
Project: welfare-store
Type: Full-stack (Vue frontend + Rust/Axum backend)
Tech Stack: Rust/Axum, Vue 2, MySQL, JWT

Available Harness Systems:
- TodoWrite: /todo command
- Task System: /tasks command
- Background Tasks: /background command
- Subagent: Use 'subagent create/assign/list' commands
- Skills: Use 'skill-load' command

Context: [relevant skills based on detected work]
```

## Session Memory
At session end, key decisions and patterns learned are saved to:
`.claude/context/sessions/YYYY-MM-DD-HHMM.md`

This enables continuous learning across sessions.
