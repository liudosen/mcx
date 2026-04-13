# 小程序订单接口文档

**Base URL**: `/api/mini`
**认证方式**: 所有接口须在 Header 中携带 token（`Authorization: Bearer <token>` 或项目约定的 header key）

---

## 1. 提交订单

**POST** `/api/mini/orders`

**Request Body** (JSON):
```json
{
  "addressId": "1",
  "items": [
    {
      "skuId": "10",
      "quantity": 2
    }
  ],
  "remark": "备注信息"
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| addressId | string | 是 | 收货地址 ID |
| items | array | 是 | 商品列表，不能为空 |
| items[].skuId | string | 是 | SKU ID |
| items[].quantity | int | 是 | 购买数量，必须 > 0 |
| remark | string | 否 | 订单备注 |

**成功响应** (code: 200):
```json
{
  "code": 200,
  "message": "success",
  "data": { }
}
```
data 为 OrderResp，见文末数据结构说明。

**错误情况**:

| HTTP / code | message | 原因 |
|-------------|---------|------|
| 400 | 订单商品不能为空 | items 为空数组 |
| 400 | addressId 格式错误 | addressId 非数字字符串 |
| 400 | 商品数量必须大于0 | quantity <= 0 |
| 403 | - | 地址不属于当前用户 |
| 404 | 收货地址不存在 | addressId 对应记录不存在 |
| 404 | SKU {id} 不存在或商品已下架 | skuId 无效或商品已下架 |

---

## 2. 我的订单列表

**GET** `/api/mini/orders`

**Query 参数**:

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| page | int | 否 | 页码，默认 1 |
| pageSize | int | 否 | 每页数量，默认 10，最大 50 |
| status | int | 否 | 订单状态筛选，不传则返回全部（状态枚举见文末） |

**成功响应** (code: 200):
```json
{
  "code": 200,
  "message": "success",
  "data": {
    "list": [],
    "total": 100,
    "page": 1,
    "pageSize": 10
  }
}
```

---

## 3. 订单详情

**GET** `/api/mini/orders/{id}`

**Path 参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| id | int | 订单 ID |

**成功响应** (code: 200):
```json
{
  "code": 200,
  "message": "success",
  "data": { }
}
```
data 为 OrderResp。

**错误情况**:

| code | message | 原因 |
|------|---------|------|
| 403 | - | 不是当前用户的订单 |
| 404 | 订单不存在 | id 对应记录不存在 |

---

## 4. 取消订单

**PUT** `/api/mini/orders/{id}/cancel`

**Path 参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| id | int | 订单 ID |

**限制**: 只有状态为 `0（待付款）` 的订单可以取消，取消后状态变为 `4（已取消）`。

**成功响应** (code: 200):
```json
{
  "code": 200,
  "message": "success",
  "data": { }
}
```
data 为更新后的 OrderResp。

**错误情况**:

| code | message | 原因 |
|------|---------|------|
| 400 | 只有待付款的订单才能取消 | 订单状态不是待付款 |
| 403 | - | 不是当前用户的订单 |
| 404 | 订单不存在 | id 对应记录不存在 |

---

## 数据结构：OrderResp

```json
{
  "id": "1",
  "orderNo": "20260401120000001",
  "userId": "1",
  "status": 0,
  "statusLabel": "待付款",
  "totalAmount": 9900,
  "paidAmount": 0,
  "discountAmount": 0,
  "remark": null,
  "address": {
    "id": "1",
    "receiverName": "张三",
    "phone": "13800138000",
    "province": "广东省",
    "city": "深圳市",
    "district": "南山区",
    "detailAddress": "科技园路1号",
    "label": "家"
  },
  "items": [
    {
      "id": "1",
      "spuId": "5",
      "skuId": "10",
      "goodsTitle": "商品名称",
      "goodsImage": "https://example.com/image.jpg",
      "specInfo": [],
      "unitPrice": 4950,
      "quantity": 2,
      "subtotal": 9900
    }
  ],
  "createdAt": "2026-04-01 12:00:00",
  "updatedAt": "2026-04-01 12:00:00"
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| id | string | 订单 ID |
| orderNo | string | 订单号 |
| userId | string | 用户 ID |
| status | int | 状态值，见枚举表 |
| statusLabel | string | 状态文字 |
| totalAmount | int | 总金额（单位：分） |
| paidAmount | int | 实付金额（单位：分） |
| discountAmount | int | 优惠金额（单位：分） |
| remark | string / null | 订单备注 |
| address | object / null | 收货地址快照 |
| address.receiverName | string | 收货人姓名 |
| address.phone | string | 收货人电话 |
| address.province | string | 省 |
| address.city | string | 市 |
| address.district | string | 区/县 |
| address.detailAddress | string | 详细地址 |
| address.label | string | 地址标签（家/公司等） |
| items | array | 订单商品列表 |
| items[].goodsTitle | string | 商品名称 |
| items[].goodsImage | string | 商品图片 URL |
| items[].specInfo | array | 规格信息（JSON 数组） |
| items[].unitPrice | int | 单价（单位：分） |
| items[].quantity | int | 购买数量 |
| items[].subtotal | int | 小计（单位：分） |
| createdAt | string | 创建时间，格式 `YYYY-MM-DD HH:mm:ss` |
| updatedAt | string | 更新时间，格式 `YYYY-MM-DD HH:mm:ss` |

> **注意**: 所有金额字段单位均为**分**，前端展示时需除以 100。

---

## 订单状态枚举

| 值 | statusLabel | 说明 |
|----|-------------|------|
| 0 | 待付款 | 订单已提交，等待支付 |
| 1 | 待发货 | 已支付，等待商家发货 |
| 2 | 待收货 | 已发货，等待用户确认收货 |
| 3 | 已完成 | 订单完成 |
| 4 | 已取消 | 订单已取消 |
