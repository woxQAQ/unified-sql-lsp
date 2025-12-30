package wasm

import (
	"fmt"

	"github.com/tetratelabs/wazero/api"
)

// Memory provides safe memory operations for Wasm module interaction.
//
// Wasm modules have their own isolated memory space that is separate from Go's memory.
// Direct memory access can lead to:
// - Out-of-bounds reads/writes (security vulnerabilities)
// - Type confusion (reading bytes as strings without null termination)
// - Memory leaks (forgetting to free allocated memory)
//
// This Memory helper type wraps wazero's api.Memory interface to provide:
// 1. Safe string operations with automatic null-termination handling
// 2. Bounds checking on all read operations
// 3. Abstraction over raw memory addresses
// 4. Consistent error handling across all memory operations
//
// In F003, write operations will be added with a proper memory allocator
// that interfaces with the Wasm module's malloc/free functions.
type Memory struct {
	mem api.Memory
}

// NewMemory creates a memory helper.
func NewMemory(module api.Module) *Memory {
	return &Memory{mem: module.Memory()}
}

// ReadString reads a null-terminated string from Wasm memory.
func (m *Memory) ReadString(ptr uint32, maxLen uint32) (string, bool) {
	// Read bytes until null terminator or maxLen.
	buf, ok := m.mem.Read(ptr, maxLen)
	if !ok {
		return "", false
	}

	// Find null terminator.
	end := len(buf)
	for i, b := range buf {
		if b == 0 {
			end = i
			break
		}
	}

	return string(buf[:end]), true
}

// ReadBytes reads raw bytes from Wasm memory.
func (m *Memory) ReadBytes(ptr uint32, length uint32) ([]byte, bool) {
	return m.mem.Read(ptr, length)
}

// WriteString writes a string to Wasm memory.
// Returns pointer and length, or error if allocation fails.
// TODO: Implement in F003 with proper memory allocator.
func (m *Memory) WriteString(s string) (uint32, uint32, error) {
	return 0, 0, fmt.Errorf("memory allocation not yet implemented - will be added in F003")
}

// WriteBytes writes bytes to Wasm memory.
// TODO: Implement in F003 with proper memory allocator.
func (m *Memory) WriteBytes(data []byte) (uint32, uint32, error) {
	return 0, 0, fmt.Errorf("memory allocation not yet implemented - will be added in F003")
}
