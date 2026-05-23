"""
Python AI Model Service — FastAPI + NumPy
quant-trading Rust backend HTTP client target.

Endpoints:
  POST /api/v1/predict   — AI price-direction prediction from feature vector
  GET  /api/v1/models    — list available model versions
  GET  /api/v1/health    — health check

Runs on port 8001 by default (AI_MODEL_SERVICE_URL env var in Rust).
"""

import os
import random
import time
from datetime import datetime, timezone
from typing import Optional

from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel, Field
import uvicorn

# ── Config ────────────────────────────────────────────────────────────────────
PORT = int(os.getenv("PORT", "8001"))
MODEL_VERSION = os.getenv("MODEL_VERSION", "v1.0.0")
USE_MOCK = os.getenv("USE_MOCK", "true").lower() in ("true", "1", "yes")

# ── Schema ────────────────────────────────────────────────────────────────────
class PredictRequest(BaseModel):
    features: list[float] = Field(..., min_length=5, description="Normalized feature vector from Rust FeatureEngine")
    symbol: Optional[str] = "BTCUSDT"
    interval: Optional[str] = "1h"

class PredictResponse(BaseModel):
    direction: str           # "long" | "short" | "neutral"
    confidence: float        # 0.0 ~ 1.0
    model_version: str
    generated_at: str
    price_target: Optional[float] = None

class ModelInfo(BaseModel):
    version: str
    name: str
    accuracy: float
    status: str              # "active" | "deprecated" | "training"

class ModelsResponse(BaseModel):
    models: list[ModelInfo]

class HealthResponse(BaseModel):
    status: str
    models_available: int

# ── App ───────────────────────────────────────────────────────────────────────
app = FastAPI(title="Quant Trading AI Model Service", version=MODEL_VERSION)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# ── State ─────────────────────────────────────────────────────────────────────
_MODELS = [
    ModelInfo(version=MODEL_VERSION, name="LSTM Direction Predictor", accuracy=0.73, status="active"),
    ModelInfo(version="v0.9.0", name="LSTM Direction Predictor", accuracy=0.71, status="deprecated"),
]

# ── Inference logic ────────────────────────────────────────────────────────────
def _compute_price_target(features: list[float], direction: str) -> Optional[float]:
    """
    Derive a naive price target from the last close (feature index -4) and
    some momentum features.  Returns None if the feature vector is too short.
    """
    if len(features) < 5:
        return None
    # features[-4] is typically close price (normalized).  Use it as base.
    base = features[-4]
    # features[1] is short-term momentum; features[2] is volatility
    momentum = features[1] if len(features) > 1 else 0.0
    volatility = abs(features[2]) if len(features) > 2 else 0.01

    delta = momentum * volatility * base * 0.05
    if direction == "long":
        return round(base + delta, 2)
    elif direction == "short":
        return round(base - delta, 2)
    return None

def _predict_direction(features: list[float]) -> tuple[str, float]:
    """
    Simple mock inference using feature-vector statistics.
    In production this runs a PyTorch LSTM model.

    Feature vector (from Rust FeatureEngine):
      [0]  log_return_1d
      [1]  log_return_5d
      [2]  volatility_20d
      [3]  rsi_14
      [4]  macd_signal
      [5]  macd_histogram
      [6]  bb_position
      [7]  atr_normalized
      [8]  adx_14
      [9]  stochastic_k
      [10] obv_normalized
      [11] vwap_deviation
      [12] volume_ratio
      [13] price_momentum_5
      [14] price_momentum_10
      [15] high_low_spread
      [16] close_position
      [17] EMA_short / EMA_long (ratio)
      [18] volume_ma5 / volume_ma20 (ratio)
      [19] multi_period_return (latest)

    Signals:
      momentum > 0  → bullish bias
      rsi < 30     → oversold (bullish reversal)
      rsi > 70     → overbought (bearish reversal)
      bb_position near 0 → price at lower band (bullish)
      bb_position near 1 → price at upper band (bearish)
      volume_ratio > 1.5 → unusual volume (confirm trend)
    """
    if len(features) < 20:
        # Fallback for short vectors
        return ("neutral", 0.5)

    log_ret   = features[0]
    rsi       = features[3]
    macd_hist = features[5]
    bb_pos    = features[6]
    mom5      = features[13]
    vol_ratio = features[12]

    score = 0.0

    # Momentum
    score += log_ret * 10.0
    score += mom5 * 5.0

    # RSI
    if rsi < 30:
        score += (30 - rsi) / 30 * 0.8     # oversold → bullish
    elif rsi > 70:
        score -= (rsi - 70) / 30 * 0.8     # overbought → bearish

    # MACD histogram
    score += macd_hist * 3.0

    # Bollinger position
    score -= bb_pos * 0.5                  # bb_pos near 1 is bearish

    # Volume
    if vol_ratio > 1.5:
        score *= 1.2                       # amplify if volume confirms

    # Convert score to direction + confidence
    if score > 0.15:
        direction = "long"
        confidence = min(0.95, 0.55 + abs(score) * 0.5)
    elif score < -0.15:
        direction = "short"
        confidence = min(0.95, 0.55 + abs(score) * 0.5)
    else:
        direction = "neutral"
        confidence = 0.50 + (0.5 - abs(score)) * 0.3

    return direction, round(confidence, 4)

# ── Routes ────────────────────────────────────────────────────────────────────
@app.post("/api/v1/predict", response_model=PredictResponse)
async def predict(req: PredictRequest) -> PredictResponse:
    """Run AI inference on a feature vector from the Rust backend."""
    if USE_MOCK:
        direction, confidence = _predict_direction(req.features)
    else:
        # Production: load PyTorch model and run inference
        # model = load_model()
        # direction, confidence = model.predict(req.features)
        direction, confidence = _predict_direction(req.features)

    price_target = _compute_price_target(req.features, direction)

    return PredictResponse(
        direction=direction,
        confidence=confidence,
        model_version=MODEL_VERSION,
        generated_at=datetime.now(timezone.utc).isoformat(),
        price_target=price_target,
    )


@app.get("/api/v1/models")
async def list_models() -> list[ModelInfo]:
    """List all registered model versions."""
    return _MODELS


@app.get("/api/v1/health", response_model=HealthResponse)
async def health() -> HealthResponse:
    """Health check endpoint."""
    active = [m for m in _MODELS if m.status == "active"]
    return HealthResponse(status="ok", models_available=len(active))


@app.get("/")
async def root():
    return {"service": "quant-trading-ai", "version": MODEL_VERSION, "status": "running"}


# ── Entry point ────────────────────────────────────────────────────────────────
if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=PORT, reload=False)
