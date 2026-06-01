# T0-Checklist-Binance-WS-Connector-20260517

## 检查时间
2026-05-17 09:36

## 功能名称
**Binance WebSocket Connector（市场数据管道 Phase 2）**

---

## P0 检查项（必须通过）

### ✅ 功能范围明确（25分）
**标准：一句话能说清做什么**

> 实现 Binance WebSocket 连接器，从 `wss://stream.binance.com:9443/stream` 订阅实时行情数据（ ticker/depth/kline），通过 Redis Pub/Sub 转发给 WebSocket Hub。

**得分：25/25 ✅**

---

### ✅ 用户角色明确（20分）
**标准：明确谁在用这个功能**

| 角色 | 使用场景 |
|------|----------|
| 量化交易者 | 通过前端 WebSocket 订阅实时行情，触发策略信号 |
| 系统 | Binance WS Connector 采集数据 → Redis → WebSocket Hub → 前端 |

**得分：20/20 ✅**

---

## P1 检查项

### ✅ 边界清晰（15分）
**标准：明确不做什么**

- ❌ 不支持 Binance 认证交易（API Secret）
- ❌ 不支持多交易所（OKX/Bybit）
- ❌ 不做 K线历史数据导入（已有 kline 模块）
- ✅ 只做 BTC/USDT 永续（USDT 结算）

**得分：15/15 ✅**

---

### ✅ 数据来源明确（15分）
**标准：数据从哪来、存哪、怎么用**

- **来源**：`wss://stream.binance.com:9443/stream`（主要）+ Binance REST API（降级）
- **存储**：Redis HASH `ticker:{symbol}`、STRING `depth:{symbol}`、Pub/Sub channel `market:raw`
- **使用**：WebSocket Hub 订阅 Redis Pub/Sub → 转发前端；REST API 读取缓存

**得分：15/15 ✅**

---

## P2 检查项

### ⚠️ 非功能需求（10分）
**标准：性能、安全、可用性期望**

- 重连：指数退避（1s→2s→4s→8s→max 30s）
- 心跳：60s Ping/Pong
- WS 广播队列满时断开慢客户端
- 延迟：WS 推送 ≤ 100ms

**当前评估：已覆盖主要非功能需求**

**得分：10/10 ⚠️（有记录，标记待确认）**

---

### ✅ 依赖关系（10分）
**标准：是否依赖其他功能/系统**

- ✅ 依赖 Redis（已部署）
- ✅ 依赖 Phase 1 的 Redis 缓存层
- ✅ 依赖 WebSocket Hub（Phase 3，串联工作）
- ⚠️ 依赖 JWT 认证（已有）

**得分：10/10 ✅**

---

### ✅ 验收标准（5分）
**标准：可量化的完成标准**

| ID | 标准 | 验证方式 |
|----|------|----------|
| AC1 | WS 订阅 ticker 推送频率 ≥ 1条/秒 | WS 连接 10s 计数 |
| AC2 | 连续运行 5 分钟无断连 | 监控 WS 连接状态 |
| AC3 | 服务启动后 10s 内开始接收 Binance 数据 | 日志时间戳验证 |
| AC4 | Redis 故障时 WS 自动降级 REST | kill redis 测试 |
| AC5 | 不支持交易对返回 404 | 请求 INVALIDCOIN 验证 |

**得分：5/5 ✅**

---

## 📊 最终得分

| 类别 | 得分 | 最高 |
|------|------|------|
| P0 | 45 | 45 ✅ |
| P1 | 30 | 30 ✅ |
| P2 | 25 | 25 |
| **总计** | **100** | **100** |

---

## ✅ 检查结果

**🔵 通过（≥70分 且 P0 全部通过）**

---

## 📝 需求澄清记录

1. **范围**：Phase 2 独立模块，不影响 Phase 1 REST API
2. **串联**：Phase 2 完成后再做 Phase 3（WS Hub），避免重复返工
3. **降级**：WS 断连期间 REST 继续兜底（Phase 1 已实现）
4. **测试**：需要 mock Binance WS 服务器进行单元测试