.PHONY: help build build-release build-all test test-verbose test-coverage run run-lsp clean clean-all fmt clippy check grammar install

# Default target
.DEFAULT_GOAL := help

# Project variables
CARGO := cargo
PROJECT_NAME := unified-sql-lsp

help:
	@echo "$(PROJECT_NAME) - Development Commands"
	@echo ""
	@echo "Build:"
	@echo "  build              Build workspace"
	@echo "  build-release      Build workspace (release)"
	@echo "  build-grammar      Build grammar crate"
	@echo "  build-lsp          Build LSP server"
	@echo ""
	@echo "Test:"
	@echo "  test               Run all tests"
	@echo "  test-verbose       Run tests with output"
	@echo "  test-grammar       Run grammar tests"
	@echo "  test-lsp           Run LSP tests"
	@echo "  test-e2e           Run E2E tests"
	@echo ""
	@echo "Grammar:"
	@echo "  grammar            Build all dialects"
	@echo "  grammar-mysql      Build MySQL grammar"
	@echo "  grammar-postgresql Build PostgreSQL grammar"
	@echo ""
	@echo "Run:"
	@echo "  run                Run LSP server"
	@echo "  run-release        Run LSP server (release)"
	@echo ""
	@echo "Quality:"
	@echo "  fmt                Format code"
	@echo "  clippy             Run linter"
	@echo "  check              Run all checks"
	@echo ""
	@echo "Other:"
	@echo "  clean              Clean build artifacts"
	@echo "  install            Install dependencies"
	@echo "  update             Update dependencies"
	@echo "  docs               Generate documentation"
	@echo "  status             Show git status"

## @build: Build the entire workspace
build:
	$(CARGO) build --workspace

## @build: Build the entire workspace in release mode
build-release:
	$(CARGO) build --workspace --release

## @build: Build grammar crate
build-grammar:
	$(CARGO) build -p unified-sql-grammar

## @build: Build LSP server
build-lsp:
	$(CARGO) build -p unified-sql-lsp-lsp

## @build: Build IR crate
build-ir:
	$(CARGO) build -p unified-sql-lsp-ir

## @build: Build lowering crate
build-lowering:
	$(CARGO) build -p unified-sql-lsp-lowering

## @build: Build semantic crate
build-semantic:
	$(CARGO) build -p unified-sql-lsp-semantic

## @build: Build catalog crate
build-catalog:
	$(CARGO) build -p unified-sql-lsp-catalog

## @test: Run all tests
test:
	$(CARGO) test --workspace

## @test: Run tests with output
test-verbose:
	$(CARGO) test --workspace -- --nocapture

## @test: Run tests with coverage
test-coverage:
	$(CARGO) test --workspace

## @test: Run grammar tests
test-grammar:
	$(CARGO) test -p unified-sql-grammar
	cd crates/grammar && npm test

## @test: Run grammar tests for MySQL dialect
test-grammar-mysql:
	cd crates/grammar && npm run test:mysql

## @test: Run grammar tests for PostgreSQL dialect
test-grammar-postgresql:
	cd crates/grammar && npm run test:postgresql

## @test: Run LSP server tests
test-lsp:
	$(CARGO) test -p unified-sql-lsp-lsp

## @test: Run specific test (use TEST=name variable)
test-specific:
	$(CARGO) test --workspace $(TEST)

## @run: Run the LSP server
run:
	$(CARGO) run -p unified-sql-lsp-lsp

## @run: Run the LSP server in release mode
run-release:
	$(CARGO) run --release -p unified-sql-lsp-lsp

## @clean: Clean build artifacts
clean:
	$(CARGO) clean

## @fmt: Format code with rustfmt
fmt:
	$(CARGO) fmt --all

## @check: Check code with clippy
clippy:
	$(CARGO) clippy --workspace -- -D warnings

## @check: Run all checks (fmt check and clippy)
check: fmt-check clippy

## @grammar: Build grammar for all dialects
grammar:
	cd crates/grammar && ./build.sh

## @grammar: Build MySQL dialect grammar
grammar-mysql:
	cd crates/grammar/src/grammar && DIALECT=mysql tree-sitter generate

## @grammar: Build PostgreSQL dialect grammar
grammar-postgresql:
	cd crates/grammar/src/grammar && DIALECT=postgresql tree-sitter generate

## @misc: Update dependencies
update:
	$(CARGO) update
	cd crates/grammar && npm update

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

## @e2e: Run all E2E tests
test-e2e:
	cd tests/e2e-rs && $(CARGO) test
