"""Python AI 预测服务 — FastAPI 入口"""
import asyncio
import json
from datetime import datetime, timezone
from contextlib import asynccontextmanager
from typing import Optional

from fastapi import FastAPI, WebSocket, WebSocketDisconnect, HTTPException
from fastapi.middleware.cors import CORSMiddleware

from config import AI_SERVICE_PORT, AI_SERVICE_HOST, PREDICTION_TIMEOUT_SECS
from schemas import (
    HealthResponse, KlinesResponse, PredictRequest, PredictResponse,
    WsPredictMessage, KlineData
)
from deepseek_client import DeepSeekClient
from binance_ws import BinanceWSClient, get_recent_klines
from features import compute_indicators, signal_from_indicators


# ─── 全局状态 ──────────────────────────────────────────────
deepseek_client: Optional[DeepSeekClient] = None
ws_clients: dict[str, list[WebSocket]] = {}
ws_binance: dict[str, BinanceWSClient] = {}
_connected = {"deepseek": False, "binance": False}


# ─── 生命周期 ──────────────────────────────────────────────
@asynccontextmanager
async def lifespan(app: FastAPI):
    global deepseek_client
    try:
        deepseek_client = DeepSeekClient()
        _connected["deepseek"] = True
    except Exception as e:
        print(f"[启动] DeepSeek 客户端初始化: {e}")
        deepseek_client = None
    yield
    for client in ws_binance.values():
        client.stop()
    ws_binance.clear()
    ws_clients.clear()


# ─── App ──────────────────────────────────────────────────
app = FastAPI(title="AI Quant Prediction Service", version="1.0.0", lifespan=lifespan)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:5173", "http://localhost:8081", "http://localhost:80"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


# ─── 辅助 ──────────────────────────────────────────────────
async def do_predict(symbol: str, interval: str, klines: list[dict]) -> PredictResponse:
    """执行一次完整预测流程"""
    indicators = compute_indicators(klines)

    direction: str = "neutral"
    confidence: float = 0.5
    signal: str = "neutral"
    analysis: str = "技术指标降级信号（DeepSeek 不可用）"
    price_target: Optional[float] = None

    if deepseek_client:
        try:
            result = await deepseek_client.analyze_klines(
                symbol=symbol,
                interval=interval,
                klines_data=klines,
                indicators=indicators,
            )
            direction = result.get("direction", "neutral")
            confidence = float(result.get("confidence", 0.5))
            signal = result.get("signal", "neutral")
            analysis = result.get("analysis", "")
            price_target = result.get("price_target")
        except Exception as e:
            print(f"[预测] DeepSeek 调用失败，降级规则信号: {e}")
            signal, confidence = signal_from_indicators(indicators)
            direction = "long" if signal in ("buy", "strong_buy") else "short" if signal in ("sell", "strong_sell") else "neutral"
            analysis = f"DeepSeek 调用失败 ({e})，使用规则信号 {signal} 作为降级"
    else:
        signal, confidence = signal_from_indicators(indicators)
        direction = "long" if signal in ("buy", "strong_buy") else "short" if signal in ("sell", "strong_sell") else "neutral"
        analysis = "DeepSeek API Key 未配置，使用规则信号"

    return PredictResponse(
        symbol=symbol,
        interval=interval,
        direction=direction,  # type: ignore[arg-type]
        confidence=confidence,
        signal=signal,  # type: ignore[arg-type]
        analysis=analysis,
        price_target=price_target,
        indicators=indicators,
        generated_at=datetime.now(timezone.utc),
    )


# ─── REST 端点 ─────────────────────────────────────────────

@app.get("/health", response_model=HealthResponse)
async def health():
    return HealthResponse(
        status="ok",
        deepseek_connected=deepseek_client is not None and _connected["deepseek"],
        binance_connected=_connected["binance"],
    )


@app.get("/api/v1/klines/{symbol}", response_model=KlinesResponse)
async def get_klines(symbol: str, interval: str = "1h", limit: int = 100):
    symbol = symbol.upper()
    try:
        klines = await asyncio.wait_for(
            get_recent_klines(symbol.lower(), interval, limit),
            timeout=10.0
        )
    except asyncio.TimeoutError:
        raise HTTPException(status_code=504, detail="Binance API 超时")
    except Exception as e:
        raise HTTPException(status_code=502, detail=f"Binance API 错误: {e}")
    return KlinesResponse(symbol=symbol, interval=interval, klines=klines)


@app.post("/api/v1/predict", response_model=PredictResponse)
async def predict(req: PredictRequest):
    symbol = req.symbol.upper()
    interval = req.interval

    if not req.klines:
        try:
            klines = await asyncio.wait_for(
                get_recent_klines(symbol.lower(), interval, limit=100),
                timeout=10.0
            )
        except asyncio.TimeoutError:
            raise HTTPException(status_code=504, detail="获取K线数据超时")
        except Exception as e:
            raise HTTPException(status_code=502, detail=f"获取K线数据失败: {e}")
    else:
        klines = [k.model_dump() for k in req.klines]

    return await asyncio.wait_for(do_predict(symbol, interval, klines), timeout=float(PREDICTION_TIMEOUT_SECS))


# ─── WebSocket 端点 ────────────────────────────────────────

@app.websocket("/ws/predict/{symbol}")
async def ws_predict(websocket: WebSocket, symbol: str, interval: str = "1h"):
    symbol = symbol.upper()
    key = f"{symbol}:{interval}"

    await websocket.accept()

    if key not in ws_clients:
        ws_clients[key] = []
    ws_clients[key].append(websocket)

    try:
        # 首次连接：立即推送一次预测
        try:
            klines = await asyncio.wait_for(
                get_recent_klines(symbol.lower(), interval, limit=100),
                timeout=10.0
            )
        except Exception:
            klines = []

        if klines:
            resp = await do_predict(symbol, interval, klines)
            await websocket.send_json(WsPredictMessage(
                type="prediction",
                **resp.model_dump(),
                kline_close_time=klines[-1]["close_time"] if klines else 0,
            ).model_dump())

        # 启动 Binance WS 订阅
        if key not in ws_binance:
            ws_binance[key] = BinanceWSClient(
                symbol=symbol.lower(),
                interval=interval,
                on_kline=lambda k: _on_kline(key, k),
            )
            ws_binance[key].start()
            _connected["binance"] = True

        # 保持连接，接收心跳/刷新
        while True:
            data = await websocket.receive_text()
            msg = json.loads(data)
            if msg.get("type") == "ping":
                await websocket.send_json({"type": "pong", "timestamp": datetime.now(timezone.utc).isoformat()})
            elif msg.get("type") == "refresh":
                try:
                    klines = await asyncio.wait_for(
                        get_recent_klines(symbol.lower(), interval, limit=100),
                        timeout=10.0
                    )
                except Exception:
                    klines = []
                if klines:
                    resp = await do_predict(symbol, interval, klines)
                    await websocket.send_json(WsPredictMessage(
                        type="prediction",
                        **resp.model_dump(),
                        kline_close_time=klines[-1]["close_time"] if klines else 0,
                    ).model_dump())

    except WebSocketDisconnect:
        pass
    finally:
        if key in ws_clients:
            ws_clients[key] = [w for w in ws_clients[key] if w != websocket]


async def _on_kline(key: str, kline: dict):
    """新 K 线收线时，触发预测并推送给所有订阅者"""
    symbol, interval = key.split(":", 1)
    try:
        klines = await asyncio.wait_for(
            get_recent_klines(symbol.lower(), interval, limit=100),
            timeout=10.0
        )
        resp = await do_predict(symbol, interval, klines)
        msg = WsPredictMessage(
            type="prediction",
            **resp.model_dump(),
            kline_close_time=kline.get("close_time", 0),
        )
        payload = json.dumps(msg.model_dump(), default=str)

        if key in ws_clients:
            dead = []
            for ws in ws_clients[key]:
                try:
                    await ws.send_text(payload)
                except Exception:
                    dead.append(ws)
            for ws in dead:
                ws_clients[key].remove(ws)
    except Exception as e:
        print(f"[_on_kline] 预测推送失败: {e}")
        error_msg = json.dumps({
            "type": "error",
            "symbol": symbol,
            "interval": interval,
            "error": str(e),
            "generated_at": datetime.now(timezone.utc).isoformat(),
        })
        for ws in ws_clients.get(key, []):
            try:
                await ws.send_text(error_msg)
            except Exception:
                pass


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host=AI_SERVICE_HOST, port=AI_SERVICE_PORT)
