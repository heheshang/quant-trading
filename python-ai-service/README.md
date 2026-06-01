# Python AI Service

FastAPI 服务，为 quant-trading Rust 后端提供 AI 预测能力。

## 模块结构

```
python-ai-service/
├── main.py              # FastAPI 入口（REST + WebSocket）
├── config.py            # 环境变量配置
├── schemas.py           # Pydantic 请求/响应模型
├── deepseek_client.py   # DeepSeek API 客户端
├── binance_ws.py        # Binance K 线 WebSocket 客户端
├── features.py          # 技术指标计算（RSI/MACD/BB/MA/ATR）
├── requirements.txt
├── Dockerfile
└── .env.example
```

## 端口约定

- **容器内端口**: `8001`
- **宿主机端口**: `8002`（docker-compose 映射 `8002:8001`）
- **Rust 后端调用**: 通过 `http://ai-service:8001`（容器网络名 + 内部端口）

## 端点

### REST

| 端点 | 方法 | 说明 |
|------|------|------|
| `/health` | GET | 健康检查（DeepSeek + Binance 连接状态） |
| `/api/v1/klines/{symbol}` | GET | 拉取 Binance K 线（interval + limit） |
| `/api/v1/predict` | POST | AI 预测（接收 K 线或自动拉取） |

### WebSocket

| 端点 | 说明 |
|------|------|
| `/ws/predict/{symbol}?interval=1h` | 实时预测推送（K 线收线时触发） |

## 历史说明

- v0.1.0 (2026-05-23): 初版 mock NumPy 服务
- v1.0.0 (2026-06-01): 升级为 DeepSeek + BinanceWS 集成，删除原 `ai-service/` 顶层目录

## 本地开发

```bash
cd python-ai-service
pip install -r requirements.txt
cp .env.example .env
# 编辑 .env 填入 DEEPSEEK_API_KEY
uvicorn main:app --port 8001 --reload
```
