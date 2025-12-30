//go:build wasm

package wasm

// This file defines the Wasm export interface for add-ons
// Add-ons must implement these functions using //go:wasmexport

import (
	"runtime"
)

// Exported functions that add-ons must implement:
//
// //go:wasmexport parse
// func parse(ptr, length uint32) uint32
//
// //go:wasmexport complete
// func complete(contextPtr, contextLen uint32) uint32
//
// //go:wasmexport metadata
// func metadata() (ptr, length uint32)

// Ensure this is compiled as Wasm
func init() {
	runtime.GC()
}
