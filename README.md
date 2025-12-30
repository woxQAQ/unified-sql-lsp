# Unified SQL LSP Server

A unified Language Server Protocol (LSP) server providing intelligent SQL code completion for multiple database engines through a Wasm-based plugin architecture.

## Features

- **Multi-Engine Support**: PostgreSQL, MySQL, and extensible to more
- **Wasm Plugin Architecture**: Safe, isolated, and performant add-ons
- **Context-Aware Completion**: Intelligent suggestions based on SQL context
- **Schema Introspection**: Auto-discovery of database schemas
- **High Performance**: Incremental parsing with Tree-sitter

## Requirements

- Go 1.24 or higher
- Make
- Node.js and npm (for tree-sitter-cli, add-on development)

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/vibe-kanban/unified-sql-lsp.git
cd unified-sql-lsp

# Download dependencies
make deps

# Build the server
make build
```

### Usage

```bash
# Run with stdio (for VS Code, etc.)
./build/bin/unified-sql-lsp

# Run with TCP
./build/bin/unified-sql-lsp --port 4389

# Run with custom log level
./build/bin/unified-sql-lsp --log-level debug

# Run with configuration file
./build/bin/unified-sql-lsp --config config/server.yaml
```

## Development

### Project Structure

```
unified-sql-lsp/
├── cmd/server/          # Main application entry point
├── internal/            # Private implementation packages
├── pkg/                 # Public packages
├── addons/              # Wasm add-ons for different engines
├── api/                 # API definitions
├── docs/                # Technical documentation
└── scripts/             # Build and utility scripts
```

### Building Wasm Add-ons

```bash
make build-wasm
```

### Running Tests

```bash
make test
make test-coverage
```

### Development Tools

```bash
make tools  # Install development tools
make fmt    # Format code
make lint   # Run linters
make vet    # Run go vet
```

## Documentation

- [Technical Design Document](docs/技术设计文档.md) - Complete architecture and design
- [Feature List](FEATURE_LIST.yaml) - Implementation roadmap
- [Development Guide](docs/development.md) - Development setup and workflow

## Architecture

The unified SQL LSP server follows a **layered architecture with WebAssembly plugin system**:

1. **Multi-client single connection model**: Each LSP client maintains one database connection
2. **Centralized server**: Single server instance serving multiple clients concurrently
3. **Wasm plugin isolation**: Each engine Add-on compiled as independent Wasm module
4. **Incremental parsing**: Tree-sitter based high-performance incremental parsing

## License

[Specify your license]

## Contributing

Contributions are welcome! Please see [docs/技术设计文档.md](docs/技术设计文档.md) for architecture guidelines.
