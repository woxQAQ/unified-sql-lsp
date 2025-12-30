package wasm

import (
	"context"
	"fmt"
	"os"
	"time"

	"go.uber.org/zap"
)

// ModuleLoader handles loading and compiling Wasm modules.
type ModuleLoader struct {
	runtime *Runtime
	logger  *zap.Logger
}

// NewModuleLoader creates a new module loader.
func NewModuleLoader(runtime *Runtime, logger *zap.Logger) *ModuleLoader {
	return &ModuleLoader{
		runtime: runtime,
		logger:  logger.With(zap.String("component", "wasm-loader")),
	}
}

// ModuleSource represents a source for Wasm bytecode.
type ModuleSource interface {
	// Bytes returns the Wasm bytecode.
	Bytes() ([]byte, error)

	// Name returns a name/identifier for this module.
	Name() string

	// Size returns the size in bytes.
	Size() int64
}

// FileModuleSource loads Wasm from a file.
type FileModuleSource struct {
	Path string
}

// Bytes reads the Wasm file.
func (f *FileModuleSource) Bytes() ([]byte, error) {
	return os.ReadFile(f.Path)
}

// Name returns the file path as the module name.
func (f *FileModuleSource) Name() string {
	return f.Path
}

// Size returns the file size.
func (f *FileModuleSource) Size() int64 {
	info, err := os.Stat(f.Path)
	if err != nil {
		return 0
	}
	return info.Size()
}

// MemoryModuleSource loads Wasm from memory.
type MemoryModuleSource struct {
	ModuleName string
	Data       []byte
}

// Bytes returns the Wasm bytecode.
func (m *MemoryModuleSource) Bytes() ([]byte, error) {
	return m.Data, nil
}

// Name returns the module name.
func (m *MemoryModuleSource) Name() string {
	return m.ModuleName
}

// Size returns the data size.
func (m *MemoryModuleSource) Size() int64 {
	return int64(len(m.Data))
}

// LoadModule loads a Wasm module from a source.
// Compiles it if not already cached.
func (l *ModuleLoader) LoadModule(ctx context.Context, source ModuleSource) (*CompiledModule, error) {
	// Check cache first
	if cached, ok := l.runtime.GetCompiledModule(source.Name()); ok {
		l.logger.Debug("Module cache hit",
			zap.String("module", source.Name()),
		)
		return cached, nil
	}

	// Load Wasm bytes
	wasmBytes, err := source.Bytes()
	if err != nil {
		return nil, fmt.Errorf("failed to read module %s: %w", source.Name(), err)
	}

	// Compile the module
	l.logger.Info("Compiling Wasm module",
		zap.String("module", source.Name()),
		zap.Int64("size_bytes", source.Size()),
	)

	startTime := time.Now()

	// wazero.CompileModule decodes and validates the Wasm binary
	// This is CPU-intensive but only done once per module
	compiled, err := l.runtime.runtime.CompileModule(ctx, wasmBytes)
	if err != nil {
		return nil, &CompilationError{
			ModuleName: source.Name(),
			Err:        err,
		}
	}

	duration := time.Since(startTime)

	// Wrap with metadata
	compiledModule := &CompiledModule{
		Module:     compiled,
		Name:       source.Name(),
		Source:     source.Name(),
		SizeBytes:  source.Size(),
		CompiledAt: time.Now().Unix(),
	}

	// Cache the compiled module
	l.runtime.StoreCompiledModule(compiledModule)

	l.logger.Info("Module compiled successfully",
		zap.String("module", source.Name()),
		zap.Duration("duration", duration),
	)

	return compiledModule, nil
}

// LoadModuleFromFile is a convenience function for loading from a file path.
func (l *ModuleLoader) LoadModuleFromFile(ctx context.Context, path string) (*CompiledModule, error) {
	source := &FileModuleSource{Path: path}
	return l.LoadModule(ctx, source)
}

// LoadModuleFromMemory loads from a byte slice.
func (l *ModuleLoader) LoadModuleFromMemory(ctx context.Context, name string, data []byte) (*CompiledModule, error) {
	source := &MemoryModuleSource{ModuleName: name, Data: data}
	return l.LoadModule(ctx, source)
}
