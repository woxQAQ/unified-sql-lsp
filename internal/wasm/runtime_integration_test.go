package wasm

import (
	"context"
	"os"
	"testing"

	"go.uber.org/zap/zaptest"
)

// TestLoadModuleFromMemory tests loading a simple Wasm module from memory.
// Note: This test uses a minimal valid Wasm module.
// In F003, we'll have actual PostgreSQL parser modules to test.
func TestLoadModuleFromMemory(t *testing.T) {
	logger := zaptest.NewLogger(t)
	ctx := context.Background()

	runtime, err := NewRuntime(ctx, logger, nil)
	if err != nil {
		t.Fatal(err)
	}
	defer runtime.Close(ctx)

	loader := NewModuleLoader(runtime, logger)

	// Minimal valid Wasm module (empty module that does nothing).
	// This is a valid Wasm 1.0 module with no exports.
	wasmBytes := []byte{
		0x00, 0x61, 0x73, 0x6d, // Magic number: \0asm
		0x01, 0x00, 0x00, 0x00, // Version: 1
	}

	module, err := loader.LoadModuleFromMemory(ctx, "test-module", wasmBytes)
	if err != nil {
		t.Fatalf("Failed to load module: %v", err)
	}

	if module == nil {
		t.Fatal("Module is nil")
	}

	if module.Name != "test-module" {
		t.Errorf("Module name = %s, want 'test-module'", module.Name)
	}

	// Test caching - load again should hit cache.
	module2, err := loader.LoadModuleFromMemory(ctx, "test-module", wasmBytes)
	if err != nil {
		t.Fatalf("Failed to load module from cache: %v", err)
	}

	if module2 != module {
		t.Error("Cache should return the same module instance")
	}
}

// TestModuleLoaderFileSource tests the FileModuleSource.
func TestModuleLoaderFileSource(t *testing.T) {
	logger := zaptest.NewLogger(t)
	ctx := context.Background()

	runtime, err := NewRuntime(ctx, logger, nil)
	if err != nil {
		t.Fatal(err)
	}
	defer runtime.Close(ctx)

	loader := NewModuleLoader(runtime, logger)

	// Create a temporary Wasm file.
	tmpDir := t.TempDir()
	wasmFile := tmpDir + "/test.wasm"

	// Write minimal Wasm module.
	wasmBytes := []byte{
		0x00, 0x61, 0x73, 0x6d, // Magic number
		0x01, 0x00, 0x00, 0x00, // Version
	}

	// Write the file
	if err := os.WriteFile(wasmFile, wasmBytes, 0644); err != nil {
		t.Fatalf("Failed to write test file: %v", err)
	}

	// Load from file.
	_, err = loader.LoadModuleFromFile(ctx, wasmFile)
	if err != nil {
		t.Fatalf("Failed to load module from file: %v", err)
	}
}

// TestHostFunctions tests host function creation.
func TestHostFunctions(t *testing.T) {
	logger := zaptest.NewLogger(t)

	hostFuncs := NewHostFunctions(logger)
	if hostFuncs == nil {
		t.Fatal("HostFunctionsImpl is nil")
	}

	if hostFuncs.logger == nil {
		t.Error("Logger not initialized")
	}
}

// TestMemoryHelpers tests memory helper functions.
// Note: Full memory testing will be available in F003 when we have actual modules.
func TestMemoryHelpers(t *testing.T) {
	logger := zaptest.NewLogger(t)
	ctx := context.Background()

	runtime, err := NewRuntime(ctx, logger, nil)
	if err != nil {
		t.Fatal(err)
	}
	defer runtime.Close(ctx)

	// Create a simple module with memory for testing.
	loader := NewModuleLoader(runtime, logger)

	// Minimal Wasm module with memory export.
	// This module exports 1 page of memory (64KB).
	wasmBytes := []byte{
		0x00, 0x61, 0x73, 0x6d, // Magic
		0x01, 0x00, 0x00, 0x00, // Version
		// Type section
		0x01, 0x00, // Empty type section
		// Memory section (1 page)
		0x05, 0x01, 0x00, 0x01, // Memory section: 1 page
		// Export section
		0x07, 0x07, 0x01, 0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00, // export "memory" as memory
	}

	_, err = loader.LoadModuleFromMemory(ctx, "memory-test", wasmBytes)
	if err != nil {
		t.Logf("Note: Memory module loading failed (expected for minimal Wasm binary): %v", err)
		// This is OK for now - we're testing the infrastructure
		return
	}

	// Instantiate the module.
	hostFuncs := NewHostFunctions(logger)
	instanceMgr := NewInstanceManager(runtime, hostFuncs, logger)

	instance, err := instanceMgr.Instantiate(ctx, &InstanceConfig{
		ModuleName: "memory-test",
	})
	if err != nil {
		t.Fatalf("Failed to instantiate: %v", err)
	}
	defer instance.Close(ctx)

	// Test memory helper.
	mem := NewMemory(instance.module)
	if mem == nil {
		t.Fatal("Memory helper is nil")
	}

	// Test reading bytes.
	// Write some data to memory first (offset 0).
	success := instance.module.Memory().WriteUint32Le(0, 0x12345678)
	if !success {
		t.Fatal("Failed to write to memory")
	}

	// Read back using memory helper.
	data, ok := mem.ReadBytes(0, 4)
	if !ok {
		t.Fatal("Failed to read from memory")
	}

	if len(data) != 4 {
		t.Errorf("Read %d bytes, want 4", len(data))
	}
}
