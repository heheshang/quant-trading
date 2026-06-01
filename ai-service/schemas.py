"""Pydantic 请求/响应模型"""
from datetime import datetime
from typing import Optional, Literal
from pydantic import BaseModel, Field


class KlineData(BaseModel):
    open_time: int
    open: str
    high: str
    low: str
    close: str
    volume: str
    close_time: int
    quote_volume: str = ""
    trades: int = 0
    taker_buy_volume: str = ""


class KlinesRequest(BaseModel):
    symbol: str = "BTCUSDT"
    interval: str = "1h"
    limit: int = Field(default=100, ge=10, le=1000)


class KlinesResponse(BaseModel):
    symbol: str
    interval: str
    klines: list[KlineData]


class PredictRequest(BaseModel):
    symbol: str = "BTCUSDT"
    interval: str = "1h"
    klines: Optional[list[KlineData]] = None  # 可选，不传则自动拉取


SignalLiteral = Literal["strong_buy", "buy", "neutral", "sell", "strong_sell"]
DirectionLiteral = Literal["long", "short", "neutral"]


class PredictResponse(BaseModel):
    symbol: str
    interval: str
    direction: DirectionLiteral
    confidence: float = Field(ge=0.0, le=1.0)
    signal: SignalLiteral
    analysis: str
    price_target: Optional[float] = None
    indicators: Optional[dict] = None  # 技术指标快照
    generated_at: datetime


class WsPredictMessage(BaseModel):
    type: Literal["prediction", "heartbeat", "error"] = "prediction"
    symbol: str
    interval: str
    direction: DirectionLiteral
    confidence: float
    signal: SignalLiteral
    analysis: str
    price_target: Optional[float] = None
    indicators: Optional[dict] = None
    kline_close_time: int
    generated_at: datetime
    error: Optional[str] = None


class HealthResponse(BaseModel):
    status: str
    deepseek_connected: bool
    binance_connected: bool
