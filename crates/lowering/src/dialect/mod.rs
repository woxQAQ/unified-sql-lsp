// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Dialect-specific lowering implementations

pub mod base;
pub mod mysql;
pub mod postgresql;
pub mod shared;

pub use base::DialectLoweringBase;
pub use mysql::MySQLLowering;
pub use postgresql::PostgreSQLLowering;
pub use shared::SharedLowering;
