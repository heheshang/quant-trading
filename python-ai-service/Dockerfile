FROM python:3.11-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt -i https://mirrors.aliyun.com/pypi/simple/

COPY . .

ENV PORT=8001
ENV USE_MOCK=true
ENV MODEL_VERSION=v1.0.0

EXPOSE 8001

CMD ["python3", "main.py"]
