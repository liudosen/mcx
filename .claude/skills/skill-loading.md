# Skill Loading System - 按需技能加载

## 格言
*用到什么知识, 临时加载什么知识*

## 核心理念

Skill 不是塞进 system prompt 的静态知识，而是**按需注入的工具**。

```
传统方式 (错误):
  System: "你是一个 Rust 开发者，熟悉以下模式: ..."

按需加载 (正确):
  用户: "添加一个 REST API"
  → Agent 发现需要 skill: rust-patterns
  → 通过 tool_result 注入 skill 指南
  → Agent 参考执行
```

## Skill 文件结构

```
.claude/skills/
├── _index.md           # Skill 注册表
├── rust-patterns.md    # Rust 编码模式
├── tdd-workflow.md     # TDD 工作流
├── verification-loop.md # 验证循环
├── api-design.md       # API 设计指南
├── database.md         # 数据库模式
└── security.md         # 安全检查清单
```

## _index.md 注册表

```markdown
# Skill Index

## Available Skills

| Skill | 触发关键词 | 文件 |
|--------|-----------|------|
| rust-patterns | Rust, Axum, async | rust-patterns.md |
| tdd-workflow | TDD, 测试, test | tdd-workflow.md |
| verification-loop | 验证, check, test | verification-loop.md |
| api-design | API, REST, endpoint | api-design.md |
| database | database, SQL, query | database.md |
| security | 安全, auth, permission | security.md |

## 加载规则

- 首次提到关键词时加载
- 同一个 skill 只加载一次
- 加载后通过 tool_use result 返回给 agent
```

## Skill 动态加载流程

```
1. Agent 分析用户请求
2. 检测需要的 skill 关键词
3. 读取对应 skill 文件
4. 通过 tool_result 注入上下文
5. Agent 参考执行
```

## 实现: skill-load 工具

Agent 可以调用 `skill-load` 工具：

```bash
# 加载特定 skill
skill-load rust-patterns

# 列出可用 skills
skill-load --list
```

## Skill 内容模板

```markdown
# Skill: [名称]

## 何时使用
- [触发场景 1]
- [触发场景 2]

## 核心模式

### Pattern 1
\`\`\`rust
// 示例代码
\`\`\`

### Pattern 2
\`\`\`rust
// 示例代码
\`\`\`

## 注意事项
- [坑点 1]
- [坑点 2]
```

## 与 Context Compact 的关系

Skill 加载是上下文压缩策略的一部分：

```
层1: 核心上下文 (CLAUDE.md, rules/)
层2: 技能知识 (按需加载的 skills)
层3: 临时上下文 (tool results, 任务状态)
```

## 最佳实践

### 好的 Skill 设计
- 原子化: 一个 skill 只解决一个问题
- 可组合: skills 可以组合使用
- 描述清晰: 触发条件明确

### 避免
- 不要前置加载所有 skill
- 不要让 skill 过于庞大
- 不要用 skill 代替文档

## 示例场景

### 场景 1: 用户要求 TDD
```
用户: 用 TDD 方式实现产品列表 API

Agent 流程:
1. 识别需要 tdd-workflow skill
2. skill-load tdd-workflow
3. 接收 skill 内容
4. 按照 TDD 流程: Red → Green → Refactor
```

### 场景 2: 用户要求安全审查
```
用户: 审查登录接口的安全性

Agent 流程:
1. 识别需要 security skill
2. skill-load security
3. 接收安全检查清单
4. 按清单逐项审查
```

### 场景 3: 复杂任务组合
```
用户: 实现订单服务，需要：
  - REST API (api-design)
  - 数据库操作 (database)
  - 异步处理 (rust-patterns)
  - 测试 (tdd-workflow)

Agent 流程:
1. 加载所有相关 skills
2. 组合使用各 skill 的指导
3. 实现完整功能
```

## 触发检测关键词

```bash
# Rust patterns
rust, axum, async, tokio, trait, impl, Arc, Result

# TDD
tdd, 测试, test, mock, assert

# API Design
api, rest, endpoint, route, handler, request, response

# Database
database, sql, query, transaction, migration

# Security
security, auth, jwt, password, permission, sql injection, xss

# Verification
verify, check, clippy, cargo test, lint
```

## 性能优化

- Skills 文件保持轻量 (< 5KB each)
- 常用 patterns 内联到 rules
- 只加载当前任务相关的 skill
