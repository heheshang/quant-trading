# ADR-021: 套利模块架构设计

**状态：** 已批准
**日期：** 2026-05-21
**版本：** v1.0

---

## 1. 背景与目标

P3-F1 套利模块需要支持三种套利策略：跨期套利、跨品种套利、期现套利。核心挑战在于：

1. **多数据源同步**：套利需要同时获取 A/B 两侧价格
2. **双向订单协调**：需要同时或先后下单（side_a + side_b）
3. **价差计算**：实时计算 spread/z-score，支持多种计算模式
4. **信号过滤**：避免虚假信号，只在高置信度时入场

## 2. 技术决策

### 2.1 模块结构

```
services/
  arbitrage/
    mod.rs           # 模块入口
    types.rs         # 数据类型定义
    pair_manager.rs  # 套利对管理
    spread_calculator.rs  # 价差计算引擎
    signal_generator.rs   # 信号生成器
    executor.rs           # 执行引擎
    market_data.rs        # 市场数据获取

handlers/
  arbitrage.rs      # HTTP API 处理器
```

### 2.2 价差计算模式

支持三种模式：

| 模式 | 公式 | 适用场景 |
|------|------|----------|
| `ratio` | spread = price_a / price_b | 跨品种套利 |
| `percentage` | spread = (price_a - price_b) / price_b * 100 | 跨期套利 |
| `zscore` | z = (spread - mean) / std | 统计套利 |

Z-score 计算使用滑动窗口（默认 20 期），数据存于内存 `RingBuffer`。

### 2.3 信号生成逻辑

```
入口信号：
  - long_spread:  spread < historical_mean - z_score_entry * std
  - short_spread: spread > historical_mean + z_score_entry * std

出场信号：
  - 价差回归至 exit_threshold 区间内
  - 或持仓超过最大持仓时间

止损信号：
  - 价差继续扩大超过 2 * entry_threshold
```

### 2.4 双向执行策略

```
OrderExecutionStrategy:
  1. 先下流动性好的一侧
  2. 再下流动性差的一侧
  3. 如果时间差超过 500ms，重新计算价差
  4. 部分成交时等待成交确认后再下另一侧
```

### 2.5 数据存储

- **套利对配置**：PostgreSQL `arbitrage_pairs` 表
- **持仓记录**：PostgreSQL `arbitrage_positions` 表
- **信号历史**：PostgreSQL `arbitrage_signals` 表
- **实时价差**：Redis 缓存（TTL 1s）

## 3. API 设计

见 PRD-P3-F1-Arbitrage.md Section 4。

## 4. 测试策略

- 单元测试：价差计算、Z-score、信号生成（Mock 市场数据）
- 集成测试：模拟跨期套利完整流程
- 覆盖率目标：整体 70%，核心（spread_calculator/signal_generator）80%

## 5. 风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| 双侧订单部分成交 | 支持部分成交状态跟踪 |
| 市场流动性不足 | 下单前检查订单簿深度 |
| 数据源延迟 | 记录报价时间戳，延迟 > 1s 丢弃 |
| 价差剧烈波动 | 硬止损（价差扩大 2x 时强制平仓）|
