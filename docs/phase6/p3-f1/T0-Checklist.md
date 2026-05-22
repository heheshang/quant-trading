# T0 Checklist — P3-F1 套利模块

## T0: 需求分析与 PRD ✅
- [x] PRD 文档完成
- [x] API 设计（套利对 CRUD + 监控端点）
- [x] 数据模型（arbitrage_pairs / positions / signals 三表）
- [x] 三种套利类型信号逻辑明确定义
- [x] 工时估算：7天

## T1: 架构设计（待开始）
- [ ] ADR 架构文档
- [ ] 技术选型决策（价差计算、信号生成、双向执行）
- [ ] 模块划分（pair_manager / spread_calculator / signal_generator / executor）

## T2: 核心实现
- [ ] arbitrage 模块创建
- [ ] 三种套利类型核心逻辑
- [ ] CRUD API + 监控端点
- [ ] 单元测试（≥ 70%）

## T3: 前后端集成
- [ ] 前端套利对管理页面
- [ ] 价差监控图表
- [ ] 持仓状态面板

## T4: 测试与修复
- [ ] 后端 281 tests → 新增测试
- [ ] 前端集成测试
- [ ] Bug 修复

## T5: 代码审查
- [ ] Code Review
- [ ] 覆盖率检查（整体70%/核心80%）

## T6: 自动化门禁
- [ ] CI/CD pipeline

## T7-T9: 文档与发布
- [ ] Final Review
- [ ] Sign-Off
- [ ] Release Note
