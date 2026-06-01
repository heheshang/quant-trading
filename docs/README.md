# 📚 项目文档总索引

> 最后生成时间: 2026-06-01 09:55:00
> 整理自 rust-software-dev 文档治理规范 v1.0

## 📊 文档统计

| 目录 | 文档数量 | 说明 |
|------|---------|------|
| requirements/ | 54 | T0-T1 PM 产出（PRD、T0 Checklist、Gherkin） |
| architecture/ | 56 | T2 TechLead 产出（ADR、TechDesign、Contract Review） |
| rfc/ | 0 | RFC 文档（draft/accepted/implemented/archived） |
| design/ | 36 | T3 Designer 产出（Design Spec、原型、UI Checklist） |
| api/ | 10 | T4.5+ 契约文档（OpenAPI、API 说明） |
| qa/ | 12 | T6 QA 产出（测试报告、Bug 清单、PIPELINE 报告） |
| operations/ | 1 | T8 DevOps 产出（部署指南） |
| guides/ | 6 | T7 TechWriter 产出（用户手册） |
| changelog/ | 1 | 版本变更记录 |
| t9/ | 34 | T9 最终评审（Final Review、Release Note、Sign-Off） |

**总计：210 个文件**

---

## 📑 按目录索引

### 🎯 requirements/ (T1 PM 产出)
- PRD-Binance-WS-Connector-20260517.md
- PRD-Dashboard.md
- PRD-Exchange-Integration.md
- PRD-Market-Data-Pipeline-20260517.md
- PRD-Missing-Features-20260519.md
- PRD-P0-F1-API-Signing-20260521.md
- PRD-P0-F2-Daily-Loss-Limit-20260521.md
- PRD-P1-F5-Kline-Partition-20260521.md
- PRD-P2-F3-Backtest-Price-Limit-20260521.md
- PRD-P2-F4-Export-20260521.md
- PRD-P3-F1-Arbitrage.md
- PRD-P3-F2-Grid-Trading-20260521.md
- PRD-P3-F3-AI-Quant-20260522.md
- PRD-Phase4-Historical-Kline-Persistence-20260517.md
- PRD-Phase4-Risk-Management-20260521.md
- PRD-Phase5-Stress-Testing-20260521.md
- PRD-Position-Alert.md
- PRD-Review-Workflow.md
- PRD-Trigger-Order.md
- PRD-WebSocket.md
- PRD-backtest-engine.md
- PRD-kline-management.md
- PRD-market-depth-ticker.md
- PRD-market-module.md
- PRD-portfolio.md
- PRD-strategy-management.md
- PRD-strategy-sandbox-mvp.md
- PRD-trade-execution.md
- PRD-trading-execution.md
- PRD.md
- T0-Checklist-Binance-WS-Connector-20260517.md
- T0-Checklist-P0-F1-API-Signing-20260521.md
- T0-Checklist-P0-F2-Daily-Loss-Limit-20260521.md
- T0-Checklist-P1-F5-Partition-20260521.md
- T0-Checklist-P2-F3-Price-Limit-20260521.md
- T0-Checklist-P2-F4-Export-20260521.md
- T0-Checklist-P3-F2-Grid-20260521.md
- T0-Checklist-P3-F3-AI-Quant-20260522.md
- T0-Checklist-Phase4-Risk-20260521.md
- T0-Checklist-Phase5-Stress-Testing-20260521.md
- T0-Checklist-market-data-pipeline-20260517.md
- T0-Checklist.md
- T1-Checklist-P0-F1-API-Signing-20260521.md
- T1-Checklist-P3-F2-Grid-20260521.md
- T1-Checklist-P3-F3-AI-Quant-20260522.md
- T1-Checklist-Phase4-Risk-20260521.md
- T1-Checklist-Phase5-Stress-Testing-20260521.md
- kline-management.feature
- market-depth-ticker.feature
- market-module.feature
- strategy-management-prd.md
- strategy-management.feature
- trade-execution.feature
- trading-execution.feature

### 🏗️ architecture/ (T2 TechLead 产出)
- ADR-001-rust-axum-backend.md
- ADR-002-websocket-realtime.md
- ADR-003-strategy-engine-sandbox.md
- ADR-004-backtest-engine.md
- ADR-005-Market-Data-Pipeline.md
- ADR-005-market-data-pipeline.md
- ADR-006-Binance-WS-Connector.md
- ADR-006-matching-engine.md
- ADR-007-Historical-Kline-Persistence.md
- ADR-007-backtest-engine-detailed.md
- ADR-008-backtest-engine-frontend-alignment.md
- ADR-009-portfolio-module.md
- ADR-010-Risk-Rule-Engine.md
- ADR-010-order-management.md
- ADR-011-TradingFront-WS.md
- ADR-012-API-Signing-Architecture.md
- ADR-012-api-signing-authentication.md
- ADR-013-Conditional-Order-Engine.md
- ADR-013-risk-manager.md
- ADR-014-Alert-Notification-Service.md
- ADR-014-emergency-risk.md
- ADR-015-P3-F3-AI-Quant-Module.md
- ADR-015-Risk-ATR-Trailing-Stop.md
- ADR-016-Strategy-State-Control.md
- ADR-017-Risk-Dashboard-Frontend.md
- ADR-018-Stress-Testing-Tool.md
- ADR-019-Binance-API-Signing.md
- ADR-020-Kline-Partition.md
- ADR-021-Arbitrage-Architecture.md
- ADR-021-Grid-Trading-Architecture.md
- ADR-MARKET-DEPTH-TICKER.md
- ADR-STRATEGY-MGMT.md
- Architecture-Health-Report-20260517.md
- Architecture-Health-Report-20260519.md
- Architecture-Health-Report-20260521.md
- Contract-Review-AI-Quant.md
- Contract-Review-Market.md
- Contract-Review-Order.md
- Contract-Review.md
- README.md
- REVIEW-OrderManagement-T9.md
- REVIEW-Portfolio-T9-final.md
- REVIEW-TradingExecution-T9.md
- T2-Architecture-Binance-WS-Connector-20260517.md
- T2-Architecture-Historical-Kline-Persistence.md
- TechDesign-Backend-P0-F1-API-Signing-20260519.md
- TechDesign-Frontend-P0-F1-API-Signing-20260519.md
- architecture-overview.html
- backend-handlers-order-supplement.rs
- backtest-api-design-v1.1.md
- data-model.md
- frontend-api-order-supplement.ts
- frontend-backend-boundary.md
- frontend-types-order-supplement.ts
- reference-code-skeleton-db-handler-fix.md
- risk-analysis.md

### 📋 rfc/ (RFC 文档)
rfc/draft/ — 草稿状态
rfc/accepted/ — 已接受
rfc/implemented/ — 已实施
rfc/archived/ — 已归档

### 🎨 design/ (T3 Designer 产出)
- ADR-022-AI-Quant-Module.md
- ApiKeyManagement_UI-Checklist.md
- Arbitrage_UI-Checklist.md
- Backtest_Checklist.md
- Backtest_Prototype.html
- Design_ApiKeyManagement.md
- Design_Arbitrage.md
- Design_Backtest_UI.md
- Design_KlineImport.md
- Design_KlineList.md
- Design_OrderManagement.md
- Design_Portfolio.md
- Design_QuantTrading_Redesign.md
- Design_QuantTrading_UI.md
- Design_Quant_Trading_System.md
- Design_RiskDashboard.md
- Design_StrategyManagement.md
- Design_SystemAdmin.md
- Design_TradingExecution.md
- Design_TradingFront_WS.md
- KlineList_UI-Checklist.md
- P1-F1-KDJ-Indicator-PRD.md
- PRD-market-data-pipeline-implementation.md
- Portfolio_UI-Checklist.md
- QuantTrading_Prototype.html
- RiskDashboard_UI-Checklist.md
- SPEC-024-AI-Realtime-Prediction.md
- StrategyManagement_Checklist.md
- StrategyManagement_Prototype.html
- SystemAdmin_UI-Checklist.md
- T4.5_UI-Walkthrough_Report.md
- TradingExecution_Checklist.md
- prototype_StrategyManagement.html
- prototype_order_management.html
- prototype_portfolio.html
- prototype_trading_execution.html

### 🔌 api/ (T4.5+ 契约文档)
- API-AI-Quant.md
- API-Arbitrage.md
- API-Dashboard.md
- API-Trigger-Order.md
- KlineManagement_API.md
- Portfolio_API.md
- README.md
- TradingExecution_API.md
- backtest-api.md
- strategy-api.md

### 🔍 qa/ (T6 QA 产出)
- KlineManagement_QA-Report.md
- OrderManagement_QA-Report.md
- PIPELINE-REPORT-Backtest.md
- PIPELINE-REPORT-Order-Management.md
- PIPELINE-REPORT-market-view.md
- PIPELINE-REPORT.md
- StrategyManagement_QA-Report.md
- StrategyManagement_UI-Checklist.md
- T1_TradingFront_WS_20260518.md
- T3_TradingFront_WS_20260518.md
- tc_a_results.json
- tc_c_results.json

### 🚀 operations/ (T8 DevOps 产出)
- Deployment-Log-20260521.md

### 📖 guides/ (T7 TechWriter 产出)
- KlineManagement_User-Guide.md
- Portfolio_User-Guide.md
- TradingExecution_User-Guide.md
- backtest-metrics.md
- backtest-user-guide.md
- strategy-user-guide.md

### 📝 changelog/ (版本变更)
- CHANGELOG.md

### ✅ t9/ (T9 最终评审)
- Final-Review-Checklist-20260521.md
- Final-Review-Checklist-P0-F1-20260521.md
- Final-Review-Checklist-P0-F2-20260521.md
- Final-Review-Checklist-P1-F4-20260521.md
- Final-Review-Checklist-P1-F5-20260521.md
- Final-Review-Checklist-P1-F6-20260521.md
- Final-Review-Checklist-P2-F3-20260522.md
- Final-Review-Checklist-P3-F1-20260522.md
- Final-Review-Checklist-P3-F2-20260521.md
- Final-Review-Checklist-P4-20260521.md
- Final-Review-Checklist-P5-20260521.md
- Final-Review-Checklist-Strategy-Management-20260522.md
- Release-Note-20260521.md
- Release-Note-P0-F1-20260521.md
- Release-Note-P0-F2-20260521.md
- Release-Note-P1-F4-20260521.md
- Release-Note-P1-F5-20260521.md
- Release-Note-P1-F6-20260521.md
- Release-Note-P2-F3-20260522.md
- Release-Note-P3-F1-20260522.md
- Release-Note-P3-F2-20260521.md
- Release-Note-P4-20260521.md
- Release-Note-P5-20260521.md
- Release-Note-Strategy-Management-20260522.md
- Sign-Off-Record-20260521.md
- Sign-Off-Record-P0-F1-20260521.md
- Sign-Off-Record-P0-F2-20260521.md
- Sign-Off-Record-P1-F4-20260521.md
- Sign-Off-Record-P1-F5-20260521.md
- Sign-Off-Record-P1-F6-20260521.md
- Sign-Off-Record-P3-F2-20260521.md
- Sign-Off-Record-P4-20260521.md
- Sign-Off-Record-P5-20260521.md
- T9-Release-Readiness-Checklist.md
