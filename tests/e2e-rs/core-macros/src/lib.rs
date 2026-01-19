// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Procedural macros for generating E2E tests from YAML files
//!
//! This macro uses dynamic YAML discovery via build.rs and glob patterns,
//! automatically discovering all test files without manual configuration.

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

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
pub fn generate_engine_tests(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();

    // Parse engine name (keep existing logic)
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

    // Get test types for this engine
    // These match what build.rs discovers in the tests/ directory
    let test_types = get_test_types_for_engine(engine_name);

    let test_func_name = proc_macro2::Ident::new(
        &format!("test_{}", engine_name),
        proc_macro2::Span::call_site(),
    );
    let engine_ident = proc_macro2::Ident::new(engine_enum_name, proc_macro2::Span::call_site());

    // Generate glob patterns for each test type
    let glob_pattern_literals: Vec<proc_macro2::Literal> = test_types
        .iter()
        .map(|test_type| {
            let pattern = format!(
                "tests/{}/{}/*.yaml",
                format_engine_path(engine_name),
                test_type
            );
            proc_macro2::Literal::string(&pattern)
        })
        .collect();

    let serial_key = proc_macro2::Ident::new(engine_name, proc_macro2::Span::call_site());

    // Build array syntax manually
    let patterns_array = if glob_pattern_literals.len() == 1 {
        let pat = &glob_pattern_literals[0];
        quote::quote! { &[#pat] }
    } else {
        quote::quote! { &[#(#glob_pattern_literals),*] }
    };

    let output = quote::quote! {
        #[cfg(test)]
        mod #test_func_name {
            use super::*;
            use serial_test::serial;

            #[tokio::test]
            #[serial(#serial_key)]
            async fn #test_func_name() -> anyhow::Result<()> {
                use unified_sql_lsp_e2e_core::{Engine, ensure_engine_ready};
                let _guard = ensure_engine_ready(&Engine::#engine_ident).await?;

                let patterns: &[&str] = #patterns_array;
                let mut test_count = 0;

                for pattern in patterns {
                    for entry in glob::glob(pattern).map_err(|e| anyhow::anyhow!("Invalid glob pattern: {}", e))? {
                        let yaml_path = entry.map_err(|e| anyhow::anyhow!("Failed to read path: {}", e))?;
                        println!("Running test: {}", yaml_path.display());

                        unified_sql_lsp_e2e_core::run_suite(yaml_path.to_str().unwrap())
                            .await
                            .map_err(|e| anyhow::anyhow!("Test failed for {}: {}", yaml_path.display(), e))?;

                        test_count += 1;
                    }
                }

                println!("✓ Ran {} tests for {}", test_count, #engine_name);
                Ok(())
            }
        }
    };

    TokenStream::from(output)
}

/// Custom derive for test metadata
#[proc_macro_derive(TestMetadata)]
pub fn derive_test_metadata(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let output = quote! {
        impl #name {
            pub fn test_name(&self) -> &str {
                stringify!(#name)
            }
        }
    };

    TokenStream::from(output)
}

/// Get test types for a specific engine
/// These match the directory structure discovered by build.rs
fn get_test_types_for_engine(engine_name: &str) -> Vec<String> {
    match engine_name {
        "mysql_57" => vec![
            "completion".to_string(),
            "diagnostics".to_string(),
            "hover".to_string(),
        ],
        "mysql_80" => vec!["completion".to_string()],
        "postgresql_12" => vec!["completion".to_string()],
        "postgresql_16" => vec!["completion".to_string()],
        _ => Vec::new(),
    }
}

/// Format engine name for use in paths (mysql_57 -> mysql-5.7)
fn format_engine_path(engine_name: &str) -> String {
    engine_name.replace("_", "-")
}
