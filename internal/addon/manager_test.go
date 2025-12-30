package addon

import (
	"context"
	"testing"

	"github.com/woxQAQ/unified-sql-lsp/internal/config"
	"github.com/woxQAQ/unified-sql-lsp/internal/wasm"
	"go.uber.org/zap"
)

func TestManager_NewManager(t *testing.T) {
	ctx := context.Background()
	logger := zap.NewNop()

	// Create runtime
	runtime, err := wasm.NewRuntime(ctx, logger, wasm.DefaultRuntimeConfig())
	if err != nil {
		t.Fatalf("Failed to create runtime: %v", err)
	}
	defer runtime.Close(ctx)

	// Create host functions
	hostFuncs := wasm.NewHostFunctions(logger)

	// Create config
	cfg := &config.ServerConfig{
		AddonPaths: []string{"/tmp/addons"},
	}

	// Create manager
	manager := NewManager(cfg, runtime, hostFuncs, logger)

	if manager == nil {
		t.Fatal("NewManager() returned nil")
	}

	if manager.IsLoaded() {
		t.Error("Manager should not be loaded initially")
	}
}

func TestManager_GetAddon_NotFound(t *testing.T) {
	ctx := context.Background()
	logger := zap.NewNop()

	runtime, err := wasm.NewRuntime(ctx, logger, wasm.DefaultRuntimeConfig())
	if err != nil {
		t.Fatalf("Failed to create runtime: %v", err)
	}
	defer runtime.Close(ctx)

	hostFuncs := wasm.NewHostFunctions(logger)
	cfg := &config.ServerConfig{}
	manager := NewManager(cfg, runtime, hostFuncs, logger)

	// Try to get non-existent add-on
	_, err = manager.GetAddon("nonexistent")
	if err == nil {
		t.Fatal("GetAddon() should fail for non-existent add-on")
	}

	_, ok := err.(*AddonNotFoundError)
	if !ok {
		t.Errorf("expected AddonNotFoundError, got %T", err)
	}
}

func TestManager_FindAddonForEngine_NotFound(t *testing.T) {
	ctx := context.Background()
	logger := zap.NewNop()

	runtime, err := wasm.NewRuntime(ctx, logger, wasm.DefaultRuntimeConfig())
	if err != nil {
		t.Fatalf("Failed to create runtime: %v", err)
	}
	defer runtime.Close(ctx)

	hostFuncs := wasm.NewHostFunctions(logger)
	cfg := &config.ServerConfig{}
	manager := NewManager(cfg, runtime, hostFuncs, logger)

	// Try to find add-on for non-existent engine
	_, err = manager.FindAddonForEngine("PostgreSQL")
	if err == nil {
		t.Fatal("FindAddonForEngine() should fail when no add-ons found")
	}
}

func TestManager_Shutdown(t *testing.T) {
	ctx := context.Background()
	logger := zap.NewNop()

	runtime, err := wasm.NewRuntime(ctx, logger, wasm.DefaultRuntimeConfig())
	if err != nil {
		t.Fatalf("Failed to create runtime: %v", err)
	}

	hostFuncs := wasm.NewHostFunctions(logger)
	cfg := &config.ServerConfig{}
	manager := NewManager(cfg, runtime, hostFuncs, logger)

	// Shutdown should work even without loaded add-ons
	err = manager.Shutdown(ctx)
	if err != nil {
		t.Errorf("Shutdown() failed: %v", err)
	}

	// Runtime should be closed
	if !runtime.IsClosed() {
		t.Error("Runtime should be closed after shutdown")
	}
}
