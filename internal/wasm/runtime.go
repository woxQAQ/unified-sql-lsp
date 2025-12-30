package wasm

import (
	"context"
	"sync"

	"github.com/tetratelabs/wazero"
	"go.uber.org/zap"
)

// Runtime manages the wazero runtime lifecycle.
// It's a singleton that creates a single wazero.Runtime for the entire application.
type Runtime struct {
	// wazero runtime (singleton)
	runtime wazero.Runtime

	// Compiled module cache (key: module name/path -> value: compiled module)
	// This avoids recompiling the same Wasm binary multiple times
	modules sync.Map // map[string]*CompiledModule

	// Active module instances (for cleanup on shutdown)
	// key: instance ID -> value: api.Module
	instances sync.Map // map[string]interface{}(wazero API module)

	// Configuration
	config *RuntimeConfig

	// Logger
	logger *zap.Logger

	// Shutdown management
	closeOnce sync.Once
	closed    chan struct{}
}

// RuntimeConfig holds runtime configuration.
type RuntimeConfig struct {
	// Memory limits for Wasm modules (in pages, 64KB each)
	// Default: 256 pages = 16MB max memory per module
	MemoryPages uint32

	// Enable debug logging for Wasm execution
	DebugEnabled bool

	// Compilation cache directory (for persistent caching)
	// If empty, uses in-memory caching only
	CacheDir string

	// Maximum number of concurrent instances
	MaxInstances int
}

// CompiledModule wraps a wazero.CompiledModule with metadata.
type CompiledModule struct {
	// wazero compiled module
	Module wazero.CompiledModule

	// Module metadata
	Name      string
	Source    string // File path or identifier
	SizeBytes int64

	// Compilation timestamp
	CompiledAt int64
}

// NewRuntime creates and initializes a new wazero runtime.
// This should be called once during application startup.
func NewRuntime(ctx context.Context, logger *zap.Logger, config *RuntimeConfig) (*Runtime, error) {
	// Validate config
	if config == nil {
		config = DefaultRuntimeConfig()
	}

	// Create wazero runtime with context
	r := wazero.NewRuntime(ctx)

	runtime := &Runtime{
		runtime: r,
		config:  config,
		logger:  logger.With(zap.String("component", "wasm-runtime")),
		closed:  make(chan struct{}),
	}

	logger.Info("Wasm runtime initialized",
		zap.Uint32("memory_pages", config.MemoryPages),
		zap.Bool("debug_enabled", config.DebugEnabled),
		zap.String("cache_dir", config.CacheDir),
		zap.Int("max_instances", config.MaxInstances),
	)

	return runtime, nil
}

// DefaultRuntimeConfig returns sensible defaults.
func DefaultRuntimeConfig() *RuntimeConfig {
	return &RuntimeConfig{
		MemoryPages:  256, // 16MB
		DebugEnabled: false,
		CacheDir:     "",
		MaxInstances: 100,
	}
}

// Close gracefully shuts down the runtime.
// Safe to call multiple times (idempotent).
func (r *Runtime) Close(ctx context.Context) error {
	var err error
	r.closeOnce.Do(func() {
		r.logger.Info("Shutting down Wasm runtime")

		// Close all active instances first
		r.instances.Range(func(key, value interface{}) bool {
			if inst, ok := value.(interface{ Close(context.Context) error }); ok {
				if closeErr := inst.Close(ctx); closeErr != nil {
					r.logger.Warn("Failed to close instance",
						zap.String("instance_id", key.(string)),
						zap.Error(closeErr),
					)
				}
			}
			return true
		})

		// Close the runtime (closes compiled modules)
		err = r.runtime.Close(ctx)

		close(r.closed)
		r.logger.Info("Wasm runtime shutdown complete")
	})

	return err
}

// GetCompiledModule retrieves a compiled module from cache.
func (r *Runtime) GetCompiledModule(name string) (*CompiledModule, bool) {
	if val, ok := r.modules.Load(name); ok {
		if mod, ok := val.(*CompiledModule); ok {
			return mod, true
		}
	}
	return nil, false
}

// StoreCompiledModule stores a compiled module in cache.
func (r *Runtime) StoreCompiledModule(module *CompiledModule) {
	r.modules.Store(module.Name, module)
}

// GetInstance retrieves an active instance.
func (r *Runtime) GetInstance(instanceID string) (interface{}, bool) {
	return r.instances.Load(instanceID)
}

// StoreInstance stores an active instance.
func (r *Runtime) StoreInstance(instanceID string, instance interface{}) {
	r.instances.Store(instanceID, instance)
}

// DeleteInstance removes an instance from tracking.
func (r *Runtime) DeleteInstance(instanceID string) {
	r.instances.Delete(instanceID)
}

// IsClosed returns whether the runtime has been closed.
func (r *Runtime) IsClosed() bool {
	select {
	case <-r.closed:
		return true
	default:
		return false
	}
}
