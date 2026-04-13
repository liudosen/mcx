# Skill Index

> Skill 按需加载系统 - 用到什么加载什么

## Available Skills

| Skill | 触发关键词 | 文件 |
|--------|-----------|------|
| rust-patterns | rust, axum, async, tokio | rust-patterns.md |
| tdd-workflow | tdd, 测试, test driven | tdd-workflow.md |
| verification-loop | verify, check, clippy, test | verification-loop.md |
| skill-loading | skill, load, inject | skill-loading.md |
| context-compact | context, compress, compact | context-compact.md |
| api-design | api, rest, endpoint, route | api-design.md |
| database | database, sql, query, migration | database.md |
| security | security, auth, jwt, permission | security.md |
| ui-visual-review | 截屏, screenshot, UI 问题, 样式, 居中, visual review | ui-visual-review.md |

## 加载规则

1. **首次提及触发**: 首次提到关键词时加载
2. **同 Skill 只加载一次**: 避免重复注入
3. **通过 tool_result 注入**: 不是塞进 system prompt

## 动态加载流程

```
1. Agent 分析用户请求
2. 检测 Skill 关键词
3. 读取 Skill 文件内容
4. 通过 tool_result 返回给 Agent
5. Agent 参考执行
```

## 使用方式

### Agent 内部使用

```bash
# 加载特定 Skill
skill-load rust-patterns

# 列出所有可用 Skills
skill-load --list

# 查看 Skill 内容
skill-load --show tdd-workflow
```

### 用户触发

```
用户: 用 TDD 方式实现登录功能

Agent 检测: tdd, test 关键词
→ 自动加载 tdd-workflow skill
→ 加载 security skill (auth 相关)
→ 按照 TDD 流程执行
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

## 注意事项
- [坑点 1]
- [坑点 2]
```

## 性能优化

- Skills 文件保持轻量 (< 5KB each)
- 常用 patterns 内联到 rules
- 只加载当前任务相关的 Skill

## 触发检测关键词

| Skill | 关键词 |
|--------|--------|
| rust-patterns | rust, axum, async, tokio, trait, impl, Arc, Result |
| tdd-workflow | tdd, 测试, test, mock, assert |
| verification-loop | verify, check, clippy, cargo test, lint |
| api-design | api, rest, endpoint, route, handler, request, response |
| database | database, sql, query, transaction, migration |
| security | security, auth, jwt, password, permission, sql injection |
| context-compact | context, compress, summary, archive |
| skill-loading | skill, load, inject, on-demand |
