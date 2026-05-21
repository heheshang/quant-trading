# T0 Checklist — P2-F4 订单/账单 Excel 导出

## 1. 功能理解
- [x] 目标：导出订单历史、成交记录、账户账单为 CSV/Excel
- [x] 复用现有 list_orders / list_trades / list_positions / get_account API
- [x] 新增 export 格式参数，复用现有分页/过滤逻辑

## 2. 技术方案
- [x] 后端：新增 `GET /api/v1/exports/orders` + `/exports/trades` + `/exports/account`
- [x] Query 参数复用 ListOrdersQuery / ListTradesQuery + `format=csv|xlsx`
- [x] Excel 库：Rust `calamine`（支持 .xlsx，无需系统 Excel）
- [x] CSV 库：Rust `csv` crate
- [x] 前端：订单管理页 + 成交记录页 + 账户页面添加「导出」按钮
- [x] 前端下载：Blob + URL.createObjectURL 触发下载

## 3. 风险评估
- [x] 数据量大（10万+行）：分页导出，每页5000条，流式写入
- [x] Excel 格式：calamine 支持 xlsx，无需系统 Excel
- [x] 前端内存：Blob URL 方式，大文件分片

## 4. API 设计
```
GET /api/v1/exports/orders?format=csv|xlsx&status=&symbol=&start_time=&end_time=
GET /api/v1/exports/trades?format=csv|xlsx&symbol=&start_time=&end_time=
GET /api/v1/exports/account?format=csv|xlsx
```

## 5. 数据模型（复用现有）
- OrderResponse: id, symbol, side, price, quantity, status, created_at...
- TradeResponse: id, order_id, symbol, price, quantity, side, fee, realized_pnl, created_at...
- AccountResponse: total_equity, available, position_value, unrealized_pnl...

## 6. 前端入口
- 订单管理页 (OrderManagementView.vue)：导出按钮
- 成交记录（tab 或独立页）：导出按钮
- 账户/资产页 (PortfolioView.vue)：导出账单按钮

## 7. 测试策略
- 后端：单元测试验证 CSV/xlsx 内容正确性
- 前端：Vitest 测试导出按钮存在 + download 触发

## 8. 依赖
- Backend: `calamine = "0.26"`（Excel）, `csv = "1.3"`
- Frontend: 无新增（纯前端 Blob 下载）

## 9. 验收标准
- [ ] GET /api/v1/exports/orders?format=csv → 返回标准 CSV 文件
- [ ] GET /api/v1/exports/orders?format=xlsx → 返回有效 xlsx 文件
- [ ] 前端「导出 CSV」和「导出 Excel」按钮可用
- [ ] 导出内容包含完整字段，无截断
- [ ] 大数据量（>5000条）分页流畅，无 OOM
