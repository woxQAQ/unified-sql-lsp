package wasm

import (
	"context"

	"github.com/tetratelabs/wazero/api"
	"go.uber.org/zap"
)

// HostFunctionsImpl implements host functions for Wasm modules.
type HostFunctionsImpl struct {
	logger *zap.Logger
}

// NewHostFunctions creates a new host functions implementation.
func NewHostFunctions(logger *zap.Logger) *HostFunctionsImpl {
	return &HostFunctionsImpl{
		logger: logger.With(zap.String("component", "wasm-host")),
	}
}

// logMessage is called by Wasm modules to log messages.
// Signature: log_message(level, ptr, length)
// level: 0 = debug, 1 = info, 2 = warn, 3 = error
func (h *HostFunctionsImpl) logMessage(ctx context.Context, mod api.Module, level uint32, ptr uint32, length uint32) {
	// Read message from Wasm memory.
	msg, ok := mod.Memory().Read(ptr, length)
	if !ok {
		h.logger.Error("Failed to read log message from Wasm memory",
			zap.Uint32("ptr", ptr),
			zap.Uint32("length", length),
		)
		return
	}

	// Log based on level.
	// 0 = debug, 1 = info, 2 = warn, 3 = error
	switch level {
	case 0:
		h.logger.Debug(string(msg))
	case 1:
		h.logger.Info(string(msg))
	case 2:
		h.logger.Warn(string(msg))
	case 3:
		h.logger.Error(string(msg))
	default:
		h.logger.Info(string(msg))
	}
}

// getSchema is called by Wasm modules to query schema.
// Returns pointer to SchemaInfo serialized in Wasm memory.
// TODO: Implement in F012 (PostgreSQL Schema Introspection)
func (h *HostFunctionsImpl) getSchema(ctx context.Context, mod api.Module, dbPtr uint32, dbLen uint32) {
	h.logger.Warn("get_schema not yet implemented")
	// For now, just log.
	// In F012, this will:
	// 1. Read database name from Wasm memory (dbPtr, dbLen)
	// 2. Query information_schema
	// 3. Serialize schema info to JSON
	// 4. Allocate memory in Wasm module
	// 5. Write schema info to memory
	// 6. Return pointer to schema info via result
}

// executeQuery is called by Wasm modules to execute SQL queries.
// TODO: Implement in F012
func (h *HostFunctionsImpl) executeQuery(ctx context.Context, mod api.Module, dbPtr uint32, dbLen uint32) {
	h.logger.Warn("execute_query not yet implemented")
	// For now, just log.
	// In F012, this will:
	// 1. Read database connection info from Wasm memory
	// 2. Read SQL query from Wasm memory
	// 3. Execute query
	// 4. Serialize results
	// 5. Write results to Wasm memory
	// 6. Return pointer to results via result
}
