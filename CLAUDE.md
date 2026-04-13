# Welfare Store API - Harness Engineering Guidelines

## 核心格言 (Harness Engineering)

> **模型就是 Agent。代码是 Harness。造好 Harness，Agent 会完成剩下的。**

```
Harness = Tools + Knowledge + Observation + Action Interfaces + Permissions

    Tools:          文件读写、Shell、API、数据库
    Knowledge:      产品文档、领域资料、API 规范
    Observation:    git diff、错误日志、构建状态
    Action:         CLI 命令、API 调用
    Permissions:    沙箱隔离、审批流程
```

## 12 课程格言表

| 课程 | 主题 | 格言 |
|------|------|------|
| s01 | Agent 循环 | *One loop & Bash is all you need* |
| s02 | Tool Use | *加一个工具, 只加一个 handler* |
| s03 | TodoWrite | *没有计划的 agent 走哪算哪* |
| s04 | Subagent | *大任务拆小, 每个小任务干净的上下文* |
| s05 | Skill Loading | *用到什么知识, 临时加载什么知识* |
| s06 | Context Compact | *上下文总会满, 要有办法腾地方* |
| s07 | Task System | *大目标要拆成小任务, 排好序, 记在磁盘上* |
| s08 | Background Tasks | *慢操作丢后台, agent 继续想下一步* |

---

## Agent Loop 模式

```python
def agent_loop(messages):
    while True:
        response = client.messages.create(
            model=MODEL, system=SYSTEM,
            messages=messages, tools=TOOLS,
        )
        messages.append({"role": "assistant",
                         "content": response.content})

        if response.stop_reason != "tool_use":
            return

        results = []
        for block in response.content:
            if block.type == "tool_use":
                output = TOOL_HANDLERS[block.name](**block.input)
                results.append({
                    "type": "tool_result",
                    "tool_use_id": block.id,
                    "content": output,
                })
        messages.append({"role": "user", "content": results})
```

每个课程在这个循环之上叠加一个 harness 机制。循环本身始终不变。

---

## 如何使用这个 Harness

### 自然语言交互（自动生效）
描述你的需求，Claude会自动：
- 读取 `CLAUDE.md` → 了解项目背景和技术栈
- 遵循 `.claude/rules/*.md` → 应用安全和编码规范
- 参考 `.claude/skills/*.md` → 使用 TDD 和验证闭环模式
- 使用 `.claude/commands/*.md` → 任务规划和 TDD 工作流

### 显式调用命令
| 命令 | 触发方式 | 读取文件 |
|------|----------|----------|
| 规划 | `/plan 需求描述` | commands/plan.md |
| TDD | `/tdd` | commands/tdd.md + skills/tdd-workflow.md |
| 任务 | `/tasks` | commands/tasks.md |
| Todo | `/todo` | commands/todo.md |
| 后台任务 | `/background` | commands/background.md |
| 审查 | `/review` | commands/review.md |
| 安全 | `/security` | commands/security.md |

---

## 自动闭环 (Post-Task Automation)

**每次完成代码修改后，自动执行以下流程：**

### 1️⃣ Self-Review
- 检查逻辑正确性
- 检查错误处理
- 检查安全性
- 确保遵循项目模式

### 2️⃣ 验证循环
```bash
cargo check      # 类型检查
cargo clippy -- -D warnings  # Lint检查
cargo test       # 运行测试
cargo fmt        # 格式化
```

### 3️⃣ 自动修复
- 如果有问题，自动修复
- 重新验证
- 直到全部通过

### 4️⃣ 更新文档
- API变更 → 更新 CLAUDE.md
- 新模式 → 更新对应 skill 文件
- 新端点 → 更新 API 文档

### 成功标准
```
✓ cargo check
✓ cargo clippy
✓ cargo test
✓ cargo fmt
```

---

## Project Overview

**Type**: Full-stack Welfare Store Application
**Backend**: Rust/Axum REST API (分层架构)
**Frontend**: Vue 2 Admin Template (Element UI)
**Database**: MySQL via SQLx
**Auth**: JWT with Redis session storage
**Purpose**: 福利商城后端，支持管理后台 + 微信小程序双端

## Project Structure

```
welfare-store/
├── backend/                    # Rust/Axum REST API
│   ├── src/
│   │   ├── main.rs             # App entry, router setup
│   │   ├── routes/             # 按业务线分层
│   │   │   ├── mod.rs          # 模块入口
│   │   │   ├── shared.rs       # 共享类型 (ApiResponse)
│   │   │   ├── mini_app/       # 小程序接口
│   │   │   │   ├── mod.rs
│   │   │   │   ├── auth.rs     # 微信登录、token 验证
│   │   │   │   └── address.rs  # 收货地址 CRUD
│   │   │   └── admin/          # 管理后台接口
│   │   │       ├── mod.rs
│   │   │       ├── auth.rs     # admin 登录、权限验证
│   │   │       ├── product.rs  # 产品管理
│   │   │       └── wechat_user.rs # 微信用户管理
│   │   ├── models/
│   │   │   ├── address.rs      # 收货地址模型
│   │   │   ├── admin_user.rs
│   │   │   ├── product.rs
│   │   │   └── wechat_user.rs
│   │   ├── state.rs            # AppState (DB pool, JWT config, Redis)
│   │   ├── config.rs           # Environment config
│   │   └── error.rs            # Unified error handling
│   ├── migrations/             # SQLx database migrations
│   ├── Cargo.toml
│   └── Dockerfile
│
├── frontend/                   # Vue 2 Admin Template
│
├── .claude/                    # Harness 配置
│
└── CLAUDE.md                   # This file
```

## Tech Stack

### Backend
- **Framework**: Axum 0.8 with Tower HTTP
- **Database**: MySQL (sqlx with migrate)
- **Cache**: Redis (token 存储)
- **Auth**: JWT + bcrypt
- **Runtime**: Tokio async runtime
- **Error Handling**: Thiserror + anyhow

### Frontend
- **Framework**: Vue 2.6
- **UI Library**: Element UI 2.13
- **State Management**: Vuex 3
- **Router**: Vue Router 3
- **HTTP Client**: Axios 0.18
- **Build Tool**: Vue CLI 4

## Module Responsibilities

### Backend Routes (分层设计)
| Module | Responsibility |
|--------|----------------|
| `routes/mini_app/` | 微信小程序 API（用户侧） |
| `routes/mini_app/auth.rs` | 微信登录、token 验证、WechatTokenExpired 错误 |
| `routes/mini_app/address.rs` | 收货地址 CRUD |
| `routes/admin/` | 管理后台 API（运营侧） |
| `routes/admin/auth.rs` | admin 登录、权限码 |
| `routes/admin/product.rs` | 产品管理 |
| `routes/admin/wechat_user.rs` | 微信用户管理 |
| `routes/shared.rs` | 共享类型 ApiResponse |

### Backend Core
| Module | Responsibility |
|--------|----------------|
| `models/` | 数据结构、SQLx 类型映射 |
| `state.rs` | 共享应用状态 (DB pool, Redis, JWT config) |
| `config.rs` | 环境变量解析 |
| `error.rs` | 统一错误类型和响应 |

## API Architecture

### 业务线分离
- **管理后台 API**: `/auth/*`, `/api/admin/*`
- **小程序 API**: `/api/mini/*`

### Authentication

| 业务线 | Token 存储 | 有效期 | Redis Key |
|--------|-----------|--------|-----------|
| 管理后台 | Redis | JWT_EXPIRY_HOURS | `welfare:token:{token}` |
| 小程序 | Redis | 30 天 | `welfare:wechat:token:{token}` |

### API Endpoints

#### 管理后台 (Admin)
| Method | Path | Description |
|--------|------|-------------|
| POST | `/auth/login` | admin 登录 |
| POST | `/auth/refresh` | 刷新 token |
| POST | `/auth/logout` | 登出 |
| GET | `/auth/codes` | 获取权限码 |
| GET | `/api/admin/products` | 产品列表 |
| POST | `/api/admin/products` | 创建产品 |
| GET | `/api/admin/products/{id}` | 产品详情 |
| PUT | `/api/admin/products/{id}` | 更新产品 |
| DELETE | `/api/admin/products/{id}` | 删除产品 |
| GET | `/api/admin/wechat/users` | 微信用户列表 |
| POST | `/api/admin/wechat/users` | 创建微信用户 |
| GET | `/api/admin/wechat/users/{id}` | 用户详情 |
| PUT | `/api/admin/wechat/users/{id}` | 更新用户 |
| DELETE | `/api/admin/wechat/users/{id}` | 删除用户 |

#### 小程序 (Mini App)
| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/mini/login` | 微信登录 |
| GET | `/api/mini/addresses` | 地址列表 |
| POST | `/api/mini/addresses` | 创建地址 |
| GET | `/api/mini/addresses/{id}` | 地址详情 |
| PUT | `/api/mini/addresses/{id}` | 更新地址 |
| DELETE | `/api/mini/addresses/{id}` | 删除地址 |
| PUT | `/api/mini/addresses/{id}/default` | 设置默认地址 |

## API Conventions

### Response Format
```json
{
  "code": 200,
  "data": {...},
  "message": "success"
}
```

### Error Format
```json
{
  "code": 401,
  "data": null,
  "message": "登录已过期，请重新登录"
}
```

### Token 过期处理
- 小程序端收到 `code: 401` + `message: "登录已过期，请重新登录"` 时
- 自动跳转登录页面重新获取 token

## Database Migrations

- Location: `backend/migrations/`
- Naming: `YYYYMMDDHHMMSS_description.sql`
- Run automatically on startup via `sqlx::migrate!`

## Engineering Principles

### 1. Plan-Before-Execute
Before implementing any feature:
1. Understand the full scope
2. Design the API contract
3. Plan the database migration if needed
4. Consider error cases

### 2. TDD Workflow
- Write test first for business logic
- Run `cargo test` before committing
- Maintain test coverage for new features

### 3. Security-First
- Never log sensitive data (passwords, tokens)
- Validate all inputs
- Use parameterized queries (sqlx prevents SQL injection)
- Handle errors without leaking internals

### 4. Verification Loop
After every code change:
1. `cargo check` - Type checking
2. `cargo test` - Run tests
3. `cargo clippy -- -D warnings` - Lint
4. Review the diff for unintended changes

## Code Style

- Follow Rust idioms (clippy defaults)
- Use `?` for error propagation
- Async functions return `Result<T, AppError>`
- Log with `tracing::info!` / `tracing::warn!` / `tracing::error!`

---

## Harness 系统架构

```
┌─────────────────────────────────────────────────────┐
│                     Agent (Model)                    │
│         神经网络 - Transformer, 学会推理和决策        │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│                    Harness                          │
│  ┌─────────────────────────────────────────────┐   │
│  │ Tools: cargo, git, bash, file read/write     │   │
│  ├─────────────────────────────────────────────┤   │
│  │ Knowledge: skills/*.md (按需加载)           │   │
│  ├─────────────────────────────────────────────┤   │
│  │ Observation: git diff, logs, build status   │   │
│  ├─────────────────────────────────────────────┤   │
│  │ Action: CLI commands, API calls             │   │
│  ├─────────────────────────────────────────────┤   │
│  │ Permissions: settings.json allow rules      │   │
│  └─────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│                   Project                           │
│   welfare-store: Rust/Axum backend + Vue frontend   │
└─────────────────────────────────────────────────────┘
```

**造好 Harness。Agent 会完成剩下的。**
