package wasm

import (
	"context"
	"testing"
	"time"

	"go.uber.org/zap/zaptest"
)

func TestNewRuntime(t *testing.T) {
	logger := zaptest.NewLogger(t)
	ctx := context.Background()

	runtime, err := NewRuntime(ctx, logger, nil)
	if err != nil {
		t.Fatalf("Failed to create runtime: %v", err)
	}

	if runtime == nil {
		t.Fatal("Runtime is nil")
	}

	// Cleanup
	if err := runtime.Close(context.Background()); err != nil {
		t.Errorf("Failed to close runtime: %v", err)
	}
}

func TestRuntimeCloseIdempotent(t *testing.T) {
	logger := zaptest.NewLogger(t)
	ctx := context.Background()

	runtime, err := NewRuntime(ctx, logger, nil)
	if err != nil {
		t.Fatal(err)
	}

	// Close multiple times should not error.
	if err := runtime.Close(ctx); err != nil {
		t.Errorf("First close failed: %v", err)
	}
	if err := runtime.Close(ctx); err != nil {
		t.Errorf("Second close failed: %v", err)
	}
}

func TestDefaultRuntimeConfig(t *testing.T) {
	config := DefaultRuntimeConfig()

	if config.MemoryPages != 256 {
		t.Errorf("Default memory pages = %d, want 256", config.MemoryPages)
	}

	if config.DebugEnabled {
		t.Error("Debug should be disabled by default")
	}

	if config.MaxInstances != 100 {
		t.Errorf("Default max instances = %d, want 100", config.MaxInstances)
	}
}

func TestRuntimeConfiguration(t *testing.T) {
	logger := zaptest.NewLogger(t)
	ctx := context.Background()

	config := &RuntimeConfig{
		MemoryPages:  128,
		DebugEnabled: true,
		MaxInstances: 50,
	}

	runtime, err := NewRuntime(ctx, logger, config)
	if err != nil {
		t.Fatalf("Failed to create runtime: %v", err)
	}

	if runtime.config.MemoryPages != 128 {
		t.Errorf("Memory pages not set correctly")
	}

	runtime.Close(ctx)
}

func TestRuntimeContextCancellation(t *testing.T) {
	logger := zaptest.NewLogger(t)
	ctx, cancel := context.WithCancel(context.Background())

	runtime, err := NewRuntime(ctx, logger, nil)
	if err != nil {
		t.Fatal(err)
	}

	// Cancel context.
	cancel()

	// Close with cancelled context.
	err = runtime.Close(ctx)
	// wazero should handle cancelled context gracefully
	if err != nil && err != context.Canceled {
		t.Errorf("Unexpected error when closing with cancelled context: %v", err)
	}
}

func TestRuntimeModuleCache(t *testing.T) {
	logger := zaptest.NewLogger(t)
	ctx := context.Background()

	runtime, err := NewRuntime(ctx, logger, nil)
	if err != nil {
		t.Fatal(err)
	}
	defer runtime.Close(ctx)

	// Test storing and retrieving compiled modules.
	module := &CompiledModule{
		Name:       "test-module",
		Source:     "test",
		SizeBytes:  1024,
		CompiledAt: time.Now().Unix(),
	}

	runtime.StoreCompiledModule(module)

	retrieved, ok := runtime.GetCompiledModule("test-module")
	if !ok {
		t.Fatal("Failed to retrieve module from cache")
	}

	if retrieved.Name != "test-module" {
		t.Errorf("Retrieved wrong module: %s", retrieved.Name)
	}
}

func TestRuntimeInstanceTracking(t *testing.T) {
	logger := zaptest.NewLogger(t)
	ctx := context.Background()

	runtime, err := NewRuntime(ctx, logger, nil)
	if err != nil {
		t.Fatal(err)
	}
	defer runtime.Close(ctx)

	// Test storing and retrieving instances.
	instanceID := "test-instance"
	instanceData := "test-data"

	runtime.StoreInstance(instanceID, instanceData)

	retrieved, ok := runtime.GetInstance(instanceID)
	if !ok {
		t.Fatal("Failed to retrieve instance from tracking")
	}

	if retrieved != instanceData {
		t.Errorf("Retrieved wrong instance data")
	}

	// Test deletion.
	runtime.DeleteInstance(instanceID)

	_, ok = runtime.GetInstance(instanceID)
	if ok {
		t.Error("Instance should have been deleted")
	}
}

func TestRuntimeIsClosed(t *testing.T) {
	logger := zaptest.NewLogger(t)
	ctx := context.Background()

	runtime, err := NewRuntime(ctx, logger, nil)
	if err != nil {
		t.Fatal(err)
	}

	if runtime.IsClosed() {
		t.Error("Runtime should not be closed initially")
	}

	runtime.Close(ctx)

	if !runtime.IsClosed() {
		t.Error("Runtime should be closed after Close()")
	}
}

func TestCompilationError(t *testing.T) {
	err := &CompilationError{
		ModuleName: "test",
		Err:        &testError{},
	}

	expected := "failed to compile Wasm module 'test': test error"
	if err.Error() != expected {
		t.Errorf("Error message = %s, want %s", err.Error(), expected)
	}
}

func TestInstantiationError(t *testing.T) {
	err := &InstantiationError{
		ModuleName: "test",
		InstanceID: "inst-1",
		Err:        &testError{},
	}

	expected := "failed to instantiate module 'test' (instance: inst-1): test error"
	if err.Error() != expected {
		t.Errorf("Error message = %s, want %s", err.Error(), expected)
	}
}

func TestModuleNotFoundError(t *testing.T) {
	err := &ModuleNotFoundError{ModuleName: "test"}

	expected := "module 'test' not found in cache"
	if err.Error() != expected {
		t.Errorf("Error message = %s, want %s", err.Error(), expected)
	}
}

func TestFunctionNotFoundError(t *testing.T) {
	err := &FunctionNotFoundError{
		ModuleName:   "test",
		FunctionName: "parse",
	}

	expected := "function 'parse' not found in module 'test'"
	if err.Error() != expected {
		t.Errorf("Error message = %s, want %s", err.Error(), expected)
	}
}

// testError is a simple error for testing.
type testError struct{}

func (e *testError) Error() string {
	return "test error"
}
