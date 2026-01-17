// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! E2E Test Library (Legacy Re-exports)
//!
//! This crate now re-exports from unified-sql-lsp-e2e-core for backward compatibility.

pub use unified_sql_lsp_e2e_core::*;

// Re-export procedural macros
pub use unified_sql_lsp_e2e_core_macros::generate_engine_tests;

// Re-export engine_manager for backward compatibility
pub use engine_manager::{Engine, ensure_engine_ready};
