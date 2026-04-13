---
name: claude-code-commands
description: Claude Code CLI commands reference for harness engineering
type: reference
---

# Claude Code Commands Reference

## Session Commands
- `/help` — Get help with Claude Code
- `/exit` — End session
- `/compact` — Trigger context compaction

## Slash Commands (Project-specific)
- `/plan` — Create implementation plan
- `/tdd` — Test-driven development workflow
- `/review` — Code review
- `/security` — Security review

## Agent Commands
- `/agent <type>` — Spawn specialized sub-agent
  - Types: planner, architect, code-reviewer, security-reviewer

## Context Management
- Context comfort threshold: 70k tokens
- Emergency threshold: 50k tokens
- Compact triggers automatically when approaching limits

## Hooks
- `session-start` — Load project context
- `session-end` — Save session memory
- `pre-commit` — Run verification checks

## Verification Commands
```bash
cargo check      # Type checking
cargo clippy     # Lint
cargo test       # Run tests
cargo fmt        # Format
```
