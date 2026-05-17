.PHONY: build test lint fmt clippy deny audit check test-all clean help

# Build the backend
build:
	cd backend && cargo build

# Run all tests
test:
	cd backend && cargo test

# Run tests with nextest (faster, parallel)
test-nextest:
	cd backend && cargo nextest run --all-features

# Lint: fmt + clippy + deny
lint: fmt clippy deny typos

# Check formatting
fmt:
	cd backend && cargo fmt --all

# Clippy lint with warnings as errors
clippy:
	cd backend && cargo clippy --all-targets --all-features --tests --benches -- -D warnings

# Check dependencies (licenses, security advisories, banned crates)
deny:
	cd backend && cargo deny check -d

# Typo check
typos:
	typos --format downstream

# Security audit
audit:
	cd backend && cargo audit

# Full check: build + test + lint
check: build test lint

# Full check with nextest
check-nextest: build test-nextest lint

# Run frontend checks (from repo root)
frontend-check:
	cd frontend && bunx vue-tsc --noEmit && bun run lint

# Help
help:
	@echo "量化交易系统 - Makefile targets"
	@echo ""
	@echo "  build         Build backend"
	@echo "  test          Run cargo test"
	@echo "  test-nextest  Run cargo nextest (parallel, faster)"
	@echo "  lint          Run fmt + clippy + deny + typos"
	@echo "  fmt           Check/apply rustfmt"
	@echo "  clippy        Run clippy lint"
	@echo "  deny          Check dependencies (licenses/security)"
	@echo "  typos         Check for typos"
	@echo "  audit         Run cargo audit (security advisories)"
	@echo "  check         Full check: build + test + lint"
	@echo "  check-nextest Full check with nextest"
	@echo "  frontend-check Run frontend type-check + lint"
	@echo "  clean         Remove target directory"
