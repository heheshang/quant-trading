# T1 PRD 评审 Checklist — 交易前端 WebSocket 补全

> 功能名称：TradingFront-WS
> 日期：2026-05-18
> 评审人：PM / Tech-Lead
> 状态：✅ 通过

---

## P0 阻断项

### ✅ PRD 覆盖完整
- PRD-trade-execution.md (v1.0) ✅ 覆盖 US-TE-01~10
- US-TE-01 限价/市价下单 ✅
- US-TE-02 撤销委托单 ✅
- US-TE-03 查看当前委托 ✅
- US-TE-04 撤销全部委托 ✅
- US-TE-05 查看成交历史 ✅
- US-TE-06 查看持仓 ✅
- US-TE-07 模拟撮合引擎 ✅ (matching_engine.rs)
- US-TE-08 浮动盈亏计算 ✅
- US-TE-09 交易模式切换 ✅
- US-TE-10 风险控制 ✅

### ✅ 目标用户明确
量化交易者（手动下单 + 策略信号触发），MVP 为模拟交易

### ✅ 核心功能范围清晰
前端 WebSocket 实时行情 + 完整交易 API 链路

### ✅ 依赖关系已识别
- 后端 `handlers/order.rs` 已有完整 CRUD + cancel
- `api/order.ts` ✅ 已存在完整调用
- `api/trades.ts` ✅ 已存在
- `types/order.ts` ✅ 已存在完整类型定义
- 前端组件 `OrderForm.vue`, `OrderList.vue`, `PositionPanel.vue`, `TradeRecordTab.vue` ✅ 均已实现
- 缺口：TradingView.vue 的 wsTimer 模拟需替换为真实 WS

---

## P1 建议项

- ADR-010 D5（TradeWsHub）尚未实现，需在后端补充
- 后端 `/ws/trade` 路由未单独暴露，复用现有 `/ws` 路由（按 symbol 订阅 channel）
- wsTimer 模拟价格替换为真实 WS 订阅

---

## T1 结论

**✅ 通过** — 可进入 T2 技术设计。

### T2 交付物
- `docs/adr/ADR-TradingFront-WS.md` — 前后端 WS 集成方案
- `docs/design/Design_TradingFront_WS.md` — 交易前端 WS 详细设计