"""技术指标计算"""
import pandas as pd
import numpy as np
from typing import Optional

# talipp indicators
from talipp.indicators import RSI, MACD, BB, SMA, EMA


def compute_indicators(klines: list[dict]) -> dict:
    """从 K 线列表计算技术指标，返回 dict"""
    if not klines or len(klines) < 2:
        return {}

    df = pd.DataFrame(klines)
    close = df["close"].astype(float)
    high = df["high"].astype(float)
    low = df["low"].astype(float)
    volume = df["volume"].astype(float)

    result = {}

    # RSI(14)
    if len(close) >= 14:
        rsi = RSI(period=14, input_values=close.tolist())
        result["rsi_14"] = round(float(rsi[-1]), 2) if rsi else None

    # MACD (12, 26, 9)
    if len(close) >= 26:
        macd = MACD(
            fast_period=12,
            slow_period=26,
            signal_period=9,
            input_values=close.tolist(),
        )
        macd_val = macd[-1] if macd else None
        if macd_val:
            result["macd"] = round(float(macd_val.macd), 2)
            result["macd_signal"] = round(float(macd_val.signal), 2) if macd_val.signal else None
            result["macd_hist"] = round(float(macd_val.histogram), 2) if macd_val.histogram else None

    # SMA
    for period in [7, 25]:
        if len(close) >= period:
            sma = SMA(period=period, input_values=close.tolist())
            result[f"sma_{period}"] = round(float(sma[-1]), 2) if sma else None

    # EMA
    if len(close) >= 20:
        ema = EMA(period=20, input_values=close.tolist())
        result["ema_20"] = round(float(ema[-1]), 2) if ema else None

    # Bollinger Bands (20, 2)
    if len(close) >= 20:
        bb = BB(period=20, std_dev_mult=2.0, input_values=close.tolist())
        bb_val = bb[-1] if bb else None
        if bb_val:
            result["bb_upper"] = round(float(bb_val.ub), 2)
            result["bb_middle"] = round(float(bb_val.cb), 2)
            result["bb_lower"] = round(float(bb_val.lb), 2)

    # ATR(14) - manual True Range calculation
    if len(close) >= 14 and len(high) >= 14 and len(low) >= 14:
        tr_list = []
        for i in range(1, len(close)):
            high_low = high.iloc[i] - low.iloc[i]
            high_close = abs(high.iloc[i] - close.iloc[i - 1])
            low_close = abs(low.iloc[i] - close.iloc[i - 1])
            tr_list.append(max(high_low, high_close, low_close))
        if len(tr_list) >= 14:
            result["atr_14"] = round(float(np.mean(tr_list[-14:])), 2)

    # Volume MA
    if len(volume) >= 20:
        vol_sma = SMA(period=20, input_values=volume.tolist())
        result["volume_ma_20"] = round(float(vol_sma[-1]), 2) if vol_sma else None

    # 当前价格
    result["current_price"] = float(close.iloc[-1])

    return result


def signal_from_indicators(ind: dict) -> tuple[str, float]:
    """根据技术指标生成简单规则信号（DeepSeek 不可用时的降级）"""
    direction = "neutral"
    confidence = 0.5

    rsi = ind.get("rsi_14")
    macd_hist = ind.get("macd_hist")
    price = ind.get("current_price", 0)
    sma_7 = ind.get("sma_7")
    sma_25 = ind.get("sma_25")
    ema_20 = ind.get("ema_20")

    score = 0
    # RSI
    if rsi:
        if rsi > 70:
            score -= 1
        elif rsi < 30:
            score += 1
        elif rsi > 60:
            score += 0.5
        elif rsi < 40:
            score -= 0.5

    # MACD hist
    if macd_hist:
        if macd_hist > 0:
            score += 1
        else:
            score -= 1

    # MA 金叉/死叉
    if sma_7 and sma_25:
        if sma_7 > sma_25:
            score += 1
        else:
            score -= 1

    if ema_20 and price:
        if price > ema_20:
            score += 0.5
        else:
            score -= 0.5

    if score >= 2:
        direction = "long"
        confidence = 0.65
        signal = "buy"
    elif score <= -2:
        direction = "short"
        confidence = 0.65
        signal = "sell"
    else:
        direction = "neutral"
        confidence = 0.5
        signal = "neutral"

    return signal, confidence
