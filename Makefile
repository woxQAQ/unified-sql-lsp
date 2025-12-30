# Unified SQL LSP Server - Makefile

# Variables
BINARY_NAME=unified-sql-lsp
VERSION?=dev
COMMIT?=$(shell git rev-parse --short HEAD 2>/dev/null || echo "none")
BUILD_DATE?=$(shell date -u +"%Y-%m-%dT%H:%M:%SZ")
GO?=go
GOFLAGS?=
LDFLAGS=-ldflags "-X main.version=$(VERSION) -X main.commit=$(COMMIT) -X main.date=$(BUILD_DATE)"

# Directories
CMD_DIR=./cmd/server
BUILD_DIR=./build/bin
WASM_DIR=./build/wasm
ADDONS_DIR=./addons

.PHONY: all
all: build

.PHONY: build
build: clean
	@echo "Building $(BINARY_NAME)..."
	@mkdir -p $(BUILD_DIR)
	$(GO) build $(GOFLAGS) $(LDFLAGS) -o $(BUILD_DIR)/$(BINARY_NAME) $(CMD_DIR)
	@echo "Build complete: $(BUILD_DIR)/$(BINARY_NAME)"

.PHONY: build-wasm
build-wasm:
	@echo "Building Wasm addons..."
	@mkdir -p $(WASM_DIR)
	@./scripts/build-wasm.sh

.PHONY: clean
clean:
	@echo "Cleaning build artifacts..."
	@rm -rf $(BUILD_DIR) $(WASM_DIR)

.PHONY: test
test:
	@echo "Running tests..."
	$(GO) test -v -race -coverprofile=coverage.out ./...
	$(GO) tool cover -html=coverage.out -o coverage.html

.PHONY: test-coverage
test-coverage: test
	@echo "Coverage report generated: coverage.html"

.PHONY: lint
lint:
	@echo "Running linters..."
	@if command -v golangci-lint >/dev/null 2>&1; then \
		golangci-lint run ./...; \
	else \
		echo "golangci-lint not installed. Run: make tools"; \
	fi

.PHONY: fmt
fmt:
	@echo "Formatting code..."
	$(GO) fmt ./...
	@if command -v goimports >/dev/null 2>&1; then \
		goimports -w .; \
	fi

.PHONY: vet
vet:
	@echo "Running go vet..."
	$(GO) vet ./...

.PHONY: deps
deps:
	@echo "Downloading dependencies..."
	$(GO) mod download
	$(GO) mod tidy

.PHONY: tools
tools:
	@echo "Installing development tools..."
	$(GO) install github.com/golangci/golangci-lint/cmd/golangci-lint@latest
	$(GO) install golang.org/x/tools/cmd/goimports@latest
	@echo "Installing tree-sitter CLI..."
	@npm install -g tree-sitter-cli

.PHONY: run
run: build
	@echo "Running server..."
	$(BUILD_DIR)/$(BINARY_NAME)

.PHONY: check
check: fmt vet lint test

.PHONY: help
help:
	@echo "Unified SQL LSP Server - Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  all           - Build the server (default)"
	@echo "  build         - Build the server binary"
	@echo "  build-wasm    - Build Wasm addons"
	@echo "  clean         - Remove build artifacts"
	@echo "  test          - Run tests"
	@echo "  test-coverage - Run tests with coverage"
	@echo "  lint          - Run linters"
	@echo "  fmt           - Format code"
	@echo "  vet           - Run go vet"
	@echo "  deps          - Download and tidy dependencies"
	@echo "  tools         - Install development tools"
	@echo "  run           - Build and run the server"
	@echo "  check         - Run all checks (fmt, vet, lint, test)"
	@echo "  help          - Show this help message"
