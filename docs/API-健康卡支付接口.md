# 健康卡支付接口文档

## 接口概述

小程序端使用健康卡支付订单的接口。支持使用用户身份证号作为健康卡号，自动尝试身份证后6位或用户设置的支付密码进行支付。

---

## 接口信息

**接口路径**: `POST /api/mini/orders/{id}/pay`

**认证方式**: 微信小程序 Token (Header: `Authorization: Bearer {token}`)

**业务流程**:
1. 验证订单归属和状态（仅待付款订单可支付）
2. 获取用户身份证号（作为健康卡号）
3. 调用健康卡支付服务
4. 更新订单状态和支付信息

---

## 请求参数

### Path 参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | number | 是 | 订单 ID |

### Body 参数 (JSON)

```json
{
  "paymentPassword": "string"
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| paymentPassword | string | 是 | 健康卡支付密码（用户设置的6位密码） |

**密码尝试策略**:
- 优先使用身份证后6位作为支付密码
- 如果失败，使用用户提供的 `paymentPassword`
- 最多尝试2次

---

## 响应格式

### 成功响应

```json
{
  "code": 200,
  "data": {
    "success": true,
    "paidAmount": 10,
    "orderStatus": 2,
    "message": "支付成功"
  },
  "message": "success"
}
```

### 失败响应

```json
{
  "code": 200,
  "data": {
    "success": false,
    "paidAmount": 0,
    "orderStatus": null,
    "message": "预结算业务失败: 金额不允许小于等于0！"
  },
  "message": "success"
}
```

### 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| success | boolean | 支付是否成功 |
| paidAmount | number | 实际支付金额（单位：分） |
| orderStatus | number \| null | 健康卡订单状态（2=已完成） |
| message | string | 支付结果消息 |

---

## 订单状态变更

| 原状态 | 支付成功后 | 支付失败后 |
|--------|-----------|-----------|
| 0 (待付款) | 1 (待发货) | 0 (待付款，remark 记录失败原因) |

**支付成功时更新字段**:
- `status`: 0 → 1
- `paid_amount`: 实际支付金额（分）
- `external_order_no`: 健康卡平台订单号

**支付失败时更新字段**:
- `remark`: 记录失败原因

---

## 错误码

| HTTP 状态码 | code | message | 说明 |
|------------|------|---------|------|
| 401 | 401 | 登录已过期，请重新登录 | Token 无效或过期 |
| 200 | 404 | 订单不存在 | 订单 ID 不存在 |
| 200 | 403 | 权限不足 | 订单不属于当前用户 |
| 200 | 400 | 只有待付款的订单才能支付 | 订单状态不是待付款 |
| 200 | 200 | success=false | 支付失败，查看 data.message |

---

## 前置条件

### 用户必须绑定身份证号

如果用户未绑定身份证号，支付会立即失败：

```json
{
  "code": 200,
  "data": {
    "success": false,
    "paidAmount": 0,
    "orderStatus": null,
    "message": "支付失败：用户未绑定身份证号"
  },
  "message": "success"
}
```

**解决方案**: 引导用户在个人中心绑定身份证号

---

## 金额换算规则

健康卡支付金额会自动换算：

```
订单金额（分） → 毛 ÷ 0.95 → 向上取整到最近的毛 → 转换为元
```

**示例**:
- 订单 1 分 → 换算为 0.1 元（最小支付金额）
- 订单 100 分 → 换算为 1.1 元
- 订单 1000 分 → 换算为 10.6 元

**注意**: 实际扣款金额可能略高于订单金额（因为 ÷ 0.95 换算）

---

## 请求示例

### cURL

```bash
curl -X POST 'https://api.example.com/api/mini/orders/123/pay' \
  -H 'Authorization: Bearer eyJhbGc...' \
  -H 'Content-Type: application/json' \
  -d '{
    "paymentPassword": "093538"
  }'
```

### JavaScript (微信小程序)

```javascript
wx.request({
  url: 'https://api.example.com/api/mini/orders/123/pay',
  method: 'POST',
  header: {
    'Authorization': 'Bearer ' + wx.getStorageSync('token'),
    'Content-Type': 'application/json'
  },
  data: {
    paymentPassword: '093538'
  },
  success(res) {
    if (res.data.code === 200 && res.data.data.success) {
      wx.showToast({
        title: '支付成功',
        icon: 'success'
      });
      // 跳转到订单详情或订单列表
      wx.navigateTo({
        url: '/pages/order/detail?id=123'
      });
    } else {
      wx.showToast({
        title: res.data.data.message || '支付失败',
        icon: 'none'
      });
    }
  },
  fail(err) {
    wx.showToast({
      title: '网络错误',
      icon: 'none'
    });
  }
});
```

---

## 常见问题

### Q1: 为什么需要传 paymentPassword？

A: 健康卡支付需要验证支付密码。系统会先尝试用户身份证后6位，如果失败则使用用户提供的密码。

### Q2: 支付密码错误怎么办？

A: 引导用户在健康卡平台修改支付密码，或联系客服重置。

### Q3: 支付金额为什么和订单金额不一致？

A: 健康卡支付有固定的换算规则（÷ 0.95），实际扣款金额会略高于订单金额。

### Q4: 支付失败后订单状态会变化吗？

A: 不会。订单保持待付款状态，失败原因会记录在 `remark` 字段中。

### Q5: 如何处理 401 错误？

A: Token 过期，需要重新调用微信登录接口获取新 token。

---

## 测试数据

**测试账号** (来自 `jk_order.py`):
- 卖家账号: `jyzy2240`
- 卖家密码: `DUP3AEX2qijf`
- 健康卡号: `310115199011060935`
- 支付密码: `093538`

**测试订单**:
- 金额: 1 分（实际支付 0.1 元）
- 预期结果: 支付成功，返回健康卡订单号

---

## 技术实现

### 后端实现位置

- 路由: `backend/src/routes/mini_app/order.rs:421`
- 支付服务: `backend/src/services/jk_pay.rs`
- 模型: `backend/src/models/order.rs`

### 关键逻辑

1. **Token 缓存**: 健康卡登录 token 缓存在 Redis 中，有效期 8 小时
2. **验证码识别**: 使用 ddddocr 自动识别验证码，最多重试 10 次
3. **密码重试**: 支持两次密码尝试（身份证后6位 + 用户密码）
4. **事务安全**: 订单状态更新使用数据库事务保证一致性

---

## 更新日志

### 2026-04-02
- 修复金额换算 bug：确保最小支付金额为 0.1 元
- 添加 `.max(1.0)` 防止 0 元支付被拒绝
