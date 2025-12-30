package wasm

import (
	"fmt"
	"time"
)

// CompilationError occurs when Wasm module compilation fails
type CompilationError struct {
	ModuleName string
	Err        error
}

func (e *CompilationError) Error() string {
	return fmt.Sprintf("failed to compile Wasm module '%s': %v", e.ModuleName, e.Err)
}

func (e *CompilationError) Unwrap() error {
	return e.Err
}

// InstantiationError occurs when module instantiation fails
type InstantiationError struct {
	ModuleName string
	InstanceID string
	Err        error
}

func (e *InstantiationError) Error() string {
	return fmt.Sprintf("failed to instantiate module '%s' (instance: %s): %v",
		e.ModuleName, e.InstanceID, e.Err)
}

func (e *InstantiationError) Unwrap() error {
	return e.Err
}

// ModuleNotFoundError occurs when a module is not in cache
type ModuleNotFoundError struct {
	ModuleName string
}

func (e *ModuleNotFoundError) Error() string {
	return fmt.Sprintf("module '%s' not found in cache", e.ModuleName)
}

// FunctionNotFoundError occurs when an exported function is missing
type FunctionNotFoundError struct {
	ModuleName   string
	FunctionName string
}

func (e *FunctionNotFoundError) Error() string {
	return fmt.Sprintf("function '%s' not found in module '%s'",
		e.FunctionName, e.ModuleName)
}

// MemoryAccessError occurs when memory operations fail
type MemoryAccessError struct {
	Operation string
	Address   uint32
	Length    uint32
	Err       error
}

func (e *MemoryAccessError) Error() string {
	return fmt.Sprintf("memory access failed (op=%s, addr=%d, len=%d): %v",
		e.Operation, e.Address, e.Length, e.Err)
}

func (e *MemoryAccessError) Unwrap() error {
	return e.Err
}

// HostFunctionError occurs when host function execution fails
type HostFunctionError struct {
	FunctionName string
	Err          error
}

func (e *HostFunctionError) Error() string {
	return fmt.Sprintf("host function '%s' failed: %v", e.FunctionName, e.Err)
}

func (e *HostFunctionError) Unwrap() error {
	return e.Err
}

// TimeoutError occurs when Wasm execution times out
type TimeoutError struct {
	Duration time.Duration
}

func (e *TimeoutError) Error() string {
	return fmt.Sprintf("Wasm execution timed out after %v", e.Duration)
}
