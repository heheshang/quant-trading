# Deployment Log

- Version: 1.0.0
- Date: 2026-05-21
- Author: ssk

## 环境信息

| 项目 | 值 |
|------|-----|
| 部署时间 | 2026-05-21 |
| 部署模式 | Staging (Local Docker + Binary) |
| 后端端口 | 8080 |
| 前端端口 | 5173 (dev) |
| 数据库 | PostgreSQL 16 via Docker |
| 缓存 | Redis 7 via Docker |

## 部署步骤

### 1. 数据库和缓存启动

```bash
docker compose up -d postgres redis
```

### 2. 后端启动

```bash
cd backend
cargo build
DATABASE_URL="postgres://quant:quant123@localhost:5432/quant_trading" \
REDIS_URL="redis://localhost:6379" \
cargo run
```

### 3. 健康检查

```bash
curl http://localhost:8080/health
# {"code":0,"data":{"status":"ok","version":"0.1.0"}}
```

## 部署配置清单

| 配置项 | 状态 |
|--------|------|
| docker-compose.yml | ✅ |
| Dockerfile.backend | ✅ |
| Dockerfile.frontend | ✅ |
| nginx.conf | ✅ |
| .env.example | ✅ |
| Health Check | ✅ (docker-compose + /health endpoint) |

## 版本信息

- Backend: 0.8.0 (commit `f438ebc`)
- Frontend: (build OK)
- Node.js: 22
- Rust: stable
- PostgreSQL: 16-alpine
- Redis: 7-alpine
