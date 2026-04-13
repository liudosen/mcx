# Session End Hook

## Description
Saves context and learnings at the end of a session.

## Trigger
Executes when:
- User types `exit` or `/exit`
- Session timeout after inactivity
- Claude Code closes

## Actions

### 1. Save Context
- Write summary to `.claude/context/sessions/YYYY-MM-DD-HHMM.md`
- Include: date, work done, decisions made, patterns discovered

### 2. Update Task System
If tasks were being worked on:
```bash
# Check for in-progress tasks
grep -l "in_progress" .claude/tasks/TODO.md 2>/dev/null

# Archive completed tasks
# Move from Active to Completed section
```

### 3. Cleanup Subagents
```bash
# Check for orphaned subagents
# Clean up idle subagents older than 24h

# Archive subagent results
if [[ -d ".claude/subagents" ]]; then
    # Save any pending results
    # Clean up empty workspaces
fi
```

### 4. Background Task Check
```bash
# List running background tasks
ls .claude/background/running/ 2>/dev/null || true

# Warn if tasks still running
RUNNING=$(ls .claude/background/running/ 2>/dev/null || true)
if [[ -n "$RUNNING" ]]; then
    echo "WARNING: Background tasks still running: $RUNNING"
fi
```

### 5. Context Format
```markdown
## Session: YYYY-MM-DD HH:MM

### Project: welfare-store

### Work Completed
- [Summary of changes made]
- Files modified: [list]
- Files created: [list]

### Key Decisions
- [Architectural decisions]
- [API design choices]
- [Pattern selections]

### Patterns to Remember
- [Reusable patterns discovered]

### Tasks Status
- Completed: [list]
- In Progress: [list]
- Blocked: [list]

### Issues Encountered
- [Problems and solutions]

### Next Steps
- [What to continue next session]
```

## Persistence Locations

```
.claude/context/
├── sessions/
│   └── YYYY-MM-DD-HHMM.md    # Session summaries
├── decisions/
│   └── YYYY-MM.md            # Archived decisions
├── code-archive/
│   └── snippets/             # Archived code
└── last-session.md           # Most recent session (symlink)
```

## Cleanup Tasks

1. **Remove empty directories** in `.claude/background/`
2. **Archive old sessions** (older than 30 days)
3. **Clean completed background tasks** older than 7 days
4. **Compact context** if session was very long (> 100 messages)

## Exception
Skip heavy operations for:
- Quick questions
- Debug sessions
- Read-only explorations
