"""配置管理"""
import os
from dotenv import load_dotenv

load_dotenv()

DEEPSEEK_API_KEY = os.getenv("DEEPSEEK_API_KEY", "")
DEEPSEEK_BASE_URL = os.getenv("DEEPSEEK_BASE_URL", "https://api.deepseek.com/v1")
DEEPSEEK_MODEL = os.getenv("DEEPSEEK_MODEL", "deepseek-chat")

BINANCE_WS_URL = os.getenv("BINANCE_WS_URL", "wss://stream.binance.com:9443/ws")
BINANCE_REST_URL = os.getenv("BINANCE_REST_URL", "https://api.binance.com/api/v3")

AI_SERVICE_PORT = int(os.getenv("AI_SERVICE_PORT", "8002"))
AI_SERVICE_HOST = os.getenv("AI_SERVICE_HOST", "0.0.0.0")

DEFAULT_SYMBOL = os.getenv("DEFAULT_SYMBOL", "BTCUSDT")
DEFAULT_INTERVAL = os.getenv("DEFAULT_INTERVAL", "1h")
PREDICTION_TIMEOUT_SECS = int(os.getenv("PREDICTION_TIMEOUT_SECS", "30"))