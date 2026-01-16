// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Database adapter module

pub mod adapter;

pub use adapter::{DatabaseAdapter, MySQLAdapter, PostgreSQLAdapter, adapter_from_test_path};
