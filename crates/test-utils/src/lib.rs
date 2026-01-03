// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Testing utilities for unified-sql-lsp
//!
//! This crate provides common testing components including:
//! - Mock catalog implementations
//! - CST node builders for lowering tests
//! - SQL-specific test helpers and assertions
//! - Test fixtures and sample data

pub mod assertions;
pub mod fixtures;
pub mod mock_catalog;
pub mod mock_cst;

// Re-exports for convenience
pub use mock_catalog::{MockCatalog, MockCatalogBuilder};
pub use mock_cst::{MockCstBuilder, MockCstNode};
