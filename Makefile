# ==============================================================================
# Unified SQL LSP - Makefile
# ==============================================================================

# Project variables
CARGO := cargo
PROJECT_NAME := unified-sql-lsp
E2E_DIR := tests/e2e-rs

# Cargo flags
CARGO_FLAGS := --workspace
RELEASE_FLAGS := --release

# Nextest configuration
NEXTEST_COMMON_ARGS := --no-fail-fast --failure-output=immediate-final --status-level all
NEXTEST_THREADS_SERIAL := 1
NEXTEST_THREADS_PARALLEL := 4

# Colors for help output (optional, disable if not supported)
COLOR_RESET := \033[0m
COLOR_BOLD := \033[1m
COLOR_CATEGORY := \033[36m

# ==============================================================================
# Default Target
# ==============================================================================
.DEFAULT_GOAL := help

# ==============================================================================
# Build Targets
# ==============================================================================

## @build: Build the entire workspace in debug mode
build:
	$(CARGO) build $(CARGO_FLAGS)

## @build: Build the entire workspace in release mode (optimized)
build-release:
	$(CARGO) build $(CARGO_FLAGS) $(RELEASE_FLAGS)

# ==============================================================================
# Development Targets
# ==============================================================================

## @dev: Run the LSP server in debug mode
run:
	$(CARGO) run -p unified-sql-lsp-lsp

## @dev: Run the LSP server in release mode (optimized)
run-release:
	$(CARGO) run $(RELEASE_FLAGS) -p unified-sql-lsp-lsp

## @dev: Watch for changes and rebuild (requires cargo-watch)
watch:
	@cargo watch --version >/dev/null 2>&1 || (echo "cargo-watch not found. Install with: cargo install cargo-watch" && exit 1)
	cargo watch -x 'build --workspace' -x 'test --workspace'

# ==============================================================================
# Testing Targets
# ==============================================================================

## @test: Run all unit tests with cargo
test:
	$(CARGO) test $(CARGO_FLAGS)

## @test: Run all unit tests with nextest (faster, requires cargo-nextest)
test-nextest:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	$(CARGO) nextest run $(CARGO_FLAGS) $(NEXTEST_COMMON_ARGS)

# ==============================================================================
# E2E Testing Targets
# ==============================================================================

## @e2e: Run all E2E tests (single-threaded to avoid Docker conflicts)
test-e2e:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd $(E2E_DIR) && cargo nextest run $(CARGO_FLAGS) $(NEXTEST_COMMON_ARGS) --test-threads=$(NEXTEST_THREADS_SERIAL)

## @e2e: Run all E2E tests in parallel (3-4x speedup)
test-e2e-parallel:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd $(E2E_DIR) && cargo nextest run $(CARGO_FLAGS) $(NEXTEST_COMMON_ARGS) --test-threads=$(NEXTEST_THREADS_PARALLEL)

## @e2e: Run MySQL 5.7 E2E tests
test-e2e-mysql-5.7:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd $(E2E_DIR) && cargo nextest run --package mysql-5-7-e2e-tests $(NEXTEST_COMMON_ARGS) --test-threads=$(NEXTEST_THREADS_SERIAL)

## @e2e: Run MySQL 8.0 E2E tests
test-e2e-mysql-8.0:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd $(E2E_DIR) && cargo nextest run --package mysql-8-0-e2e-tests $(NEXTEST_COMMON_ARGS) --test-threads=$(NEXTEST_THREADS_SERIAL)

## @e2e: Run PostgreSQL 12 E2E tests
test-e2e-postgresql-12:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd $(E2E_DIR) && cargo nextest run --package postgresql-12-e2e-tests $(NEXTEST_COMMON_ARGS) --test-threads=$(NEXTEST_THREADS_SERIAL)

## @e2e: Run PostgreSQL 16 E2E tests
test-e2e-postgresql-16:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd $(E2E_DIR) && cargo nextest run --package postgresql-16-e2e-tests $(NEXTEST_COMMON_ARGS) --test-threads=$(NEXTEST_THREADS_SERIAL)

## @e2e: Run all MySQL E2E tests (5.7 and 8.0)
test-e2e-mysql:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd $(E2E_DIR) && cargo nextest run --package mysql-5-7-e2e-tests --package mysql-8-0-e2e-tests $(NEXTEST_COMMON_ARGS) --test-threads=$(NEXTEST_THREADS_SERIAL)

## @e2e: Run all PostgreSQL E2E tests (12 and 16)
test-e2e-postgresql:
	@cargo nextest --version >/dev/null 2>&1 || (echo "cargo-nextest not found. Install with: cargo install cargo-nextest --locked" && exit 1)
	cd $(E2E_DIR) && cargo nextest run --package postgresql-12-e2e-tests --package postgresql-16-e2e-tests $(NEXTEST_COMMON_ARGS) --test-threads=$(NEXTEST_THREADS_SERIAL)

## @e2e: Run E2E tests with cargo test (fallback for systems without nextest)
test-e2e-legacy:
	cd $(E2E_DIR) && $(CARGO) test $(CARGO_FLAGS)

# ==============================================================================
# Code Quality Targets
# ==============================================================================

## @quality: Format code with rustfmt
fmt:
	$(CARGO) fmt --all

## @quality: Check code formatting without making changes
fmt-check:
	$(CARGO) fmt --all -- --check

## @quality: Run clippy lints with warnings as errors
clippy:
	$(CARGO) clippy $(CARGO_FLAGS) -- -D warnings

## @quality: Run all checks (fmt check and clippy)
check: fmt-check clippy

## @quality: Format and check code (fmt + clippy)
fix: fmt clippy

# ==============================================================================
# Documentation Targets
# ==============================================================================

## @docs: Generate and open documentation
docs:
	$(CARGO) doc $(CARGO_FLAGS) --no-deps --open

## @docs: Generate documentation without opening
docs-build:
	$(CARGO) doc $(CARGO_FLAGS) --no-deps

# ==============================================================================
# Maintenance Targets
# ==============================================================================

## @maint: Clean build artifacts
clean:
	$(CARGO) clean

## @maint: Update dependencies
update:
	$(CARGO) update

## @maint: Check for outdated dependencies
outdated:
	@cargo outdated --version >/dev/null 2>&1 || (echo "cargo-outdated not found. Install with: cargo install cargo-outdated" && exit 1)
	cargo outdated

# ==============================================================================
# Performance Profiling Targets
# ==============================================================================

## @profiling: Run quick benchmark suite
benchmark:
	@echo "Running quick benchmark suite..."
	@cargo bench --benches completion,parsing,semantic

## @profiling: Run complete profiling suite
profile-all:
	@echo "Running complete profiling suite..."
	@./scripts/profiling/run_all.sh

## @profiling: Generate CPU flamegraph
flamegraph:
	@echo "Generating flamegraph..."
	@./scripts/profiling/flamegraph.sh

## @maint: Display project size analysis
du:
	@echo "Target directory size:"
	@du -sh target 2>/dev/null || echo "No target directory found"

# ==============================================================================
# Git Targets
# ==============================================================================

## @git: Show project status (git status, branch info, recent commits)
status:
	@echo "=== Git Status ==="
	@git status
	@echo ""
	@echo "=== Branch ==="
	@git branch -v
	@echo ""
	@echo "=== Recent Commits ==="
	@git log --oneline -5

## @git: Create a new git commit (use MSG="commit message")
commit:
	git add -A
	git commit -m "$(MSG)"

## @git: Amend the last commit (use MSG="new message")
amend:
	git add -A
	git commit --amend -m "$(MSG)"

# ==============================================================================
# Help Target
# ==============================================================================

## @help: Show this help message
help:
	@echo "$(COLOR_BOLD)Unified SQL LSP - Available Commands$(COLOR_RESET)"
	@echo ""
	@awk 'BEGIN{cat=""} \
		/^## @/ { \
			category = substr($$2, 2); \
			desc = ""; for(i=3; i<=NF; i++) desc = desc $$i " "; \
			getline; \
			cmd = $$0; \
			gsub(/:.*/, "", cmd); \
			if (cmd != "" && cat != category) { \
				if (cat != "") print ""; \
				printf "$(COLOR_CATEGORY)%s$(COLOR_RESET)\n", toupper(category); \
				cat = category; \
			} \
			if (cmd != "") printf "  $(COLOR_BOLD)%-30s$(COLOR_RESET) %s\n", cmd, desc; \
		} \
	' $(MAKEFILE_LIST)
	@echo ""
	@echo "Run $(COLOR_BOLD)make <target>$(COLOR_RESET) to execute a command"

<<<<<<< HEAD
# ==============================================================================
# PHONY Declarations
# ==============================================================================
.PHONY: build build-release \
	run run-release watch \
	test test-nextest \
	test-e2e test-e2e-parallel \
	test-e2e-mysql-5.7 test-e2e-mysql-8.0 test-e2e-mysql \
	test-e2e-postgresql-12 test-e2e-postgresql-16 test-e2e-postgresql \
	test-e2e-legacy \
	fmt fmt-check clippy check fix \
	docs docs-build \
	clean update outdated du \
	status commit amend \
	help
||||||| a71b8fb
## @e2e: Run E2E tests with cargo test (fallback)
test-e2e-legacy:
	cd tests/e2e-rs && $(CARGO) test
=======
## @e2e: Run E2E tests with cargo test (fallback)
test-e2e-legacy:
	cd tests/e2e-rs && $(CARGO) test

# Benchmarking
.PHONY: benchmark profile-all flamegraph

benchmark:
	@echo "Running quick benchmark suite..."
	@cargo bench --benches completion,parsing,semantic

profile-all:
	@echo "Running complete profiling suite..."
	@./scripts/profiling/run_all.sh

flamegraph:
	@echo "Generating flamegraph..."
	@./scripts/profiling/flamegraph.sh
>>>>>>> feature/perf-001-profiling
