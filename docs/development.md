# Development Guide

## Setup Development Environment

### Prerequisites

1. Go 1.24+
2. Make
3. Git
4. Node.js and npm (for tree-sitter-cli)

### Initial Setup

```bash
# Clone repository
git clone https://github.com/vibe-kanban/unified-sql-lsp.git
cd unified-sql-lsp

# Install Go dependencies
make deps

# Install development tools
make tools

# Verify installation
make build
./build/bin/unified-sql-lsp --help
```

## Development Workflow

### 1. Make Changes

Edit code in your preferred editor.

### 2. Format Code

```bash
make fmt
```

### 3. Run Checks

```bash
make check  # Runs fmt, vet, lint, and test
```

### 4. Build

```bash
make build
```

### 5. Test

```bash
# Run all tests
make test

# Run with coverage
make test-coverage
```

## Project Structure

See [Technical Design Document](技术设计文档.md) Section 6.2 for complete project structure.

## Adding New Features

1. Create a feature branch from `main`
2. Implement the feature
3. Add tests
4. Update documentation
5. Submit a pull request

## Building Wasm Add-ons

Wasm add-on development is covered in F003. See [Technical Design Document](技术设计文档.md) Section 3.

## Troubleshooting

### Go Version Issues

Ensure you're using Go 1.24 or higher:

```bash
go version
```

### Dependency Issues

```bash
go mod download
go mod tidy
```

### Build Failures

```bash
make clean
make build
```

## Makefile Targets

- `all` - Build the server (default)
- `build` - Build the server binary
- `build-wasm` - Build Wasm addons
- `clean` - Remove build artifacts
- `test` - Run tests
- `test-coverage` - Run tests with coverage
- `lint` - Run linters
- `fmt` - Format code
- `vet` - Run go vet
- `deps` - Download and tidy dependencies
- `tools` - Install development tools
- `run` - Build and run the server
- `check` - Run all checks
- `help` - Show help message
