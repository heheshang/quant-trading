"""Binance WebSocket K 线订阅"""
import asyncio
import json
import httpx
from datetime import datetime
from typing import Optional, Callable, Awaitable
import websockets
import websockets.client
from config import BINANCE_WS_URL, BINANCE_REST_URL


class BinanceWSClient:
    """Binance K 线 WebSocket 客户端，支持重连"""

    def __init__(
        self,
        symbol: str = "btcusdt",
        interval: str = "1h",
        on_kline: Optional[Callable[[dict], Awaitable[None]]] = None,
    ):
        self.symbol = symbol.lower()
        self.interval = interval
        self.on_kline = on_kline
        self._running = False
        self._task: Optional[asyncio.Task] = None
        self._reconnect_delay = 1
        self._max_reconnect_delay = 60

    @property
    def ws_url(self) -> str:
        return f"{BINANCE_WS_URL}/{self.symbol}@kline_{self.interval}"

    async def _fetch_recent_klines(self, limit: int = 100) -> list[dict]:
        """通过 REST API 获取最近 K 线"""
        url = f"{BINANCE_REST_URL}/klines"
        params = {"symbol": self.symbol.upper(), "interval": self.interval, "limit": limit}
        async with httpx.AsyncClient(timeout=10) as client:
            resp = await client.get(url, params=params)
            resp.raise_for_status()
            data = resp.json()

        klines = []
        for d in data:
            klines.append({
                "open_time": d[0],
                "open": d[1],
                "high": d[2],
                "low": d[3],
                "close": d[4],
                "volume": d[5],
                "close_time": d[6],
                "quote_volume": d[7],
                "trades": d[8],
                "taker_buy_volume": d[9],
            })
        return klines

    async def _ws_loop(self):
        while self._running:
            try:
                async with websockets.connect(self.ws_url) as ws:
                    self._reconnect_delay = 1  # 重置重连延迟
                    while self._running:
                        msg = await ws.recv()
                        payload = json.loads(msg)

                        if payload.get("e") == "kline":
                            kline = payload["k"]
                            kline_data = {
                                "open_time": kline["t"],
                                "open": kline["o"],
                                "high": kline["h"],
                                "low": kline["l"],
                                "close": kline["c"],
                                "volume": kline["v"],
                                "close_time": kline["T"],
                                "is_closed": kline["x"],  # K 线是否收线
                                "symbol": self.symbol.upper(),
                                "interval": self.interval,
                            }
                            if self.on_kline and kline_data["is_closed"]:
                                await self.on_kline(kline_data)
            except websockets.exceptions.ConnectionClosed:
                pass
            except Exception as e:
                print(f"[BinanceWS] 连接异常: {e}, {self._reconnect_delay}s 后重连...")

            if self._running:
                await asyncio.sleep(self._reconnect_delay)
                self._reconnect_delay = min(self._reconnect_delay * 2, self._max_reconnect_delay)

    def start(self):
        self._running = True
        self._task = asyncio.create_task(self._ws_loop())

    def stop(self):
        self._running = False
        if self._task:
            self._task.cancel()
            self._task = None


async def get_recent_klines(symbol: str = "BTCUSDT", interval: str = "1h", limit: int = 100) -> list[dict]:
    """同步获取最近 K 线（工具函数）"""
    client = BinanceWSClient(symbol.lower(), interval)
    return await client._fetch_recent_klines(limit)