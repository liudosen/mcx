# TodoWrite Command - 任务规划系统

## 格言
*没有计划的 agent 走哪算哪*

## 核心概念

TodoWrite 是 Harness 提供给 Agent 的"记事本"——让 Agent 能够：
1. 规划多步骤任务
2. 跟踪进度
3. 在长对话中保持目标不丢失

## 文件位置
- 任务文件: `.claude/tasks/TODO.md`
- 任务元数据: `.claude/tasks/meta.json`

## 任务格式

```markdown
# Task Board

## Active Tasks

### [ ] TASK-001: 任务标题
- **Created**: 2024-01-01T10:00:00Z
- **Priority**: high|medium|low
- **Status**: pending|in_progress|blocked|completed
- **Dependencies**: [TASK-000]
- **Agent**: main|subagent-name
- **Description**:
  任务详细描述

### [ ] TASK-002: 另一个任务
- **Created**: 2024-01-01T11:00:00Z
- **Priority**: medium
- **Status**: pending
- **Description**:
  描述
```

## Agent 操作接口

### 添加任务
```bash
# 创建新任务
echo "### [ ] TASK-003: 新任务
- **Created**: $(date -u +%Y-%m-%dT%H:%M:%SZ)
- **Priority**: medium
- **Status**: pending
- **Description**:
  任务描述" >> .claude/tasks/TODO.md
```

### 更新任务状态
```bash
# 更新状态 (pending -> in_progress -> completed)
# 使用 sed 替换状态行
```

### 列出任务
```bash
# 显示所有任务
cat .claude/tasks/TODO.md

# 只显示活跃任务
grep -A 10 "## Active Tasks" .claude/tasks/TODO.md
```

### 标记完成
```bash
# 将 [ ] 改为 [x]
sed -i 's/\[ \] TASK-001/[x] TASK-001/' .claude/tasks/TODO.md
```

## 使用场景

### 场景 1: 新功能开发
```
用户: 添加用户注册功能

Agent 思考: 这是一个多步骤任务，需要规划
1. 创建数据库迁移
2. 添加 model
3. 添加 route handler
4. 添加验证
5. 添加测试
6. 更新文档

Agent 执行:
- 创建 TASK-001 到 TASK-006
- 开始处理 TASK-001
```

### 场景 2: 长对话中的目标保持
```
用户: (3小时后) 继续刚才的工作

Agent 读取:
- CLAUDE.md (项目背景)
- .claude/tasks/TODO.md (当前任务)
- 继续从上次中断的地方开始
```

### 场景 3: 子任务分解
```
TASK-001: 实现产品 CRUD
  ├── TASK-001-1: 创建 Product model
  ├── TASK-001-2: 添加 list endpoint
  ├── TASK-001-3: 添加 create endpoint
  ├── TASK-001-4: 添加 update endpoint
  └── TASK-001-5: 添加 delete endpoint
```

## 集成到 Agent Loop

在处理用户请求时：

1. **检测多步骤任务** → 自动创建 TODO
2. **单步骤任务** → 直接执行
3. **长任务完成** → 更新 TODO 状态
4. **新请求** → 检查 TODO，避免重复工作

## 优先级规则

| 优先级 | 使用场景 |
|--------|----------|
| high | 阻塞其他任务、紧急 bug、发布 blocker |
| medium | 正常功能开发 |
| low | 优化、文档、重构 |

## 状态流转

```
pending → in_progress → completed
   ↑           ↓
   └── blocked ←┘
```

## 注意事项

- 每个任务必须有唯一 ID (TASK-XXX)
- 完成后保留任务记录（便于追踪）
- 高优先级任务优先处理
- 阻塞任务需要记录原因
