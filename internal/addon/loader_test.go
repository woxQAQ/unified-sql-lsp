package addon

import (
	"context"
	"path/filepath"
	"testing"

	"github.com/woxQAQ/unified-sql-lsp/internal/wasm"
	"go.uber.org/zap"
)

func TestLoader_LoadAddon_Valid(t *testing.T) {
	t.Skip("Requires valid Wasm binary - will be tested in integration tests")

	ctx := context.Background()
	logger := zap.NewNop()

	// Create runtime
	runtime, err := wasm.NewRuntime(ctx, logger, wasm.DefaultRuntimeConfig())
	if err != nil {
		t.Fatalf("Failed to create runtime: %v", err)
	}
	defer runtime.Close(ctx)

	loader := NewLoader(runtime, logger)
	dir := filepath.Join("testdata", "addons", "valid-postgresql")

	addon, err := loader.LoadAddon(ctx, dir)
	if err != nil {
		t.Fatalf("LoadAddon() failed: %v", err)
	}

	if addon.Name() != "postgresql" {
		t.Errorf("expected name 'postgresql', got '%s'", addon.Name())
	}

	if addon.Engine() != "PostgreSQL" {
		t.Errorf("expected engine 'PostgreSQL', got '%s'", addon.Engine())
	}

	if addon.Version() != "1.0.0" {
		t.Errorf("expected version '1.0.0', got '%s'", addon.Version())
	}

	if !addon.SupportsVersion("14") {
		t.Error("expected to support version 14")
	}

	capabilities := addon.Capabilities()
	if len(capabilities) != 2 {
		t.Errorf("expected 2 capabilities, got %d", len(capabilities))
	}
}

func TestLoader_LoadAddon_ManifestNotFound(t *testing.T) {
	ctx := context.Background()
	logger := zap.NewNop()

	runtime, err := wasm.NewRuntime(ctx, logger, wasm.DefaultRuntimeConfig())
	if err != nil {
		t.Fatalf("Failed to create runtime: %v", err)
	}
	defer runtime.Close(ctx)

	loader := NewLoader(runtime, logger)
	dir := filepath.Join("testdata", "addons", "nonexistent")

	_, err = loader.LoadAddon(ctx, dir)
	if err == nil {
		t.Fatal("LoadAddon() should fail for nonexistent directory")
	}

	_, ok := err.(*ManifestNotFoundError)
	if !ok {
		t.Errorf("expected ManifestNotFoundError, got %T", err)
	}
}

func TestLoader_LoadAddon_InvalidManifest(t *testing.T) {
	ctx := context.Background()
	logger := zap.NewNop()

	runtime, err := wasm.NewRuntime(ctx, logger, wasm.DefaultRuntimeConfig())
	if err != nil {
		t.Fatalf("Failed to create runtime: %v", err)
	}
	defer runtime.Close(ctx)

	loader := NewLoader(runtime, logger)
	dir := filepath.Join("testdata", "addons", "missing-fields")

	_, err = loader.LoadAddon(ctx, dir)
	if err == nil {
		t.Fatal("LoadAddon() should fail for invalid manifest")
	}

	_, ok := err.(*ManifestValidationError)
	if !ok {
		t.Errorf("expected ManifestValidationError, got %T", err)
	}
}

func TestLoader_LoadAddon_WasmNotFound(t *testing.T) {
	ctx := context.Background()
	logger := zap.NewNop()

	runtime, err := wasm.NewRuntime(ctx, logger, wasm.DefaultRuntimeConfig())
	if err != nil {
		t.Fatalf("Failed to create runtime: %v", err)
	}
	defer runtime.Close(ctx)

	loader := NewLoader(runtime, logger)
	dir := filepath.Join("testdata", "addons", "missing-wasm")

	_, err = loader.LoadAddon(ctx, dir)
	if err == nil {
		t.Fatal("LoadAddon() should fail for missing Wasm file")
	}

	_, ok := err.(*WasmNotFoundError)
	if !ok {
		t.Errorf("expected WasmNotFoundError, got %T", err)
	}
}

func TestLoader_DiscoverAddons_EmptyDir(t *testing.T) {
	ctx := context.Background()
	logger := zap.NewNop()

	runtime, err := wasm.NewRuntime(ctx, logger, wasm.DefaultRuntimeConfig())
	if err != nil {
		t.Fatalf("Failed to create runtime: %v", err)
	}
	defer runtime.Close(ctx)

	loader := NewLoader(runtime, logger)

	// Use a directory that exists but has no valid add-ons
	_, err = loader.DiscoverAddons(ctx, []string{"testdata/addons/invalid-yaml"})
	if err == nil {
		t.Fatal("DiscoverAddons() should fail when no add-ons found")
	}

	_, ok := err.(*NoAddonsFoundError)
	if !ok {
		t.Errorf("expected NoAddonsFoundError, got %T", err)
	}
}

func TestLoader_DiscoverAddons_PathNotExist(t *testing.T) {
	ctx := context.Background()
	logger := zap.NewNop()

	runtime, err := wasm.NewRuntime(ctx, logger, wasm.DefaultRuntimeConfig())
	if err != nil {
		t.Fatalf("Failed to create runtime: %v", err)
	}
	defer runtime.Close(ctx)

	loader := NewLoader(runtime, logger)

	// Should return error when no add-ons found
	_, err = loader.DiscoverAddons(ctx, []string{"/nonexistent/path"})
	if err == nil {
		t.Fatal("DiscoverAddons() should fail when path doesn't exist")
	}

	_, ok := err.(*NoAddonsFoundError)
	if !ok {
		t.Errorf("expected NoAddonsFoundError, got %T", err)
	}
}
