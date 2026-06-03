# E2E Tests — quant-trading

端到端测试套件。两层结构：

- **Node 冒烟** (`runner.mjs`) — 零依赖的 stdlib HTTP 脚本，19 个断言。CI 和本地都能用。
- **Playwright 套件** (`*.spec.ts`) — 完整业务场景，10 个 spec 文件，共 60+ 个 `test()` 用例。

## 目录

```
e2e-tests/
├── runner.mjs          # Node 冒烟入口 + 共享 helper (httpReq/expectStatus/expectJsonField/retryHelper)
├── helpers.ts          # Playwright-friendly 包装 + registerAndLogin() 缓存
├── auth.spec.ts        # 最小冒烟样例 (保留供新贡献者参考)
├── auth_full.spec.ts   # 9 cases: 注册/登录/改密/刷新/隔离
├── order.spec.ts       # 6 cases: 下单/查单/撤单
├── trigger_order.spec.ts  # 6 cases: 止损/止盈/OCO/TWAP
├── backtest.spec.ts    # 5 cases: 异步回测 + 轮询
├── ai_prediction.spec.ts  # 6 cases: AI 模型 + Rust/Python 双路径
├── strategy.spec.ts    # 6 cases: 策略 CRUD + 启停
├── portfolio.spec.ts   # 6 cases: 持仓/权益/绩效
├── risk.spec.ts        # 6 cases: 风控规则 + 暂停/恢复
├── market_data.spec.ts # 6 cases: K 线/行情/WS
├── export.spec.ts      # 6 cases: 导出 CSV/JSON
└── run.sh              # CI/本地 driver
```

## Helper API

| 名字 | 用途 |
|------|------|
| `httpReq(method, url, body, token)` | 裸 HTTP 请求，带超时和 JSON 解析 |
| `expectStatus(resp, allowed, label)` | 断言状态码在允许集合内 |
| `expectJsonField(resp, dottedPath, label, predicate?)` | 沿点路径校验 JSON 字段，可选 regex/函数断言 |
| `retryHelper(fn, opts)` | 指数退避重试，返回首个真值 |
| `registerAndLogin(page)` | 一次性注册新用户并缓存 token (worker 级别) |
| `getAuth()` | 获取当前缓存的 {token, userId, ...} |
| `api(page, method, path, body)` | Playwright request 风格封装，自动注入 Bearer |

## 环境变量

| 变量 | 默认 | 说明 |
|------|------|------|
| `E2E_BASE_URL` | `http://localhost:8080` | Rust 后端地址 |
| `AI_SERVICE_URL` | `http://localhost:8001` | Python AI 服务地址 |
| `E2E_TIMEOUT_MS` | `15000` | 单次 HTTP 请求超时 |
| `COMPOSE_PROJECT` | `quant-trading` | docker compose 项目名 |
| `HEALTH_TIMEOUT` | `120` | run.sh 健康检查等待秒数 |
| `KEEP_SERVICES` | `0` | run.sh 跑完后是否保留 docker 栈 |

## 快速运行

```bash
# 一键起栈 + 跑 E2E
npm run test:e2e

# 只跑 spec（假设服务在跑）
npm run test:e2e:specs

# 只跑 Node 冒烟
npm run test:e2e:smoke

# 清理栈
npm run test:e2e:stop

# 打开 HTML 报告
npm run test:e2e:report
```

## CI

`.github/workflows/ci.yml` 新增 `e2e-tests` job：

- 依赖 `backend-build` + `frontend-build`
- 启动 docker compose stack，等待 health 端点
- 运行 Playwright spec
- 上传 `e2e-report/` + `e2e-logs/` 为 artifact (保留 7 天)
- 清理栈

## 设计原则

1. **可失败但可观测** — 任何网络/服务异常都映射到合理的状态码断言；不允许 throw 出未捕获异常。
2. **状态共享但安全** — `registerAndLogin` 每个 worker 缓存一个用户；多个 spec 通过同一 token 加速，但读路径覆盖隔离。
3. **宽松断言** — 我们不绑定具体 JSON 形状 (避免后端重构雪崩)，只断言关键字段存在 + 状态码正确。
4. **冒烟先行** — `runner.mjs` 是最快反馈层 (1-2s)；Playwright 跑不动时先看冒烟。
