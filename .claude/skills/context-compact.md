# Context Compact - 上下文压缩策略

## 格言
*上下文总会满, 要有办法腾地方*

## 问题背景

长对话会导致：
- Token 消耗剧增
- 模型推理变慢
- 关键信息被稀释

## 三层压缩策略

```
┌─────────────────────────────────────────────┐
│ Layer 1: 核心层 (不压缩)                      │
│ - CLAUDE.md                                  │
│ - rules/*                                    │
│ - 当前任务相关 skill                          │
└─────────────────────────────────────────────┘
         ↓ 压缩触发
┌─────────────────────────────────────────────┐
│ Layer 2: 摘要层                              │
│ - 任务进度摘要                               │
│ - 重要决策记录                               │
│ - 技术栈关键约束                             │
└─────────────────────────────────────────────┘
         ↓ 继续增长
┌─────────────────────────────────────────────┐
│ Layer 3: 归档层                              │
│ - 已完成任务的结论                           │
│ - 历史讨论摘要                               │
│ - 可检索的知识碎片                           │
└─────────────────────────────────────────────┘
```

## 压缩触发条件

当对话消息数量超过阈值时触发压缩：

| 对话长度 | 阈值 | 策略 |
|----------|------|------|
| 短 | < 20 条 | 不压缩 |
| 中 | 20-50 条 | 摘要 Layer 2 |
| 长 | 50-100 条 | 压缩 Layer 2 + 归档 Layer 3 |
| 超长 | > 100 条 | 全面压缩 + 存档 |

## 压缩操作

### 1. 消息摘要

```bash
# 提取关键信息
compress-context --summarize

# 输出:
# ## Conversation Summary
# - 用户要求: 添加产品分类功能
# - 已完成: Category model, migration
# - 进行中: category CRUD endpoints
# - 待完成: 分类层级管理
```

### 2. 决策归档

```bash
# 归档决策
compress-context --archive-decisions

# 输出到: .claude/context/decisions/YYYY-MM.md
```

### 3. 代码片段存档

```bash
# 归档不再活跃的代码上下文
compress-context --archive-code

# 移动到: .claude/context/code-archive/
```

## 压缩工具

```bash
# 查看当前上下文大小
compress-context --size

# 执行压缩
compress-context --compact

# 查看压缩历史
compress-context --history
```

## 实现机制

### 触发检查点

在 agent loop 的每个迭代检查：

```python
def should_compact(messages):
    if len(messages) > 20:
        return "light"  # 摘要
    if len(messages) > 50:
        return "medium"  # 压缩 + 归档
    if len(messages) > 100:
        return "heavy"  # 全面压缩
    return None
```

### 压缩算法

```python
def compact_messages(messages, strategy="light"):
    if strategy == "light":
        # 只保留最后 N 条和摘要
        return summarize_recent(messages, keep=10)
    elif strategy == "medium":
        # 保留关键决策 + 最近上下文
        return keep_decisions(messages) + summarize_recent(messages, keep=15)
    else:
        # 全面压缩
        return archive_and_summarize(messages)
```

## 会话持久化

压缩后的信息存档到文件：

```
.claude/context/
├── sessions/
│   ├── 2024-01-01-morning.md    # 按日期归档
│   └── 2024-01-01-afternoon.md
├── decisions/                    # 决策记录
│   ├── 2024-01.md
│   └── 2024-02.md
├── code-archive/                 # 代码片段存档
│   └── snippets/
└── summary.md                    # 当前会话摘要
```

## 压缩原则

### 必须保留
- CLAUDE.md 核心内容
- 当前任务目标
- 未完成的工作
- 技术决策

### 可以压缩
- 已完成的实现细节
- 重复的验证输出
- 试错过程
- 详细的错误堆栈

### 永久归档
- 最终设计决策
- 解决过的 bug 方案
- 重要发现

## 与 TodoWrite 的集成

```
压缩时检查 TODO 状态:
- 如果任务状态变化，更新 TODO
- 压缩后仍然可以从 TODO 恢复上下文
```

## 使用示例

```bash
# 查看当前上下文大小
$ compress-context --size
Context: 45 messages (约 3200 tokens)

# 执行轻度压缩
$ compress-context --compact
压缩摘要:
- 保留最近 10 条消息
- 归档 35 条历史消息
- 保存决策到 .claude/context/decisions/

新上下文大小: 12 messages (约 850 tokens)
```

## 最佳实践

1. **不要等到满了才压缩** - 预防性压缩更高效
2. **保持摘要准确** - 摘要要能恢复关键信息
3. **分类存储** - 决策、代码、讨论分开放置
4. **定期清理** - 删除过时的归档

## 警告标志

当看到这些时应该考虑压缩：
- Token 使用量快速增长
- 模型开始重复之前的错误
- 关键信息被埋在大量历史中
