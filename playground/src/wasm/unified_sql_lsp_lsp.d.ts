// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

/**
 * TypeScript definitions for SQL LSP WASM module
 *
 * This file will be replaced by wasm-pack generated output
 * once the WASM build is configured.
 */

export function __wbg_lspserver_free(ptr: number): void;
export function __wbg_lspserver_completion(ptr: number, text_ptr: number, text_len: number, line: number, col: number): number;
export function __wbg_lspserver_hover(ptr: number, text_ptr: number, text_len: number, line: number, col: number): number;
export function __wbg_lspserver_diagnostics(ptr: number, text_ptr: number, text_len: number): number;

export class LspServer {
  constructor(dialect: string);
  completion(text: string, line: number, col: number): string;
  hover(text: string, line: number, col: number): string;
  diagnostics(text: string): string;
}
