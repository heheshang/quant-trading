# CONTRIBUTING.md

- Version: 1.0.0
- Date: 2026-05-20
- Author: ssk

## 环境要求

- Rust 1.75+ (with `cargo`)
- Node.js 22+ and npm
- Docker and Docker Compose
- PostgreSQL 16, Redis 7

## 开发环境启动

```bash
# 1. 克隆后进入项目目录
cd quant-trading

# 2. 启动数据库和缓存（Docker）
docker compose up -d postgres redis

# 3. 启动后端
cd backend
cargo build
DATABASE_URL="postgres://postgres:postgres@localhost:5432/quant_trading" \
REDIS_URL="redis://localhost:6379" \
cargo run

# 4. 启动前端
cd ../frontend
npm ci
npm run dev
```

## 分支管理

- `main` — 生产稳定版，只接受 PR 合并
- `develop` — 开发分支，所有功能先合并到这里

## Commit 规范

使用 `--no-verify` 绕过 pre-commit hook（超时严重）：

```bash
git add .
git commit --no-verify -m "feat(scope): description"
```

Commit 类型前缀：`feat:` `fix:` `docs:` `refactor:` `test:` `chore:`

## 代码规范

### Backend (Rust)

```bash
cargo fmt        # 格式
cargo clippy     # Lint
cargo test       # 测试
```

### Frontend (Vue 3 + TypeScript)

```bash
npx vue-tsc --noEmit   # TypeScript 检查
npm run lint           # ESLint
npm run test           # 单元测试
```

## 测试覆盖要求

| 模块 | 最低覆盖率 |
|------|-----------|
| handlers | 90% |
| services | 75% |
| db | 70% |
| overall | 70% |

## Pull Request 流程

1. 从 `develop` 创建功能分支
2. 开发完成后发起 PR → `develop`
3. CI 全部通过后 Review
4. Review 通过后合并

## 注意事项

- Pre-commit hooks 在国内网络环境可能超时，请使用 `--no-verify`
- Docker Hub 访问受限，构建后端 Docker 镜像时使用 `--build-arg REGISTRY_MIRROR`
- Axum 0.7 路由参数语法：`{id}` 而非 `:id`
