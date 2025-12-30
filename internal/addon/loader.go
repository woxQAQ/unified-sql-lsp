package addon

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"github.com/woxQAQ/unified-sql-lsp/internal/wasm"
	"go.uber.org/zap"
)

// Loader handles loading add-ons from disk.
type Loader struct {
	runtime     *wasm.Runtime
	moduleLoader *wasm.ModuleLoader
	logger      *zap.Logger
}

// NewLoader creates a new add-on loader.
func NewLoader(runtime *wasm.Runtime, logger *zap.Logger) *Loader {
	return &Loader{
		runtime:      runtime,
		moduleLoader: wasm.NewModuleLoader(runtime, logger),
		logger:       logger.With(zap.String("component", "addon-loader")),
	}
}

// LoadAddon loads a single add-on from a directory.
func (l *Loader) LoadAddon(ctx context.Context, dir string) (*Addon, error) {
	l.logger.Debug("Loading add-on", zap.String("dir", dir))

	// Parse manifest
	manifest, err := ParseManifest(dir)
	if err != nil {
		return nil, err
	}

	l.logger.Info("Loading add-on",
		zap.String("name", manifest.Name),
		zap.String("version", manifest.Version),
		zap.String("engine", manifest.Engine),
	)

	// Compile Wasm module (uses internal caching)
	wasmPath := manifest.WasmPath()
	compiled, err := l.moduleLoader.LoadModuleFromFile(ctx, wasmPath)
	if err != nil {
		return nil, &AddonLoadError{
			AddonName: manifest.Name,
			Err:       err,
		}
	}

	// Create add-on instance
	addon := &Addon{
		Manifest:  manifest,
		Compiled:  compiled,
		LoadedAt:  time.Now(),
	}

	l.logger.Info("Add-on loaded successfully",
		zap.String("name", manifest.Name),
		zap.Int64("size_bytes", compiled.SizeBytes),
	)

	return addon, nil
}

// DiscoverAddons scans directories for add-ons.
func (l *Loader) DiscoverAddons(ctx context.Context, paths []string) ([]*Addon, error) {
	var addons []*Addon
	var errs []error

	for _, basePath := range paths {
		l.logger.Debug("Scanning add-on directory", zap.String("path", basePath))

		// Read subdirectories
		entries, err := os.ReadDir(basePath)
		if err != nil {
			if os.IsNotExist(err) {
				l.logger.Warn("Add-on path does not exist", zap.String("path", basePath))
				continue
			}
			return nil, fmt.Errorf("failed to read directory '%s': %w", basePath, err)
		}

		// Try to load each subdirectory as an add-on
		for _, entry := range entries {
			if !entry.IsDir() {
				continue
			}

			addonDir := filepath.Join(basePath, entry.Name())

			addon, err := l.LoadAddon(ctx, addonDir)
			if err != nil {
				l.logger.Error("Failed to load add-on",
					zap.String("dir", addonDir),
					zap.Error(err),
				)
				errs = append(errs, err)
				continue
			}

			addons = append(addons, addon)
		}
	}

	// If we found some add-ons but had errors, log warning but continue
	if len(addons) > 0 && len(errs) > 0 {
		l.logger.Warn("Some add-ons failed to load",
			zap.Int("loaded", len(addons)),
			zap.Int("failed", len(errs)),
		)
	}

	// If no add-ons loaded, return error
	if len(addons) == 0 {
		return nil, &NoAddonsFoundError{Paths: paths}
	}

	return addons, nil
}
