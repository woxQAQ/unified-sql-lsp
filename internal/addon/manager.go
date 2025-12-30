package addon

import (
	"context"
	"fmt"
	"sync"

	"github.com/woxQAQ/unified-sql-lsp/internal/config"
	"github.com/woxQAQ/unified-sql-lsp/internal/wasm"
	"go.uber.org/zap"
)

// Manager manages add-on lifecycle.
type Manager struct {
	cfg         *config.ServerConfig
	runtime     *wasm.Runtime
	loader      *Loader
	registry    *Registry
	instanceMgr *wasm.InstanceManager
	logger      *zap.Logger

	mu     sync.RWMutex
	loaded bool
}

// NewManager creates a new add-on manager.
func NewManager(
	cfg *config.ServerConfig,
	runtime *wasm.Runtime,
	hostFuncs *wasm.HostFunctionsImpl,
	logger *zap.Logger,
) *Manager {
	return &Manager{
		cfg:         cfg,
		runtime:     runtime,
		loader:      NewLoader(runtime, logger),
		registry:    NewRegistry(logger),
		instanceMgr: wasm.NewInstanceManager(runtime, hostFuncs, logger),
		logger:      logger.With(zap.String("component", "addon-manager")),
	}
}

// LoadAll discovers and loads all add-ons from configured paths.
func (m *Manager) LoadAll(ctx context.Context) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	if m.loaded {
		return fmt.Errorf("add-ons already loaded")
	}

	m.logger.Info("Loading add-ons",
		zap.Strings("paths", m.cfg.AddonPaths),
	)

	// Discover add-ons
	addons, err := m.loader.DiscoverAddons(ctx, m.cfg.AddonPaths)
	if err != nil {
		// Check if it's a NoAddonsFoundError - log warning but don't fail
		if _, ok := err.(*NoAddonsFoundError); ok {
			m.logger.Warn("No add-ons found in configured paths",
				zap.Strings("paths", m.cfg.AddonPaths),
			)
			m.loaded = true
			return nil
		}
		return err
	}

	// Register all add-ons
	for _, addon := range addons {
		if err := m.registry.Register(addon); err != nil {
			m.logger.Error("Failed to register add-on",
				zap.String("name", addon.Manifest.Name),
				zap.Error(err),
			)
			continue
		}
	}

	m.loaded = true

	m.logger.Info("Add-ons loaded successfully",
		zap.Int("count", len(addons)),
	)

	return nil
}

// GetAddon retrieves an add-on by name.
func (m *Manager) GetAddon(name string) (*Addon, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	addon, ok := m.registry.Get(name)
	if !ok {
		return nil, &AddonNotFoundError{AddonName: name}
	}

	return addon, nil
}

// FindAddonForEngine finds an add-on for a database engine.
func (m *Manager) FindAddonForEngine(engine string) (*Addon, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	addons := m.registry.LookupByEngine(engine)
	if len(addons) == 0 {
		return nil, fmt.Errorf("no add-on found for engine '%s'", engine)
	}

	// Return first match (future: support version selection)
	return addons[0], nil
}

// Instantiate creates a new instance of an add-on.
func (m *Manager) Instantiate(ctx context.Context, addonName string) (*wasm.Instance, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	// Get add-on
	addon, ok := m.registry.Get(addonName)
	if !ok {
		return nil, &AddonNotFoundError{AddonName: addonName}
	}

	// Create instance config
	config := &wasm.InstanceConfig{
		ModuleName: addon.Manifest.Name,
		// InstanceID will be auto-generated
		Context: ctx,
	}

	// Instantiate
	instance, err := m.instanceMgr.Instantiate(ctx, config)
	if err != nil {
		return nil, err
	}

	return instance, nil
}

// Shutdown gracefully shuts down all add-ons.
func (m *Manager) Shutdown(ctx context.Context) error {
	m.logger.Info("Shutting down add-on manager")

	// Runtime close handles instance cleanup
	if err := m.runtime.Close(ctx); err != nil {
		m.logger.Error("Failed to shutdown runtime", zap.Error(err))
		return err
	}

	m.logger.Info("Add-on manager shutdown complete")
	return nil
}

// Registry returns the add-on registry (for testing/inspection).
func (m *Manager) Registry() *Registry {
	return m.registry
}

// IsLoaded returns whether add-ons have been loaded.
func (m *Manager) IsLoaded() bool {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.loaded
}
