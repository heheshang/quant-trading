# Release Note v0.8.0

- Version: 0.8.0
- Date: 2026-05-21
- Author: ssk

## 升级说明

本次升级包含 **CI 覆盖率门禁** 和 **文档完善**，为 Phase 1/2 画上句号。

## 新增功能

### CI 覆盖率门禁 (T6)
- 集成 `cargo-llvm-cov` 生成 LCOV 报告
- 新增 `coverage-gate` job，阈值：
  - Overall: ≥ 70%
  - Handlers: ≥ 90%
  - Services: ≥ 75%
  - DB: ≥ 70%

### 文档完善 (T7)
- 新增 `CONTRIBUTING.md`：开发环境、分支管理、Commit 规范
- 新增 `docs/architecture/README.md`：架构概览、ADR 索引
- 新增 `docs/api/README.md`：API 分组、文档索引
- 更新 `CHANGELOG.md` 至 v0.8.0

## 部署信息

- **后端**: Rust + Axum 0.7，监听 `:8080`
- **前端**: Vue 3 + TypeScript + Vite
- **数据库**: PostgreSQL 16
- **缓存**: Redis 7
- **Health Check**: `GET /health` → 200 OK

## 测试状态

- Backend: 247 tests ✅
- Frontend: 709 tests ✅
- CI: 14 jobs

## 已知限制

- Docker Hub 国内访问受限，构建后端 Docker 镜像需配置 registry mirror
- Pre-commit hooks 国内网络超时，请使用 `git commit --no-verify`
- Cargo audit 无法访问 advisory-db（网络限制）
