# OSS 对象存储配置指南

## 功能说明

本项目已集成阿里云 OSS 对象存储服务，用于商品图片上传和管理。

### 功能特性

- ✅ 前端直传 OSS（不占用服务器带宽）
- ✅ 自动生成上传签名（1小时有效期）
- ✅ 支持单图/多图上传
- ✅ 文件大小限制（默认 5MB）
- ✅ 自动按日期分目录存储
- ✅ 支持图片预览和删除

## 配置步骤

### 1. 获取阿里云 OSS 凭证

登录阿里云控制台：https://oss.console.aliyun.com/

1. 创建 Bucket（存储空间）
   - 区域：选择离用户最近的区域（如：华东1-杭州）
   - 读写权限：**公共读**（允许匿名访问图片）
   - 其他选项保持默认

2. 获取 AccessKey
   - 进入 RAM 访问控制：https://ram.console.aliyun.com/
   - 创建用户 → 勾选"编程访问"
   - 保存 AccessKey ID 和 AccessKey Secret
   - 为用户授权：`AliyunOSSFullAccess`

### 2. 配置后端环境变量

编辑 `backend/.env` 文件，填入以下配置：

```bash
# OSS 配置
OSS_ENDPOINT=oss-cn-hangzhou.aliyuncs.com          # OSS 区域节点
OSS_ACCESS_KEY_ID=LTAI5tXXXXXXXXXXXXXX             # 你的 AccessKey ID
OSS_ACCESS_KEY_SECRET=xxxxxxxxxxxxxxxxxxxxx         # 你的 AccessKey Secret
OSS_BUCKET=welfare-store                            # Bucket 名称
OSS_DOMAIN=https://welfare-store.oss-cn-hangzhou.aliyuncs.com  # 访问域名
```

**配置说明：**

- `OSS_ENDPOINT`: OSS 区域节点地址
  - 华东1（杭州）: `oss-cn-hangzhou.aliyuncs.com`
  - 华北2（北京）: `oss-cn-beijing.aliyuncs.com`
  - 华南1（深圳）: `oss-cn-shenzhen.aliyuncs.com`
  - 更多区域：https://help.aliyun.com/document_detail/31837.html

- `OSS_DOMAIN`: 图片访问域名
  - 默认域名：`https://{bucket}.{endpoint}`
  - 自定义域名：如果绑定了 CDN，填写 CDN 域名

### 3. 配置 CORS（跨域访问）

在阿里云 OSS 控制台配置 CORS 规则：

1. 进入 Bucket → 权限管理 → 跨域设置
2. 创建规则：
   - 来源：`*`（或指定前端域名）
   - 允许 Methods：`GET, POST, PUT, DELETE, HEAD`
   - 允许 Headers：`*`
   - 暴露 Headers：`ETag, x-oss-request-id`
   - 缓存时间：`600`

### 4. 启动服务

```bash
cd backend
cargo run
```

后端服务启动后，上传接口地址：
```
GET http://localhost:8081/api/admin/upload/signature?filename=test.jpg
```

### 5. 前端使用

在商品管理页面，使用 `<oss-upload>` 组件：

```vue
<template>
  <!-- 单图上传 -->
  <oss-upload v-model="form.image_url" :limit="1" />

  <!-- 多图上传 -->
  <oss-upload v-model="form.images" :limit="9" :multiple="true" />
</template>

<script setup>
import OssUpload from '@/components/OssUpload/index.vue'
import { ref } from 'vue'

const form = ref({
  image_url: '',
  images: []
})
</script>
```

## 文件存储规则

上传的文件按以下规则存储：

```
products/
  └── 20260403/              # 按日期分目录
      ├── uuid1_image1.jpg   # UUID + 原文件名
      ├── uuid2_image2.png
      └── ...
```

## API 接口

### 获取上传签名

**请求：**
```http
GET /api/admin/upload/signature?filename=test.jpg
Authorization: Bearer {token}
```

**响应：**
```json
{
  "code": 200,
  "data": {
    "url": "https://welfare-store.oss-cn-hangzhou.aliyuncs.com",
    "key": "products/20260403/uuid_test.jpg",
    "policy": "eyJleHBpcmF0aW9uIjoi...",
    "oss_access_key_id": "LTAI5tXXXXXX",
    "signature": "xxxxxxxxxxxxx",
    "expire": 1712134800,
    "host": "https://welfare-store.oss-cn-hangzhou.aliyuncs.com"
  },
  "message": "success"
}
```

## 常见问题

### 1. 上传失败：403 Forbidden

**原因：** AccessKey 权限不足或签名错误

**解决：**
- 检查 AccessKey 是否正确
- 确认 RAM 用户已授权 `AliyunOSSFullAccess`
- 检查 Bucket 权限设置为"公共读"

### 2. 图片无法访问：404 Not Found

**原因：** Bucket 权限设置错误

**解决：**
- 进入 OSS 控制台 → Bucket → 权限管理
- 读写权限设置为：**公共读**

### 3. 跨域错误：CORS policy

**原因：** 未配置 CORS 规则

**解决：**
- 按照上述步骤配置 CORS
- 确保允许的 Methods 包含 `POST`

### 4. 上传速度慢

**优化方案：**
- 选择离用户最近的 OSS 区域
- 配置 CDN 加速
- 前端压缩图片后再上传

## 成本优化建议

1. **CDN 加速**：配置阿里云 CDN，减少 OSS 流量费用
2. **图片压缩**：上传前压缩图片，减少存储空间
3. **生命周期管理**：设置过期文件自动删除规则
4. **存储类型**：下架商品图片转为低频存储

## 安全建议

1. ✅ 使用 RAM 子账号，不要使用主账号 AccessKey
2. ✅ 定期轮换 AccessKey
3. ✅ 上传签名设置合理的过期时间（1小时）
4. ✅ 限制文件类型和大小
5. ✅ 敏感图片使用私有 Bucket + 签名 URL

## 技术架构

```
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│  管理后台    │      │  Backend    │      │  阿里云OSS   │
│  (Vue)      │─────▶│  (Rust)     │─────▶│             │
└─────────────┘      └─────────────┘      └─────────────┘
      │                     │                     │
      │              1. 请求上传签名              │
      │◀────────────────────┘                     │
      │                                           │
      │              2. 直传文件                   │
      └──────────────────────────────────────────▶│
                                                  │
                           3. 返回图片URL          │
                           ◀─────────────────────┘
```

## 相关文档

- 阿里云 OSS 文档：https://help.aliyun.com/product/31815.html
- OSS 签名机制：https://help.aliyun.com/document_detail/31951.html
- OSS 区域列表：https://help.aliyun.com/document_detail/31837.html
