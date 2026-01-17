// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Build script for E2E test macros
//!
//! This build script enables cargo to track changes in the tests/ directory,
//! ensuring that the macro is recompiled when YAML test files are added or modified.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Track changes in the tests directory so cargo knows to rerun the build script
    // when YAML files are added, removed, or modified
    println!("cargo:rerun-if-changed=tests/");

    // Future enhancement: discover YAML files at compile time and generate
    // a list that can be used by the macro. This will enable fully dynamic
    // test discovery without hardcoding test file names.
    //
    // Planned implementation:
    // 1. Recursively walk tests/ directory
    // 2. Find all *.yaml files matching pattern: tests/{engine}/*/{test_name}.yaml
    // 3. Generate const arrays or compile-time includes for macro consumption
    // 4. Support glob patterns like: tests/mysql-5.7/completion/*.yaml

    // For now, the build script primarily serves as a change tracker.
    // Full YAML discovery will be implemented in a future task to keep
    // changes incremental and testable.

    // Optional: Print debug info in development builds
    if env::var("DEBUG").is_ok() {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let tests_path = Path::new(&manifest_dir).join("tests");
        if tests_path.exists() {
            if let Ok(entries) = fs::read_dir(&tests_path) {
                let count = entries.filter_map(|e| e.ok()).count();
                println!("cargo:warning=Found {} test directories", count);
            }
        }
    }
}
