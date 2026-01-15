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

## @e2e: Run all E2E tests
test-e2e:
	cd tests/e2e-rs && $(CARGO) test -- --test-threads=1
