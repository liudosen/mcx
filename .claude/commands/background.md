# Background Tasks - 后台任务系统

## 格言
*慢操作丢后台, agent 继续想下一步*

## 问题背景

某些操作耗时较长：
- `cargo build` (可能 5-10 分钟)
- `cargo test` (特别是集成测试)
- Docker 构建
- 数据库迁移

如果 agent 同步等待，就无法做其他事情。

## 解决方案

```
同步方式 (阻塞):
Agent: 执行 cargo build...
      [等待 10 分钟]
      [继续]

异步方式 (非阻塞):
Agent: 启动后台任务 cargo build...
      [立即继续想下一步]
      [后台任务完成后通知]
```

## 实现机制

### 后台任务文件

```bash
# 任务队列
.claude/background/queue/

# 运行中任务
.claude/background/running/

# 完成任务
.claude/background/completed/

# 失败任务
.claude/background/failed/
```

### 任务状态文件

```json
{
  "id": "bg-001",
  "command": "cargo build --release",
  "cwd": "/root/workspace/welfare-store/backend",
  "started": "2024-01-01T10:00:00Z",
  "pid": 12345,
  "status": "running",
  "log_file": ".claude/background/running/bg-001.log"
}
```

## CLI 工具

```bash
# 启动后台任务
bg run "cargo build --release" --name "build-release"

# 列出运行中任务
bg list

# 查看任务日志
bg logs bg-001

# 等待任务完成
bg wait bg-001

# 取消任务
bg kill bg-001

# 重试失败任务
bg retry bg-001
```

## Agent 使用模式

### 模式 1: 启动后立即继续

```python
# Agent 决策:
# 1. 启动后台编译
bg run "cargo build"

# 2. 立即继续 (在编译期间)
#    - 编写测试
#    - 更新文档
#    - 规划下一步

# 3. 稍后检查编译结果
result = bg wait "build-release"
if result.success:
    # 继续部署
else:
    # 修复错误
```

### 模式 2: 通知机制

```bash
# 设置任务完成通知
bg run "cargo test" --notify "Task complete"

# 通知可以是:
# - 文件写入
# - webhook
# - 邮件
```

### 模式 3: 并行后台任务

```bash
# 同时启动多个独立任务
bg run "cargo build" --name "build" &
bg run "cargo fmt" --name "fmt" &
bg run "生成文档" --name "docs" &

# 等待所有完成
bg wait-all
```

## 日志管理

```bash
# 实时查看日志
bg logs -f bg-001

# 只看最后 N 行
bg logs -n 50 bg-001

# 搜索关键词
bg logs -g "error" bg-001
```

## 超时处理

```bash
# 设置超时 (默认 30 分钟)
bg run "cargo build" --timeout 15m

# 超时后自动 kill
bg run "cargo build" --timeout 10m --on-timeout kill
```

## 与其他系统集成

### 与 Task System 集成

```bash
# 任务开始时自动启动后台
task-cli update TASK-001 --status in_progress
bg run "cargo build" --task TASK-001

# 任务完成时自动检查
if bg check TASK-001; then
    task-cli update TASK-001 --status completed
fi
```

### 与 Verification Loop 集成

```bash
# 后台运行验证，主流程继续
bg run "cargo clippy -- -D warnings" --name "clippy" &

# 继续其他工作
# ...

# 稍后检查结果
if bg success clippy; then
    echo "Lint passed"
else
    bg logs clippy  # 查看错误
fi
```

## 最佳实践

1. **I/O 密集型放后台**: 编译、测试、部署
2. **需要结果的操作放同步**: 关键验证、决策判断
3. **设置合理的超时**: 避免僵尸任务
4. **及时清理日志**: 避免占用过多空间

## 示例场景

### 场景: 编译期间继续工作

```
用户: 重构 auth 模块并添加测试

Agent:
1. 启动后台编译 (检测是否有语法错误)
   bg run "cargo build 2>&1" --name "syntax-check"

2. 立即开始重构
   - 修改 auth.rs
   - 添加新的验证逻辑

3. 后台编译完成，发现语法错误
   bg notify "syntax error in auth.rs"

4. 修复错误
   - 修改代码
   - bg run "cargo build" --name "syntax-check"

5. 后台重新编译期间
   - 编写测试用例

6. 最终验证
   bg run "cargo test" --name "final-test"
```

### 场景: 并行验证

```bash
# 同时运行多个检查
bg run "cargo check" --name "check"
bg run "cargo clippy -- -D warnings" --name "clippy"
bg run "cargo test" --name "test"

# 等待所有完成
bg wait-all check clippy test

# 检查结果
for task in check clippy test; do
    if bg success $task; then
        echo "$task: PASSED"
    else
        echo "$task: FAILED"
        bg logs $task
    fi
done
```

## 注意事项

- **不要启动过多后台任务**: 避免系统负载过高
- **监控资源使用**: 特别是内存密集型任务
- **设置日志上限**: 避免日志无限增长
