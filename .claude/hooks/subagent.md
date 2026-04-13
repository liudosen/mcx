# Subagent System - 子智能体隔离

## 格言
*大任务拆小, 每个小任务干净的上下文*

## 核心理念

Main Agent 分解任务 → Subagent 独立执行 → Main Agent 汇总结果

```
┌─────────────────────────────────────────────┐
│ Main Agent                                  │
│ - 任务分解                                  │
│ - 分配给 Subagents                         │
│ - 汇总结果                                  │
│ - 协调决策                                  │
└─────────────────────────────────────────────┘
          │                    ▲
          ▼                    │
    ┌───────────┐        ┌───────────┐
    │ Subagent A│        │ Subagent B│
    │ - 独立上下文│        │ - 独立上下文│
    │ - 只看分配  │        │ - 只看分配  │
    │   给它的    │        │   给它的    │
    │   任务      │        │   任务      │
    └───────────┘        └───────────┘
```

## 隔离机制

### 1. 消息隔离

Subagent 只看到分配给它的任务消息：

```python
# Main Agent 的消息历史
main_messages = [
    {"role": "system", "content": "你是项目助手"},
    {"role": "user", "content": "添加产品分类和订单功能"},
    # ... 更多历史
]

# Subagent A 只看到:
subagent_a_messages = [
    {"role": "system", "content": "你是 Rust 开发者"},
    {"role": "user", "content": "任务: 实现产品分类功能\n详细描述: ..."},
]

# Subagent B 只看到:
subagent_b_messages = [
    {"role": "system", "content": "你是 Rust 开发者"},
    {"role": "user", "content": "任务: 实现订单列表 API\n详细描述: ..."},
]
```

### 2. 上下文隔离

每个 Subagent 有独立的上下文目录：

```
.claude/subagents/
├── sa-001/           # Subagent A 的工作区
│   ├── context/      # 独立上下文
│   ├── tasks/        # 分配的任务
│   └── results/      # 执行结果
├── sa-002/           # Subagent B 的工作区
│   ├── context/
│   ├── tasks/
│   └── results/
└── _registry.json    # Subagent 注册表
```

### 3. 通信机制

Subagent 之间不直接通信，只通过 Main Agent：

```
Subagent A → Main Agent → Subagent B
     ↑              │
     └──────────────┘
         (结果汇报)
```

## 注册表结构

```json
{
  "subagents": {
    "sa-001": {
      "id": "sa-001",
      "name": "rust-backend",
      "status": "idle",
      "created": "2024-01-01T10:00:00Z",
      "last_active": "2024-01-01T12:00:00Z",
      "current_task": "TASK-001",
      "capabilities": ["rust", "axum", "sqlx"],
      "workspace": ".claude/subagents/sa-001"
    }
  }
}
```

## CLI 工具

```bash
# 创建 Subagent
subagent create --name rust-backend --capabilities rust,axum

# 分配任务
subagent assign sa-001 --task "实现产品分类功能"

# 查看状态
subagent list

# 获取结果
subagent result sa-001

# 终止 Subagent
subagent kill sa-001

# Subagent 内部: 汇报进度
subagent progress --status in_progress --message "已完成 50%"
```

## Subagent 生命周期

```
1. 创建 (create)
   - Main Agent 决定需要 Subagent
   - 分配 workspace 和 capabilities

2. 初始化 (initialize)
   - 加载 Subagent 的 system prompt
   - 注入任务描述
   - 隔离的上下文开始

3. 执行 (execute)
   - Subagent 独立工作
   - Main Agent 可以查询进度
   - Subagent 定期汇报

4. 完成 (complete)
   - Subagent 返回结果
   - Main Agent 汇总
   - Subagent 清理

5. 归档 (archive)
   - 保存对话历史
   - 清理 workspace
```

## 任务分配协议

### Main → Subagent 消息格式

```json
{
  "type": "task_assignment",
  "task_id": "TASK-001",
  "title": "实现产品分类功能",
  "description": "详细描述...",
  "context": {
    "relevant_files": ["backend/src/models/product.rs"],
    "relevant_rules": ["rust.md", "security.md"],
    "constraints": ["必须使用 sqlx query"]
  },
  "expected_output": {
    "files": ["backend/src/models/category.rs"],
    "tests": ["tests/category_test.rs"]
  },
  "deadline": "2024-01-01T12:00:00Z"
}
```

### Subagent → Main 结果格式

```json
{
  "type": "task_result",
  "task_id": "TASK-001",
  "status": "completed",
  "output": {
    "files_created": ["backend/src/models/category.rs"],
    "files_modified": [],
    "tests_added": ["tests/category_test.rs"]
  },
  "summary": "完成了分类功能，包括...",
  "next_steps": ["可以开始订单功能"]
}
```

## 与 Task System 集成

```bash
# Main Agent 分解任务
task-cli create "实现产品分类" --agent sa-001
task-cli create "实现订单列表" --agent sa-002

# Subagent 认领并执行
subagent assign sa-001 --task TASK-001

# Subagent 汇报结果
subagent complete TASK-001 --result "已完成..."
```

## 上下文注入时机

### Subagent 初始化时

```
1. System prompt (capabilities)
2. Project context (CLAUDE.md 摘要)
3. Relevant rules (按任务类型选择)
4. Task description
5. Relevant skills (按需加载)
```

### Subagent 执行中

```
- 访问文件时 → 实时读取
- 需要知识时 → skill-load 按需加载
- 遇到错误时 → 请求 Main Agent 指导
```

## 最佳实践

1. **任务粒度**: 每个 Subagent 任务 30min-2h
2. **清晰边界**: 任务之间尽量减少依赖
3. **状态同步**: 定期汇报进度
4. **结果归档**: 保留结果供后续参考

## 示例场景

```
用户: 重构整个后端，添加鉴权和订单功能

Main Agent:
1. 分析任务
2. 分解为:
   - sa-001: 鉴权模块重构
   - sa-002: 订单功能实现
   - sa-003: 代码审查

3. 创建 Subagents
   subagent create --name auth-refactor
   subagent create --name order-feature
   subagent create --name code-review

4. 并行分配任务
   subagent assign sa-001 --task "重构 JWT 鉴权"
   subagent assign sa-002 --task "实现订单 CRUD"
   subagent assign sa-003 --task "代码审查"

5. 监控进度
   subagent list

6. 汇总结果
   subagent result sa-001
   subagent result sa-002
   subagent result sa-003

7. 清理
   subagent kill sa-001
   subagent kill sa-002
   subagent kill sa-003
```

## 注意事项

- **不要创建过多 Subagents**: 3-5 个比较合理
- **保持任务独立**: 减少跨 Subagent 依赖
- **设置超时**: Subagent 空闲太久自动清理
- **结果验证**: Main Agent 需要验证 Subagent 结果
