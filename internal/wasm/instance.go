package wasm

import (
	"context"
	"fmt"
	"time"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
	"go.uber.org/zap"
)

// InstanceManager creates and manages module instances.
type InstanceManager struct {
	runtime   *Runtime
	logger    *zap.Logger
	hostFuncs *HostFunctionsImpl
}

// NewInstanceManager creates a new instance manager.
func NewInstanceManager(runtime *Runtime, hostFuncs *HostFunctionsImpl, logger *zap.Logger) *InstanceManager {
	return &InstanceManager{
		runtime:   runtime,
		hostFuncs: hostFuncs,
		logger:    logger.With(zap.String("component", "wasm-instance")),
	}
}

// InstanceConfig holds configuration for creating instances.
type InstanceConfig struct {
	// Module name to instantiate.
	ModuleName string

	// Instance ID (if empty, generates UUID).
	InstanceID string

	// Context for cancellation.
	Context context.Context
}

// Instance represents an instantiated Wasm module.
type Instance struct {
	// wazero module instance.
	module api.Module

	// Instance metadata.
	ID        string
	Name      string
	CreatedAt int64

	// Exported functions (cached for performance).
	exports map[string]api.Function
}

// Instantiate creates a new instance from a compiled module.
// Host functions are exported to the Wasm module.
func (m *InstanceManager) Instantiate(ctx context.Context, config *InstanceConfig) (*Instance, error) {
	// Get compiled module from cache.
	compiledVal, ok := m.runtime.GetCompiledModule(config.ModuleName)
	if !ok {
		return nil, &ModuleNotFoundError{ModuleName: config.ModuleName}
	}

	compiled := compiledVal

	// Generate instance ID if not provided.
	instanceID := config.InstanceID
	if instanceID == "" {
		instanceID = generateUUID()
	}

	m.logger.Info("Instantiating Wasm module",
		zap.String("module", config.ModuleName),
		zap.String("instance_id", instanceID),
	)

	// Build host module with exported functions.
	hostBuilder := m.runtime.runtime.NewHostModuleBuilder("host")

	// Export host functions.
	if err := m.exportHostFunctions(hostBuilder); err != nil {
		return nil, fmt.Errorf("failed to export host functions: %w", err)
	}

	// Compile host module (only done once).
	if _, err := hostBuilder.Compile(ctx); err != nil {
		return nil, fmt.Errorf("failed to compile host module: %w", err)
	}

	// Instantiate the guest module with host functions.
	// This creates a sandboxed execution environment.
	moduleConfig := wazero.NewModuleConfig().
		WithName(instanceID).
		WithStartFunctions() // Call _start if present

	module, err := m.runtime.runtime.InstantiateModule(ctx, compiled.Module, moduleConfig)
	if err != nil {
		return nil, &InstantiationError{
			ModuleName: config.ModuleName,
			InstanceID: instanceID,
			Err:        err,
		}
	}

	// Cache exported functions.
	exports := m.cacheExportedFunctions(module)

	// Create instance wrapper.
	instance := &Instance{
		module:    module,
		ID:        instanceID,
		Name:      config.ModuleName,
		CreatedAt: time.Now().Unix(),
		exports:   exports,
	}

	// Track active instance.
	m.runtime.StoreInstance(instanceID, module)

	m.logger.Info("Module instantiated successfully",
		zap.String("instance_id", instanceID),
		zap.Int("exported_functions", len(exports)),
	)

	return instance, nil
}

// Close closes the instance and releases resources.
func (i *Instance) Close(ctx context.Context) error {
	return i.module.Close(ctx)
}

// cacheExportedFunctions caches references to exported functions.
// This improves performance by avoiding repeated lookups.
func (m *InstanceManager) cacheExportedFunctions(module api.Module) map[string]api.Function {
	exports := make(map[string]api.Function)

	// Cache the standard add-on functions.
	for _, name := range []string{"parse", "complete", "metadata"} {
		if fn := module.ExportedFunction(name); fn != nil {
			exports[name] = fn
		}
	}

	return exports
}

// exportHostFunctions registers Go functions for import by Wasm modules.
func (m *InstanceManager) exportHostFunctions(builder wazero.HostModuleBuilder) error {
	impl := m.hostFuncs

	// Export log_message function.
	// Wasm modules can call this to log messages.
	builder.NewFunctionBuilder().
		WithFunc(impl.logMessage).
		WithParameterNames("level", "ptr", "length").
		Export("log_message")

	// Export get_schema function.
	// Wasm modules can call this to query database schema.
	builder.NewFunctionBuilder().
		WithFunc(impl.getSchema).
		WithParameterNames("db_ptr", "db_len").
		Export("get_schema")

	// Export execute_query function.
	builder.NewFunctionBuilder().
		WithFunc(impl.executeQuery).
		WithParameterNames("db_ptr", "db_len").
		Export("execute_query")

	return nil
}

// generateUUID generates a unique instance ID.
// For now, using timestamp-based ID. In production, use proper UUID.
func generateUUID() string {
	return fmt.Sprintf("inst-%d", time.Now().UnixNano())
}
