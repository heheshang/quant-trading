# ADR-019: Binance API 签名认证架构

> 版本: v1.0  
> 状态: Accepted  
> Date: 2026-05-21

## 背景

当前 `binance_rest.rs` 仅调用公开端点。所有私有端点（账户信息/划转/挂单）因无签名而无法调用。P0-F1 为所有带认证的功能提供基础签名层。

## 决策

### 签名协议
**HMAC-SHA256 签名，遵循 Binance 规范：**

```
signature = HMAC-SHA256(secret_key, query_string)
query_string = "param1=value1&param2=value2&timestamp=1234567890123"
```

- 签名输出：hex 小写
- 必需 header：`X-MBX-APIKEY`
- 必需 query：`signature`
- 时间戳：Unix milliseconds

### 时间戳校验
- 允许偏差：±5s（Binance 硬性要求）
- 使用 Binance 服务器时间同步端点校准本地时钟
- 超偏差请求返回 401

### 加密存储
**AES-256-GCM 加密 API Key/Secret：**

- 主密钥：`ENCRYPTION_MASTER_KEY` 环境变量（32 bytes）
- 使用 `ring` 或 `aes-gcm` crate
- 每条密钥有独立 nonce，存储格式：`nonce || ciphertext || tag`

### 签名请求管道

```
1. 从数据库加载加密的 api_key/secret
2. 解密得到明文 secret
3. 构造 query_string（按字母排序）
4. 计算 HMAC-SHA256(secret, query_string)
5. 拼接完整 URL，发送请求
```

### 频率限制
- 签名请求限制：1200次/分钟（Binance 限制）
- 使用 Redis 滑动窗口计数器
- 超限返回 429

## 目录结构

```
backend/src/
  services/
    binance_signer.rs    # 签名核心逻辑
    binance_client.rs    # 统一 HTTP 客户端（公开+私有）
  db/
    api_key.rs           # API Key 模型（加密字段）
  handlers/
    exchange.rs          # /api/v1/exchange/* 端点
```

## API 设计

| 端点 | 方法 | 描述 |
|------|------|------|
| `GET /api/v1/exchange/ping` | GET | 测试签名连通性 |
| `GET /api/v1/exchange/account` | GET | 账户余额（需认证） |
| `POST /api/v1/exchange/order` | POST | 创建订单（需认证） |
| `DELETE /api/v1/exchange/order/{id}` | DELETE | 撤单（需认证） |

## 替代方案

| 方案 | 优点 | 缺点 |
|------|------|------|
| 纯前端签名（危险） | 简单 | Secret 泄露 |
| 后端签名层（选） | Secret 安全 | 需要加密存储 |
| 硬件 HSM | 最高安全 | 成本高 |

## 结论

采用后端签名层 + AES-256-GCM 加密存储，在安全性和实现成本间取得平衡。
