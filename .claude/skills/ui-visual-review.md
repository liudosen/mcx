# UI Visual Review Skill

## Description
使用 Puppeteer 截屏 + AI 图片分析进行 UI 样式审查和调整的工作流。

## 何时使用
- 修改 CSS 样式后需要验证视觉效果
- 调整布局对齐后确认是否居中
- 修改组件样式后对比效果
- 发现 UI 问题需要定位

## 核心流程

### Step 1: 截屏保存

使用 Puppeteer MCP 截取页面截图并保存为 base64 编码的 PNG：

```bash
# 截取页面截图，启用 no-sandbox（重要！）
mcp__puppeteer__puppeteer_navigate(
  url="http://localhost:9528/目标页面",
  launchOptions={"args": ["--no-sandbox", "--disable-setuid-sandbox"], "headless": true},
  allowDangerous=true
)

# 获取 base64 截图
mcp__puppeteer__puppeteer_screenshot(
  name="temp_screenshot",
  encoded=true
)
```

### Step 2: 解码保存图片

将 base64 响应保存为图片文件：

```bash
# 从响应中提取 base64 数据并解码
cat <响应文件> | grep -o '"data:image/png;base64,[^"]*' | sed 's/"data:image\/png;base64,//' | base64 -d > /tmp/analysis.png
```

### Step 3: AI 分析

使用 `understand_image` 分析截图：

```
mcp__MiniMax__understand_image(
  prompt="分析这个页面的 UI 样式问题，特别关注：1) 元素是否居中 2) 对齐是否正确 3) 间距是否均匀 4) 颜色对比度是否合适",
  image_source="/tmp/analysis.png"
)
```

### Step 4: 根据分析调整样式

根据 AI 分析结果修改对应的 CSS/样式文件。

### Step 5: 删除截图

```bash
rm /tmp/analysis.png
```

## 完整工作流示例

```
用户: 登录页面的输入框没有垂直居中

Agent:
1. mcp__puppeteer__puppeteer_navigate(url="http://localhost:9528/login", ...)
2. mcp__puppeteer__puppeteer_screenshot(name="login", encoded=true)
3. 解码保存到 /tmp/login.png
4. mcp__MiniMax__understand_image(prompt="分析输入框是否垂直居中...", image_source="/tmp/login.png")
5. 根据分析结果修改样式文件
6. rm /tmp/login.png
```

## 注意事项

- **no-sandbox**: Chrome 在 root 用户下必须加 `--no-sandbox` 和 `--disable-setuid-sandbox`
- **launchOptions**: 每次 navigate 都需要传入 `launchOptions` 才能生效
- **截图路径**: 截图数据通过 tool result 返回，需要手动解码保存
- **清理**: 完成后务必删除临时截图文件
- **端口**: 默认前端开发服务器端口 9528

## 触发关键词
- 截屏, screenshot, 截图分析
- UI 问题, 样式不对, 没有居中
- 调整样式, 修改 CSS, visual review
