# Add-on Development Guide

This guide explains how to develop Wasm add-ons for the Unified SQL LSP Server.

## Overview

Add-ons are WebAssembly (Wasm) modules that extend the LSP server with support for specific database engines. Each add-on provides parsing, completion, and diagnostics for a SQL dialect.

## Add-on Structure

An add-on is a directory containing:

```
my-addon/
├── manifest.yaml       # Add-on metadata
├── my-addon.wasm       # Compiled Wasm module
└── (source files)      # Go source code
```

## manifest.yaml Schema

Every add-on must have a `manifest.yaml` file with the following structure:

```yaml
# Required: Add-on name (typically the engine name in lowercase)
name: postgresql

# Required: Add-on version (semver)
version: 1.0.0

# Required: Database engine this add-on supports
# Must be one of: PostgreSQL, MySQL
engine: PostgreSQL

# Required: List of supported database versions
supported_versions:
  - "14"
  - "15"
  - "16"

# Required: Wasm module configuration
wasm:
  # Required: Wasm binary filename (relative to manifest directory)
  file: postgresql.wasm
  # Optional: Estimated size in KB
  size: 2048

# Required: List of capabilities provided by this add-on
# Must be subset of: completion, diagnostics, schema_introspection
capabilities:
  - completion
  - diagnostics

# Optional: Author information
author: "Your Name"

# Optional: License
license: MIT
```

## Required Exports

Your Wasm module must export the following functions:

### parse

Parse SQL code and return the syntax tree.

```go
//go:wasmexport parse
func parse(ptr uint32, length uint32) uint32
```

**Parameters:**
- `ptr`: Pointer to SQL string in Wasm memory
- `length`: Length of SQL string

**Returns:**
- Pointer to serialized syntax tree in Wasm memory

### complete

Provide completion suggestions at a given cursor position.

```go
//go:wasmexport complete
func complete(contextPtr uint32, contextLen uint32) uint32
```

**Parameters:**
- `contextPtr`: Pointer to completion context JSON
- `contextLen`: Length of context JSON

**Returns:**
- Pointer to completion results JSON

### metadata

Return add-on metadata and capabilities.

```go
//go:wasmexport metadata
func metadata() (ptr uint32, length uint32)
```

**Returns:**
- `ptr`: Pointer to metadata JSON
- `length`: Length of metadata JSON

## Host Functions

The server exports the following functions that your add-on can call:

### log_message

Log a message to the server logs.

```go
//go:wasmimport host log_message
func logMessage(level uint32, ptr uint32, length uint32)
```

**Parameters:**
- `level`: Log level (0=debug, 1=info, 2=warn, 3=error)
- `ptr`: Pointer to message string in Wasm memory
- `length`: Length of message string

**Example:**
```go
msg := "Hello from add-on"
ptr := allocateString(msg)
logMessage(1, ptr, uint32(len(msg))) // Level 1 = info
```

### get_schema

Query database schema information (TODO: to be implemented in F012).

```go
//go:wasmimport host get_schema
func getSchema(dbPtr uint32, dbLen uint32) uint32
```

**Status:** Stub only, will be implemented in F012 (Schema Introspection)

### execute_query

Execute a SQL query on the database (TODO: to be implemented in F012).

```go
//go:wasmimport host execute_query
func executeQuery(dbPtr uint32, dbLen uint32) uint32
```

**Status:** Stub only, will be implemented in F012 (Schema Introspection)

## Building Your Add-on

### Prerequisites

- Go 1.24 or later
- WASI sysroot for `GOOS=wasip1`

### Step 1: Create Go Module

```bash
mkdir postgresql-addon
cd postgresql-addon
go mod init github.com/yourname/postgresql-addon
```

### Step 2: Write Add-on Code

Create `main.go`:

```go
package main

import (
	"fmt"
)

//go:wasmexport parse
func parse(ptr, length uint32) uint32 {
	// TODO: Implement SQL parsing
	// For now, return a placeholder response
	result := `{"status": "ok", "ast": {}}`
	return allocateString(result)
}

//go:wasmexport complete
func complete(contextPtr, contextLen uint32) uint32 {
	// TODO: Implement completion
	// For now, return placeholder
	result := `{"completions": []}`
	return allocateString(result)
}

//go:wasmexport metadata
func metadata() (ptr, length uint32) {
	// Return add-on metadata
	meta := `{
		"name": "postgresql",
		"version": "1.0.0",
		"capabilities": ["completion", "diagnostics"]
	}`
	return allocateString(meta)
}

// Helper: Allocate string in Wasm memory and return pointer
func allocateString(s string) uint32 {
	// This is a simplified example
	// In real implementation, you'd use Wasm memory allocation
	// See memory management section below
	return 0
}

func main() {
	// Not executed in Wasm environment
}
```

### Step 3: Compile to Wasm

```bash
GOOS=wasip1 GOARCH=wasm go build -o postgresql.wasm
```

### Step 4: Create manifest.yaml

```yaml
name: postgresql
version: 1.0.0
engine: PostgreSQL
supported_versions:
  - "14"
  - "15"
  - "16"
wasm:
  file: postgresql.wasm
  size: 2048
capabilities:
  - completion
  - diagnostics
author: "Your Name"
license: MIT
```

### Step 5: Test Your Add-on

```bash
# Copy to server addon directory
cp postgresql.wasm /path/to/server/addons/postgresql/
cp manifest.yaml /path/to/server/addons/postgresql/

# Start server
unified-sql-lsp --addon-paths /path/to/server/addons/
```

## Memory Management

### Allocating Memory

Use the following helper to allocate and write strings to Wasm memory:

```go
import (
	"unsafe"
)

// AllocateString writes a string to Wasm memory and returns the pointer
func AllocateString(s string) (ptr uint32, length uint32) {
	data := []byte(s)

	// Allocate memory (using your Wasm runtime's allocator)
	ptr = allocate(uint32(len(data)))

	// Write string to memory
	writeMemory(ptr, data)

	return ptr, uint32(len(data))
}

// allocate is a placeholder - actual implementation depends on your Wasm runtime
func allocate(size uint32) uint32 {
	// TODO: Implement using Wasm memory growth or custom allocator
	return 0
}

// writeMemory writes bytes to Wasm memory at the given offset
func writeMemory(ptr uint32, data []byte) {
	// TODO: Implement using unsafe pointer to Wasm memory
	_ = ptr
	_ = data
}
```

### Reading Memory

To read parameters passed from the host:

```go
import "unsafe"

// ReadString reads a string from Wasm memory
func ReadString(ptr uint32, length uint32) string {
	// Get pointer to Wasm memory
	mem := wasmMemory()

	// Create byte slice from memory
	slice := (*[1 << 30]byte)(unsafe.Pointer(uintptr(ptr)))[:length:length]

	return string(slice)
}

func wasmMemory() []byte {
	// TODO: Get actual Wasm memory pointer
	// This depends on your Wasm runtime
	return nil
}
```

## Best Practices

### 1. Error Handling

Always check for errors and return meaningful error messages:

```go
//go:wasmexport parse
func parse(ptr, length uint32) uint32 {
	if ptr == 0 {
		logMessage(3, allocateString("null pointer"), 13)
		return 0
	}

	sql := readString(ptr, length)
	if sql == "" {
		logMessage(3, allocateString("empty SQL"), 9)
		return 0
	}

	// Parse SQL...
	result, err := parseSQL(sql)
	if err != nil {
		logMessage(3, allocateString(err.Error()), uint32(len(err.Error())))
		return 0
	}

	return allocateString(result)
}
```

### 2. Memory Efficiency

- Free allocated memory when done (if your runtime supports it)
- Reuse buffers when possible
- Avoid unnecessary allocations in hot paths

### 3. Performance

- Cache parsed results when possible
- Use incremental parsing for large files
- Minimize host function calls (they have overhead)

### 4. Testing

Test your add-on thoroughly:

```go
func TestParse(t *testing.T) {
	sql := "SELECT * FROM users"
	ptr, length := allocateString(sql)

	result := parse(ptr, length)

	// Assert result is valid
}
```

## Debugging

### Enable Debug Logging

```go
msg := "Debug: parsing SQL"
ptr, length := allocateString(msg)
logMessage(0, ptr, length) // Level 0 = debug
```

### Use Wasm Runtime Debugging

Start the server with debug enabled:

```bash
unified-sql-lsp --wasm-debug
```

### Common Issues

**Issue:** "invalid magic number" error
**Solution:** Make sure you're compiling with `GOOS=wasip1 GOARCH=wasm`

**Issue:** Function not found
**Solution:** Ensure all required exports have `//go:wasmexport` directive

**Issue:** Out of memory
**Solution:** Increase memory limit in server configuration

## Example Add-ons

See the `examples/` directory for complete example add-ons:
- `examples/postgresql-addon/` - PostgreSQL support
- `examples/mysql-addon/` - MySQL support

## Further Reading

- [WebAssembly in Go](https://go.dev/blog/wasm)
- [WASI Documentation](https://wasi.dev/)
- [wazero Documentation](https://wazero.io/)
- [LSP Specification](https://microsoft.github.io/language-server-protocol/)
