//go:build wasm

package wasm

// This file defines the Wasm export interface for add-ons
// Add-ons must implement these functions using //go:wasmexport
//
// NOTE: uint32 is used for pointers and lengths because WebAssembly uses a 32-bit
// linear memory model. All Wasm memory addresses are represented as 32-bit integers
// (addresses 0 to 4GB). This ensures compatibility with Wasm's memory architecture.
// See: https://github.com/golang/go/issues/59156

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
