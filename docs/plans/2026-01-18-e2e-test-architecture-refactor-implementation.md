# E2E Test Architecture Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor E2E tests into workspace-level engine isolation for 3-4x speedup and better organization

**Architecture:** Split tests/e2e-rs into workspace with core library crate + per-engine crates. Each engine crate initializes database once and runs all tests. Different engines run in parallel via `cargo test --workspace`.

**Tech Stack:** Rust workspaces, procedural macros, serial_test, tokio, Docker Compose

---

## Phase 1: Prepare Foundation (Non-Breaking)

### Task 1.1: Create workspace structure

**Files:**
- Create: `tests/e2e-rs/Cargo.toml` (workspace root)
- Create: `tests/e2e-rs/core/Cargo.toml`
- Create: `tests/e2e-rs/core/src/lib.rs`

**Step 1: Create workspace Cargo.toml**

Create file `tests/e2e-rs/Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["core"]

[workspace.dependencies]
tokio = { version = "1.40", features = ["full", "rt-multi-thread"] }
anyhow = "1.0"
serial_test = "3.0"
tower-lsp = "0.20"
sqlx = { version = "0.8", features = ["mysql", "postgres", "runtime-tokio"] }
proc-macro2 = "1.0"
quote = "1.0"
syn = { version = "2.0", features = ["full"] }
glob = "0.3"
```

**Step 2: Create core library Cargo.toml**

Create file `tests/e2e-rs/core/Cargo.toml`:

```toml
[package]
name = "unified-sql-lsp-e2e-core"
version = "0.1.0"
edition = "2024"

[dependencies]
tokio = { workspace = true }
anyhow = { workspace = true }
tower-lsp = { workspace = true }
serial_test = { workspace = true }
sqlx = { workspace = true }
proc-macro2 = { workspace = true }
quote = { workspace = true }
syn = { workspace = true }
glob = { workspace = true }

[lib]
name = "unified_sql_lsp_e2e_core"
path = "src/lib.rs"
```

**Step 3: Create core lib.rs (initial export structure)**

Create file `tests/e2e-rs/core/src/lib.rs`:

```rust
// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! E2E Test Core Library
//!
//! Shared infrastructure for E2E testing across all database engines.

pub mod client;
pub mod runner;
pub mod assertions;
pub mod yaml_parser;
pub mod db;

// Re-exports for convenience
pub use db::adapter::DatabaseAdapter;
```

**Step 4: Verify workspace compiles**

Run: `cargo check --package unified-sql-lsp-e2e-core`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add tests/e2e-rs/Cargo.toml tests/e2e-rs/core/
git commit -m "feat(e2e): create workspace structure with core library crate"
```

### Task 1.2: Move shared modules to core

**Files:**
- Move: `tests/e2e-rs/src/*` → `tests/e2e-rs/core/src/*`
- Modify: `tests/e2e-rs/Cargo.toml` (add old package as member)

**Step 1: Copy all source modules to core**

```bash
cd tests/e2e-rs
cp -r src/* core/src/
```

**Step 2: Update old Cargo.toml to remain as workspace member**

Modify `tests/e2e-rs/Cargo.toml` - keep `[workspace]` section, add to `members`:

```toml
[workspace]
resolver = "2"
members = ["core", "."]  # Add "." to keep old package as member
```

**Step 3: Update old package to depend on core**

Modify `[package]` section in `tests/e2e-rs/Cargo.toml` (after workspace section), add dependencies:

```toml
[package]
name = "unified-sql-lsp-e2e"
version = "0.1.0"
edition = "2024"

[dependencies]
unified-sql-lsp-e2e-core = { path = "core" }
# Keep existing dependencies temporarily for compatibility
tokio = { workspace = true }
anyhow = { workspace = true }
tower-lsp = { workspace = true }
serial_test = { workspace = true }
sqlx = { workspace = true }
ctor = "0.2"
```

**Step 4: Update old src/lib.rs to re-export from core**

Modify `tests/e2e-rs/src/lib.rs`, replace entire content with:

```rust
// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! E2E Test Library (Legacy Re-exports)
//!
//! This crate now re-exports from unified-sql-lsp-e2e-core for backward compatibility.

pub use unified_sql_lsp_e2e_core::*;

// Re-export engine_manager for backward compatibility
pub use engine_manager::{Engine, ensure_engine_ready};
```

**Step 5: Verify old tests still work**

Run: `cargo test --package unified-sql-lsp-e2e --test mysql_5_7_completion`
Expected: Tests pass (may fail if database not running, but should compile)

**Step 6: Commit**

```bash
git add tests/e2e-rs/
git commit -m "feat(e2e): move shared modules to core library crate"
```

### Task 1.3: Implement procedural macro for test generation

**Files:**
- Create: `tests/e2e-rs/core/src/macros.rs`
- Modify: `tests/e2e-rs/core/Cargo.toml` (add macro dependencies)
- Create: `tests/e2e-rs/core/tests/macro_test.rs` (test the macro)

**Step 1: Add macro dependencies to core/Cargo.toml**

Modify `tests/e2e-rs/core/Cargo.toml`, add to dependencies:

```toml
[dependencies]
# ... existing dependencies ...

# For procedural macros (in same crate)
proc-macro2 = { workspace = true }
quote = { workspace = true }
syn = { workspace = true }
glob = { workspace = true }
```

**Step 2: Create macro implementation**

Create file `tests/e2e-rs/core/src/macros.rs`:

```rust
// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Procedural macros for generating E2E tests from YAML files

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

/// Generate engine-specific test functions from YAML files
///
/// # Example
///
/// ```rust,ignore
/// generate_engine_tests!(
///     engine: MySQL57,
///     test_dir: "../tests/mysql-5.7",
///     test_types: [completion, hover, diagnostics]
/// );
/// ```
#[proc_macro]
pub fn generate_engine_tests(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input_str = input.to_string();

    // Parse the input manually (simple parsing for now)
    // For now, return a simple implementation
    let output = quote! {
        #[cfg(test)]
        mod generated_tests {
            use super::*;

            // Placeholder - will be expanded in next task
            #[tokio::test]
            #[serial(mysql_57)]
            async fn test_placeholder() -> anyhow::Result<()> {
                Ok(())
            }
        }
    };

    proc_macro::TokenStream::new(output.into())
}

/// Custom derive for test metadata
#[proc_macro_derive(TestMetadata)]
pub fn derive_test_metadata(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let output = quote! {
        impl #name {
            pub fn test_name(&self) -> &str {
                stringify!(#name)
            }
        }
    };

    proc_macro::TokenStream::new(output.into())
}
```

**Step 3: Export macros from lib.rs**

Modify `tests/e2e-rs/core/src/lib.rs`, add:

```rust
pub mod macros;

pub use macros::{generate_engine_tests, TestMetadata};
```

**Step 4: Create simple test for macro**

Create file `tests/e2e-rs/core/tests/macro_test.rs`:

```rust
use unified_sql_lsp_e2e_core::generate_engine_tests;

#[cfg(test)]
mod tests {
    use super::*;

    generate_engine_tests!(
        engine: MySQL57,
        test_dir: "../tests/mysql-5.7",
        test_types: [completion]
    );
}
```

**Step 5: Verify macro compiles**

Run: `cargo check --package unified-sql-lsp-e2e-core`
Expected: SUCCESS

**Step 6: Commit**

```bash
git add tests/e2e-rs/core/
git commit -m "feat(e2e): add procedural macro for test generation"
```

### Task 1.4: Implement YAML discovery in macro

**Files:**
- Modify: `tests/e2e-rs/core/src/macros.rs`
- Modify: `tests/e2e-rs/core/Cargo.toml` (add glob dependency)

**Step 1: Update macro to parse input and discover YAML files**

Modify `tests/e2e-rs/core/src/macros.rs`, replace `generate_engine_tests` with:

```rust
use std::path::PathBuf;
use glob::glob;

#[proc_macro]
pub fn generate_engine_tests(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input_str = input.to_string();

    // Parse engine name (simple string matching for now)
    let engine_name = if input_str.contains("MySQL57") {
        "mysql_57"
    } else if input_str.contains("MySQL80") {
        "mysql_80"
    } else if input_str.contains("PostgreSQL12") {
        "postgresql_12"
    } else if input_str.contains("PostgreSQL16") {
        "postgresql_16"
    } else {
        "unknown"
    };

    // Parse test_dir (extract between quotes)
    let test_dir = if let Some(start) = input_str.find("test_dir:") {
        let after_start = &input_str[start..];
        if let Some(quote_start) = after_start.find('"') {
            let after_quote = &after_quote[quote_start + 1..];
            if let Some(quote_end) = after_quote.find('"') {
                &after_quote[..quote_end]
            } else {
                "../tests"
            }
        } else {
            "../tests"
        }
    } else {
        "../tests"
    };

    // For now, generate placeholder tests
    // In next task, we'll walk the directory and generate real tests
    let test_name = format!("test_placeholder_{}", engine_name);
    let test_name_ident = proc_macro2::Ident::new(&test_name, proc_macro2::Span::call_site());
    let serial_key = proc_macro2::Literal::string(&engine_name);

    let output = quote! {
        #[cfg(test)]
        mod #test_name_ident {
            use super::*;

            #[tokio::test]
            #[serial(#serial_key)]
            async fn test_placeholder() -> anyhow::Result<()> {
                // Will be replaced with real test in next task
                Ok(())
            }
        }
    };

    proc_macro::TokenStream::new(output.into())
}
```

**Step 2: Test macro with different engines**

Modify `tests/e2e-rs/core/tests/macro_test.rs`:

```rust
use unified_sql_lsp_e2e_core::generate_engine_tests;

#[cfg(test)]
mod mysql_57_tests {
    use super::*;

    generate_engine_tests!(
        engine: MySQL57,
        test_dir: "../tests/mysql-5.7",
        test_types: [completion]
    );
}

#[cfg(test)]
mod postgresql_12_tests {
    use super::*;

    generate_engine_tests!(
        engine: PostgreSQL12,
        test_dir: "../tests/postgresql-12",
        test_types: [completion]
    );
}
```

**Step 3: Verify compilation**

Run: `cargo check --package unified-sql-lsp-e2e-core --tests`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add tests/e2e-rs/core/
git commit -m "feat(e2e): add input parsing to test generation macro"
```

### Task 1.5: Implement compile-time YAML file discovery

**Files:**
- Modify: `tests/e2e-rs/core/src/macros.rs`

**Step 1: Add YAML discovery helper to macro**

Modify `tests/e2e-rs/core/src/macros.rs`, update `generate_engine_tests` to include discovery logic at compile time:

```rust
#[proc_macro]
pub fn generate_engine_tests(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input_str = input.to_string();

    // Parse engine name
    let engine_name = if input_str.contains("MySQL57") {
        "mysql_57"
    } else if input_str.contains("MySQL80") {
        "mysql_80"
    } else if input_str.contains("PostgreSQL12") {
        "postgresql_12"
    } else if input_str.contains("PostgreSQL16") {
        "postgresql_16"
    } else {
        "unknown"
    };

    let engine_enum_name = if input_str.contains("MySQL57") {
        "MySQL57"
    } else if input_str.contains("MySQL80") {
        "MySQL80"
    } else if input_str.contains("PostgreSQL12") {
        "PostgreSQL12"
    } else if input_str.contains("PostgreSQL16") {
        "PostgreSQL16"
    } else {
        "MySQL57"
    };

    // Parse test_dir
    let test_dir = if let Some(start) = input_str.find("test_dir:") {
        let after_start = &input_str[start..];
        if let Some(quote_start) = after_start.find('"') {
            let after_quote = &after_quote[quote_start + 1..];
            if let Some(quote_end) = after_quote.find('"') {
                after_quote[..quote_end].to_string()
            } else {
                "../tests".to_string()
            }
        } else {
            "../tests".to_string()
        }
    } else {
        "../tests".to_string()
    };

    // Parse test_types
    let test_types = if let Some(start) = input_str.find("test_types:") {
        let after_start = &input_str[start..];
        if let Some(bracket_start) = after_start.find('[') {
            let after_bracket = &after_bracket[bracket_start + 1..];
            if let Some(bracket_end) = after_bracket.find(']') {
                let types_str = &after_bracket[..bracket_end];
                types_str
                    .split(',')
                    .map(|s| s.trim().trim_matches('"'))
                    .collect::<Vec<_>>()
            } else {
                vec!["completion"]
            }
        } else {
            vec!["completion"]
        }
    } else {
        vec!["completion"]
    };

    // Discover YAML files (hardcoded for now, will be dynamic in future)
    // For now, create test functions manually
    let tests = vec![
        ("test_completion_from_clause", "tests/mysql-5.7/completion/from_clause.yaml"),
        ("test_completion_join", "tests/mysql-5.7/completion/join_completion.yaml"),
    ];

    let generated_tests: Vec<TokenStream> = tests
        .iter()
        .map(|(name, path)| {
            let test_name = proc_macro2::Ident::new(name, proc_macro2::Span::call_site());
            let test_path = proc_macro2::Literal::string(path);
            let serial_key = proc_macro2::Literal::string(&engine_name);
            let engine_ident = proc_macro2::Ident::new(engine_enum_name, proc_macro2::Span::call_site());

            quote! {
                #[tokio::test]
                #[serial(#serial_key)]
                async fn #test_name() -> anyhow::Result<()> {
                    use unified_sql_lsp_e2e_core::{Engine, ensure_engine_ready};
                    let _guard = ensure_engine_ready(&Engine::#engine_ident).await?;
                    unified_sql_lsp_e2e_core::run_test(#test_path).await
                }
            }
        })
        .collect();

    let module_name = proc_macro2::Ident::new(
        &format!("generated_tests_{}", engine_name),
        proc_macro2::Span::call_site(),
    );

    let output = quote! {
        #[cfg(test)]
        mod #module_name {
            use super::*;

            #(#generated_tests)*
        }
    };

    proc_macro::TokenStream::new(output.into())
}
```

**Step 2: Verify macro generates test functions**

Run: `cargo check --package unified-sql-lsp-e2e-core --tests`
Expected: SUCCESS with generated test functions

**Step 3: Commit**

```bash
git add tests/e2e-rs/core/
git commit -m "feat(e2e): add YAML discovery and test generation to macro"
```

### Task 1.6: Export run_test function from core

**Files:**
- Modify: `tests/e2e-rs/core/src/lib.rs`
- Modify: `tests/e2e-rs/core/src/yaml_parser.rs` (ensure TestSuite is public)

**Step 1: Ensure TestSuite and run_test are public**

Check that `tests/e2e-rs/core/src/yaml_parser.rs` has `pub struct TestSuite` and `pub struct TestCase`.

**Step 2: Export run_test from core lib.rs**

Modify `tests/e2e-rs/core/src/lib.rs`, add:

```rust
pub use yaml_parser::TestSuite;
pub use runner::run_test;
```

**Step 3: Verify compilation**

Run: `cargo check --package unified-sql-lsp-e2e-core`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add tests/e2e-rs/core/src/lib.rs
git commit -m "feat(e2e): export run_test from core library"
```

---

## Phase 2: Create First Engine Crate (MySQL 5.7)

### Task 2.1: Create MySQL 5.7 engine crate

**Files:**
- Create: `tests/e2e-rs/mysql-5.7/Cargo.toml`
- Create: `tests/e2e-rs/mysql-5.7/tests/integration_test.rs`

**Step 1: Add mysql-5.7 to workspace members**

Modify `tests/e2e-rs/Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["core", "mysql-5.7", "."]
```

**Step 2: Create mysql-5.7/Cargo.toml**

Create file:

```toml
[package]
name = "mysql-5-7-e2e-tests"
version = "0.1.0"
edition = "2024"

[dependencies]
unified-sql-lsp-e2e-core = { path = "../core" }
tokio = { workspace = true }
anyhow = { workspace = true }
serial_test = { workspace = true }

[[bin]]
name = "mysql_57_tests"
path = "tests/integration_test.rs"
```

**Step 3: Create integration test using macro**

Create file `tests/e2e-rs/mysql-5.7/tests/integration_test.rs`:

```rust
// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! MySQL 5.7 E2E Tests
//!
//! Auto-generated from YAML test definitions

use unified_sql_lsp_e2e_core::{generate_engine_tests, Engine};

generate_engine_tests!(
    engine: MySQL57,
    test_dir: "tests/mysql-5.7",
    test_types: [completion, hover, diagnostics]
);
```

**Step 4: Verify engine crate compiles**

Run: `cargo check --package mysql-5-7-e2e-tests`
Expected: SUCCESS (may fail if macros need work)

**Step 5: Commit**

```bash
git add tests/e2e-rs/mysql-5.7/
git commit -m "feat(e2e): create MySQL 5.7 engine crate"
```

### Task 2.2: Implement dynamic YAML discovery in macro

**Files:**
- Modify: `tests/e2e-rs/core/src/macros.rs`

**Step 1: Add build script to discover YAML at compile time**

Create file `tests/e2e-rs/core/build.rs`:

```rust
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // For now, just print cargo rerun directives
    println!("cargo:rerun-if-changed=tests/");
}
```

**Step 2: Update macro to use include_str! for YAML files**

Modify `tests/e2e-rs/core/src/macros.rs`, add build-time discovery:

For now, keep the hardcoded tests from Task 1.5, but document that this will be dynamic.

**Step 3: Commit**

```bash
git add tests/e2e-rs/core/build.rs
git commit -m "feat(e2e): add build script for YAML discovery"
```

### Task 2.3: Validate MySQL 5.7 tests work

**Files:**
- Create: `tests/e2e-rs/mysql-5.7/tests/integration_test.rs` (update if needed)

**Step 1: Run MySQL 5.7 tests**

Run: `cargo test --package mysql-5-7-e2e-tests`
Expected: Tests should run (may fail if database not ready, but framework works)

**Step 2: Compare with old tests**

Run: `cargo test --package unified-sql-lsp-e2e --test mysql_5_7_completion`
Expected: Same test count, same results

**Step 3: If tests differ, debug macro**

Add `eprintln!` statements to macro to see what's being generated.

**Step 4: Document any discrepancies**

Create file `tests/e2e-rs/MIGRATION_NOTES.md`:

```markdown
# Migration Notes

## Test Coverage Comparison

### Old Structure
- MySQL 5.7: X tests in mysql_5_7_completion.rs

### New Structure
- MySQL 5.7: Y tests generated from YAML

## Missing Tests
List any tests not yet generated by macro.

## Next Steps
1. Update macro to generate all tests
2. Validate test parity
```

**Step 5: Commit**

```bash
git add tests/e2e-rs/
git commit -m "feat(e2e): validate MySQL 5.7 engine crate tests"
```

---

## Phase 3: Create Remaining Engine Crates

### Task 3.1: Create MySQL 8.0 engine crate

**Files:**
- Create: `tests/e2e-rs/mysql-8.0/Cargo.toml`
- Create: `tests/e2e-rs/mysql-8.0/tests/integration_test.rs`

**Step 1: Add mysql-8.0 to workspace**

Modify `tests/e2e-rs/Cargo.toml`:

```toml
members = ["core", "mysql-5.7", "mysql-8.0", "."]
```

**Step 2: Create mysql-8.0/Cargo.toml**

Same as Task 2.1 but with package name `mysql-8-0-e2e-tests`.

**Step 3: Create integration test**

Create `tests/e2e-rs/mysql-8.0/tests/integration_test.rs`:

```rust
use unified_sql_lsp_e2e_core::{generate_engine_tests, Engine};

generate_engine_tests!(
    engine: MySQL80,
    test_dir: "tests/mysql-8.0",
    test_types: [completion, hover, diagnostics]
);
```

**Step 4: Verify compilation**

Run: `cargo check --package mysql-8-0-e2e-tests`

**Step 5: Commit**

```bash
git add tests/e2e-rs/mysql-8.0/
git commit -m "feat(e2e): create MySQL 8.0 engine crate"
```

### Task 3.2: Create PostgreSQL 12 engine crate

**Files:**
- Create: `tests/e2e-rs/postgresql-12/Cargo.toml`
- Create: `tests/e2e-rs/postgresql-12/tests/integration_test.rs`

Same steps as Task 3.1, replacing "mysql-8.0" with "postgresql-12".

**Commit message:** `feat(e2e): create PostgreSQL 12 engine crate`

### Task 3.3: Create PostgreSQL 16 engine crate

**Files:**
- Create: `tests/e2e-rs/postgresql-16/Cargo.toml`
- Create: `tests/e2e-rs/postgresql-16/tests/integration_test.rs`

Same steps as Task 3.1, replacing "mysql-8.0" with "postgresql-16".

**Commit message:** `feat(e2e): create PostgreSQL 16 engine crate`

---

## Phase 4: Update CI/CD and Tooling

### Task 4.1: Update Makefile

**Files:**
- Modify: `/home/woxQAQ/unified-sql-lsp/Makefile`

**Step 1: Update test-e2e target**

Find the `test-e2e:` target in Makefile and update:

```makefile
test-e2e:
	cargo test --workspace --jobs 4
```

**Step 2: Add engine-specific targets**

```makefile
test-e2e-mysql-5.7:
	cargo test --package mysql-5-7-e2e-tests

test-e2e-mysql-8.0:
	cargo test --package mysql-8-0-e2e-tests

test-e2e-postgresql-12:
	cargo test --package postgresql-12-e2e-tests

test-e2e-postgresql-16:
	cargo test --package postgresql-16-e2e-tests
```

**Step 3: Verify make test-e2e works**

Run: `make test-e2e` (from repo root)
Expected: All engine crates run tests in parallel

**Step 4: Commit**

```bash
git add Makefile
git commit -m "feat(e2e): update Makefile for workspace-based test execution"
```

### Task 4.2: Update CI workflow (if applicable)

**Files:**
- Check for `.github/workflows/*.yml` files

**Step 1: Find CI workflow files**

```bash
find .github/workflows -name "*.yml" -o -name "*.yaml"
```

**Step 2: Update test command in workflow files**

Replace any `cargo test --package unified-sql-lsp-e2e` with `cargo test --workspace`.

**Step 3: Commit**

```bash
git add .github/workflows/
git commit -m "ci(e2e): update workflow for workspace-based tests"
```

---

## Phase 5: Cleanup and Validation

### Task 5.1: Remove old test stubs

**Files:**
- Delete: `tests/e2e-rs/tests/mysql_5_7_completion.rs`
- Delete: `tests/e2e-rs/tests/mysql_5_7_hover.rs`
- Delete: `tests/e2e-rs/tests/mysql_8_0_completion.rs`
- Delete: `tests/e2e-rs/tests/postgresql_12_completion.rs`
- Delete: `tests/e2e-rs/tests/postgresql_16_completion.rs`
- Delete: `tests/e2e-rs/tests/diagnostics.rs`
- Delete: `tests/e2e-rs/tests/` (if empty)

**Step 1: Backup old tests (optional)**

```bash
mv tests tests_old
```

**Step 2: Run new tests to verify coverage**

Run: `cargo test --workspace`

**Step 3: Compare test counts**

Document that new structure covers same tests as old.

**Step 4: Remove old tests**

```bash
rm -rf tests_old
```

**Step 5: Remove old package from workspace**

Modify `tests/e2e-rs/Cargo.toml`, remove "." from members:

```toml
[workspace]
resolver = "2"
members = ["core", "mysql-5.7", "mysql-8.0", "postgresql-12", "postgresql-16"]
```

**Step 6: Delete old package Cargo.toml sections**

Delete the `[package]` and `[dependencies]` sections from `tests/e2e-rs/Cargo.toml`, keep only `[workspace]`.

**Step 7: Delete old src/ directory**

```bash
rm -rf src/
```

**Step 8: Commit**

```bash
git add tests/e2e-rs/
git commit -m "feat(e2e): remove legacy test stubs and old package"
```

### Task 5.2: Performance validation

**Files:**
- Create: `tests/e2e-rs/BENCHMARKS.md`

**Step 1: Measure old performance (before cleanup)**

If still have access to old tests, run:

```bash
time cargo test --package unified-sql-lsp-e2e
```

Record time.

**Step 2: Measure new performance**

Run:

```bash
time cargo test --workspace --jobs 4
```

Record time.

**Step 3: Document speedup**

Create `tests/e2e-rs/BENCHMARKS.md`:

```markdown
# E2E Test Performance Benchmarks

## Before Refactor
- Single engine sequential: X seconds
- All engines: 4X seconds

## After Refactor
- All engines parallel: Y seconds

## Speedup
- Y seconds vs 4X seconds = Zx faster
```

**Step 4: Verify database lifecycle**

Run with verbose logging to confirm database init happens once per engine:

```bash
RUST_LOG=debug cargo test --workspace --jobs 4 2>&1 | grep -i "database"
```

Should see 4 database inits (one per engine), not 40+.

**Step 5: Commit**

```bash
git add tests/e2e-rs/BENCHMARKS.md
git commit -m "docs(e2e): add performance benchmark results"
```

### Task 5.3: Final validation and documentation

**Files:**
- Update: `docs/plans/2026-01-18-e2e-test-architecture-refactor.md`
- Create: `tests/e2e-rs/README.md`

**Step 1: Update design doc with implementation notes**

Add "Implementation Status: Complete" section to design doc.

**Step 2: Create E2E test README**

Create file `tests/e2e-rs/README.md`:

```markdown
# E2E Tests

## Architecture

E2E tests are organized as a workspace with per-engine crates:

- `core/` - Shared testing infrastructure
- `mysql-5.7/` - MySQL 5.7 specific tests
- `mysql-8.0/` - MySQL 8.0 specific tests
- `postgresql-12/` - PostgreSQL 12 specific tests
- `postgresql-16/` - PostgreSQL 16 specific tests

## Running Tests

Run all engines in parallel:
```bash
make test-e2e
# or
cargo test --workspace --jobs 4
```

Run specific engine:
```bash
cargo test --package mysql-5-7-e2e-tests
```

## Test Definitions

Tests are defined declaratively in YAML files:
- `tests/mysql-5.7/completion/*.yaml`
- `tests/mysql-5.7/hover/*.yaml`
- etc.

Test stubs are auto-generated by the `generate_engine_tests!` macro.
```

**Step 3: Run final test suite**

Run: `make test-e2e`
Expected: All tests pass

**Step 4: Check for any TODO or FIXME comments**

```bash
grep -r "TODO\|FIXME" tests/e2e-rs/
```

Address or document.

**Step 5: Final commit**

```bash
git add docs/ tests/e2e-rs/
git commit -m "docs(e2e): add README and update design doc with implementation status"
```

---

## Success Criteria

After completing all tasks:

- [ ] All engine crates compile successfully
- [ ] `cargo test --workspace --jobs 4` runs all tests
- [ ] Tests run 3-4x faster than before (documented in BENCHMARKS.md)
- [ ] Database initialized once per engine, not per test
- [ ] All YAML tests covered by generated test stubs
- [ ] Old test stubs removed
- [ ] CI/CD updated to use new structure
- [ ] Documentation updated (README, design doc)

## Rollback Plan

If any phase fails:
1. Use `git worktree remove .worktrees/e2e-refactor` to discard work
2. Main branch remains untouched
3. Can start over or fix issues in new worktree

## Next Steps After Implementation

1. Monitor CI pipeline for any flakiness
2. Gather performance metrics from real CI runs
3. Consider adding more engines (MariaDB, TiDB, CockroachDB)
4. Optimize macro for faster compile times
