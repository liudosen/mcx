# Task System - 任务持久化系统

## 格言
*大目标要拆成小任务, 排好序, 记在磁盘上*

## 核心理念

任务系统 = TodoWrite + 持久化 + 依赖图 + 多 Agent 支持

## 目录结构

```
.claude/tasks/
├── TODO.md              # 任务看板 (Markdown)
├── meta.json            # 任务元数据 (依赖图, 状态)
├── backlog.md           # 待办池
├── done.md              # 已完成任务归档
└── archive/            # 历史归档
    ├── 2024-01.md
    └── 2024-02.md
```

## meta.json 结构

```json
{
  "version": "1.0",
  "last_updated": "2024-01-01T10:00:00Z",
  "tasks": {
    "TASK-001": {
      "id": "TASK-001",
      "title": "实现产品 CRUD",
      "status": "in_progress",
      "priority": "high",
      "created": "2024-01-01T10:00:00Z",
      "updated": "2024-01-01T12:00:00Z",
      "dependencies": [],
      "agent": "main",
      "subtasks": ["TASK-001-1", "TASK-001-2"],
      "blockers": []
    }
  },
  "agents": {
    "main": {
      "current_task": "TASK-001",
      "last_active": "2024-01-01T12:00:00Z"
    }
  }
}
```

## 任务状态机

```
                    ┌──────────┐
                    │ blocked  │
                    └────┬─────┘
                         │ (dependencies met)
                         ▼
┌──────────┐     ┌──────────┐     ┌────────────┐
│ pending  │────▶│in_progress│────▶│ completed  │
└──────────┘     └────┬─────┘     └────────────┘
     ▲                 │
     │                 │ (blocked by dependency)
     └─────────────────┘
```

## 依赖图

```
TASK-001: 实现产品 CRUD
├── TASK-001-1: 创建 Product model    [依赖: 无]
├── TASK-001-2: 添加 list endpoint    [依赖: TASK-001-1]
├── TASK-001-3: 添加 create endpoint  [依赖: TASK-001-1]
├── TASK-001-4: 添加 update endpoint  [依赖: TASK-001-1, TASK-001-3]
└── TASK-001-5: 添加 delete endpoint  [依赖: TASK-001-1]

执行顺序:
1. TASK-001-1 (可以并行)
2. TASK-001-2, TASK-001-3 (可以并行, 依赖 1)
3. TASK-001-4 (依赖 1, 3)
4. TASK-001-5 (依赖 1)
```

## CLI 工具

```bash
# 创建任务
task-cli create "新任务标题" --priority high --agent main

# 更新状态
task-cli update TASK-001 --status in_progress
task-cli update TASK-001 --status completed

# 添加依赖
task-cli depends TASK-002 --on TASK-001

# 列出任务
task-cli list                    # 所有任务
task-cli list --status pending   # 按状态筛选
task-cli list --agent main       # 按 agent 筛选

# 查看可执行任务 (依赖已满足)
task-cli next

# 阻塞检查
task-cli blocked                 # 显示被阻塞的任务及原因
```

## Agent 协作

### Main Agent
- 管理任务系统
- 分解大任务
- 分配子任务给 subagents

### Subagents
- 只看到自己的任务
- 不能修改其他 agent 的任务
- 完成后通知 main agent

## 任务分配流程

```
1. Main Agent: 接收用户请求
2. Main Agent: 创建 TASK-001, TASK-002, ...
3. Main Agent: 设置依赖关系
4. Subagent A: 认领 TASK-001 (task-cli claim TASK-001)
5. Subagent B: 认领 TASK-002
6. 各 Subagent: 执行并更新状态
7. Main Agent: 监控进度，协调
```

## 与 TodoWrite 的区别

| 特性 | TodoWrite | Task System |
|------|-----------|-------------|
| 持久化 | 会话级 | 磁盘持久 |
| 依赖 | 无 | 有向无环图 |
| 多 Agent | 不支持 | 支持 |
| 状态历史 | 无 | 有 |
| 调度 | 手动 | 自动计算可执行任务 |

## 归档策略

### 自动归档
- 任务完成后移入 done.md
- 每日归档到 archive/YYYY-MM.md

### 手动归档
```bash
task-cli archive --before 2024-01-01
```

## 恢复流程

当 Agent 重启或新会话开始时：

```bash
# 1. 读取 TODO.md
# 2. 读取 meta.json
# 3. 恢复状态
# 4. 继续执行
```

## 示例场景

### 场景: 实现订单系统

```
用户: 添加订单管理功能

Agent:
1. task-cli create "实现订单管理" --priority high
   → TASK-003

2. task-cli create "创建 Order model" --parent TASK-003
   → TASK-003-1

3. task-cli create "订单列表 API" --parent TASK-003
   → TASK-003-2

4. task-cli create "创建订单 API" --parent TASK-003
   → TASK-003-3

5. task-cli depends TASK-003-2 --on TASK-003-1
   task-cli depends TASK-003-3 --on TASK-003-1

6. task-cli next
   → TASK-003-1 (无依赖，可执行)

7. Subagent 认领 TASK-003-1, 完成

8. task-cli next
   → TASK-003-2, TASK-003-3 (依赖已满足)
```

## 最佳实践

1. **任务粒度**: 每个任务 15min-2h 工作量
2. **依赖明确**: 避免循环依赖
3. **状态及时更新**: 完成立即标记
4. **定期检查**: `task-cli blocked` 避免死锁
