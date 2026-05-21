# PRD: P0-F1 Binance API 签名加密

> 版本: v1.0  
> 状态: Draft  
> Author: PM  
> Date: 2026-05-21

---

## 1. 背景

当前 `binance_rest.rs` 仅调用公开端点（无需签名）。所有私有端点（账户信息、划转、挂单等）无法访问。P0-F1 是阻塞所有带认证功能的前置依赖。

## 2. 目标

| 目标 | 指标 |
|------|------|
| HMAC-SHA256 签名 | 所有私有请求正确签名 |
| 时间戳校验 | 偏移 > ±5s 的请求被拒绝 |
| 密钥加密存储 | API Key/Secret 使用 AES-256-GCM 加密 |
| 频率限制 | 签名请求 ≤ 1200次/分钟 |

## 3. 功能范围

### F1: HMAC-SHA256 签名
- `X-MBX-APIKEY` header + `signature` query param
- 所有私有端点必须签名

### F2: 时间戳校验
- 签名中包含时间戳
- 偏移超过 ±5s 的请求返回 401

### F3: 加密存储
- API Key/Secret 使用 AES-256-GCM 加密
- 密钥由主密钥派生

### F4: 签名请求示例端点
- `GET /api/v1/exchange/ping` — 测试连通性

## 4. 验收标准

| AC | 标准 |
|----|------|
| AC1 | 使用有效签名访问 /exchange/ping 返回 200 |
| AC2 | 无签名请求返回 401 |
| AC3 | 错误签名返回 401 |
| AC4 | 时间戳过期返回 401 |

## 5. 技术方案

- **签名算法**: HMAC-SHA256，输出为 hex 小写
- **加密**: AES-256-GCM，主密钥来自环境变量 `ENCRYPTION_MASTER_KEY`
- **时间戳**: Unix ms，Binance 要求服务器时间偏差 < 5s
