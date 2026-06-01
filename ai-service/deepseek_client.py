"""DeepSeek API 客户端"""
import json
import httpx
from typing import Optional
from config import DEEPSEEK_API_KEY, DEEPSEEK_BASE_URL, DEEPSEEK_MODEL, PREDICTION_TIMEOUT_SECS


class DeepSeekClient:
    def __init__(self, api_key: Optional[str] = None):
        self.api_key = api_key or DEEPSEEK_API_KEY
        self.base_url = DEEPSEEK_BASE_URL
        self.model = DEEPSEEK_MODEL
        self.timeout = PREDICTION_TIMEOUT_SECS

    def _build_headers(self) -> dict:
        return {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }

    async def chat(self, messages: list[dict], temperature: float = 0.7) -> str:
        """发送对话请求，返回 assistant 回复文本"""
        if not self.api_key:
            raise RuntimeError("DEEPSEEK_API_KEY 未设置")

        url = f"{self.base_url}/chat/completions"
        payload = {
            "model": self.model,
            "messages": messages,
            "temperature": temperature,
        }

        async with httpx.AsyncClient(timeout=self.timeout) as client:
            resp = await client.post(url, headers=self._build_headers(), json=payload)
            resp.raise_for_status()
            data = resp.json()
            return data["choices"][0]["message"]["content"]

    async def analyze_klines(
        self,
        symbol: str,
        interval: str,
        klines_data: list[dict],
        indicators: dict,
    ) -> dict:
        """构造 prompt，调用 DeepSeek 生成交易信号分析"""

        # 格式化 K 线数据摘要（最近 5 根）
        recent = klines_data[-5:] if len(klines_data) >= 5 else klines_data
        kline_summary = "\n".join([
            f"  K线 {k['open_time']}: O={k['open']} H={k['high']} L={k['low']} C={k['close']} V={k['volume']}"
            for k in recent
        ])

        # 格式化技术指标
        ind_summary = "\n".join([
            f"  {k}: {v}"
            for k, v in indicators.items()
        ])

        system_prompt = (
            "你是一个专业的加密货币量化交易分析师。你的任务是根据 K 线数据和技术指标，"
            "生成交易信号和分析。只返回结构化的 JSON，不要有其他内容。\n\n"
            "输出格式（严格 JSON）：\n"
            "{\n"
            '  "direction": "long" | "short" | "neutral",\n'
            '  "confidence": 0.0~1.0,\n'
            '  "signal": "strong_buy" | "buy" | "neutral" | "sell" | "strong_sell",\n'
            '  "analysis": "简明分析，不超过 100 字",\n'
            '  "price_target": 价格数值（可选，若无法估算则不返回）\n'
            "}"
        )

        user_prompt = (
            f"交易对: {symbol}\n"
            f"周期: {interval}\n\n"
            f"最近 K 线（最新在最后）:\n{kline_summary}\n\n"
            f"技术指标:\n{ind_summary}\n\n"
            "请给出你的交易建议（严格 JSON 格式）："
        )

        try:
            content = await self.chat(
                messages=[
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_prompt},
                ],
                temperature=0.3,  # 低温度保证格式稳定
            )

            # 尝试解析 JSON
            # DeepSeek 可能返回 ```json ... ``` 包裹
            content = content.strip()
            if content.startswith("```"):
                lines = content.split("\n")
                content = "\n".join(lines[1:-1])

            return json.loads(content)
        except json.JSONDecodeError as e:
            raise RuntimeError(f"DeepSeek 返回格式错误，无法解析: {e}\n原始内容: {content[:200]}")
        except Exception as e:
            raise RuntimeError(f"DeepSeek API 调用失败: {e}")