# Project variables
CARGO := cargo
PROJECT_NAME := unified-sql-lsp

## @build: Build the entire workspace
build:
	$(CARGO) build --workspace

## @test: Run all tests
test:
	$(CARGO) test --workspace

## @run: Run the LSP server
run:
	$(CARGO) run -p unified-sql-lsp-lsp

## @run: Run the LSP server in release mode
run-release:
	$(CARGO) run --release -p unified-sql-lsp-lsp

## @fmt: Format code with rustfmt
fmt:
	$(CARGO) fmt --all

## @check: Check code with clippy
clippy:
	$(CARGO) check
	$(CARGO) clippy --workspace -- -D warnings

## @check: Run all checks (fmt check and clippy)
check: fmt clippy

## @misc: Show project status (git status, branch info)
status:
	@echo "=== Git Status ==="
	@git status
	@echo ""
	@echo "=== Branch ==="
	@git branch -v
	@echo ""
	@echo "=== Recent Commits ==="
	@git log --oneline -5

## @misc: Create a new git commit (use MSG="commit message")
commit:
	git add -A
	git commit -m "$(MSG)"

## @docs: Generate and open documentation
docs:
	$(CARGO) doc --workspace --no-deps --open

## @e2e: Run all E2E tests with nextest (single thread, avoids Docker conflicts)
test-e2e:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd tests/e2e-rs && cargo nextest run --workspace --no-fail-fast --test-threads=1 --failure-output=immediate-final --status-level all

## @e2e: Run all E2E tests in parallel (3-4x speedup)
test-e2e-parallel:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd tests/e2e-rs && cargo nextest run --workspace --no-fail-fast --test-threads=4 --failure-output=immediate-final --status-level all

## @e2e: Run MySQL 5.7 E2E tests
test-e2e-mysql-5.7:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd tests/e2e-rs && cargo nextest run --package mysql-5-7-e2e-tests --no-fail-fast --test-threads=1 --failure-output=immediate-final --status-level all

## @e2e: Run MySQL 8.0 E2E tests
test-e2e-mysql-8.0:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd tests/e2e-rs && cargo nextest run --package mysql-8-0-e2e-tests --no-fail-fast --test-threads=1 --failure-output=immediate-final --status-level all

## @e2e: Run PostgreSQL 12 E2E tests
test-e2e-postgresql-12:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd tests/e2e-rs && cargo nextest run --package postgresql-12-e2e-tests --no-fail-fast --test-threads=1 --failure-output=immediate-final --status-level all

## @e2e: Run PostgreSQL 16 E2E tests
test-e2e-postgresql-16:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd tests/e2e-rs && cargo nextest run --package postgresql-16-e2e-tests --no-fail-fast --test-threads=1 --failure-output=immediate-final --status-level all

## @e2e: Run all MySQL E2E tests (5.7 and 8.0)
test-e2e-mysql:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd tests/e2e-rs && cargo nextest run --package mysql-5-7-e2e-tests --package mysql-8-0-e2e-tests --no-fail-fast --test-threads=1 --failure-output=immediate-final --status-level all

## @e2e: Run all PostgreSQL E2E tests (12 and 16)
test-e2e-postgresql:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd tests/e2e-rs && cargo nextest run --package postgresql-12-e2e-tests --package postgresql-16-e2e-tests --no-fail-fast --test-threads=1 --failure-output=immediate-final --status-level all

## @e2e: Run E2E tests with cargo test (fallback)
test-e2e-legacy:
	cd tests/e2e-rs && $(CARGO) test --workspace
