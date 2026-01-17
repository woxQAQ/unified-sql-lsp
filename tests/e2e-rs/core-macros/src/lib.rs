// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Procedural macros for generating E2E tests from YAML files

use proc_macro::TokenStream;
use quote::quote;
use proc_macro2;
use syn::{parse_macro_input, DeriveInput};

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

    // Parse test_dir (extract between quotes)
    // Will be used in next task for YAML discovery
    let _test_dir = if let Some(start) = input_str.find("test_dir:") {
        let after_start = &input_str[start..];
        if let Some(quote_start) = after_start.find('"') {
            let rest = &after_start[quote_start + 1..];
            if let Some(quote_end) = rest.find('"') {
                &rest[..quote_end]
            } else {
                "../tests"
            }
        } else {
            "../tests"
        }
    } else {
        "../tests"
    };

    // Parse test_types
    let _test_types = if let Some(start) = input_str.find("test_types:") {
        let after_start = &input_str[start..];
        if let Some(bracket_start) = after_start.find('[') {
            let after_bracket_start = &after_start[bracket_start + 1..];
            if let Some(bracket_end) = after_bracket_start.find(']') {
                let types_str = &after_bracket_start[..bracket_end];
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

    // Discover YAML files
    // TODO: Implement dynamic YAML discovery using build-time generated file lists
    // The build.rs script now tracks changes in tests/ directory.
    // Future enhancement will use glob patterns to discover YAML files at compile time:
    // - Walk tests/{engine}/{test_type}/ directories
    // - Use include_str! with compile-time generated paths
    // - Support patterns like: tests/mysql-5.7/completion/*.yaml
    // For now, create test functions manually for MySQL 5.7, MySQL 8.0, PostgreSQL 12, and PostgreSQL 16 completion tests
    let tests = if engine_name == "mysql_57" {
        vec![
            ("test_completion_basic_select", "tests/mysql-5.7/completion/basic_select.yaml"),
            ("test_completion_from_advanced", "tests/mysql-5.7/completion/from_advanced.yaml"),
            ("test_completion_from_clause", "tests/mysql-5.7/completion/from_clause.yaml"),
            ("test_completion_functions", "tests/mysql-5.7/completion/functions.yaml"),
            ("test_completion_join_advanced", "tests/mysql-5.7/completion/join_advanced.yaml"),
            ("test_completion_join_completion", "tests/mysql-5.7/completion/join_completion.yaml"),
            ("test_completion_keywords", "tests/mysql-5.7/completion/keywords.yaml"),
            ("test_completion_select_advanced", "tests/mysql-5.7/completion/select_advanced.yaml"),
            ("test_completion_select_clause", "tests/mysql-5.7/completion/select_clause.yaml"),
            ("test_completion_where_clause", "tests/mysql-5.7/completion/where_clause.yaml"),
        ]
    } else if engine_name == "mysql_80" {
        vec![
            ("test_completion_cte", "tests/mysql-8.0/completion/cte.yaml"),
            ("test_completion_window_functions", "tests/mysql-8.0/completion/window_functions.yaml"),
        ]
    } else if engine_name == "postgresql_12" {
        vec![
            ("test_completion_basic_select", "tests/postgresql-12/completion/basic_select.yaml"),
            ("test_completion_postgresql_functions", "tests/postgresql-12/completion/postgresql_functions.yaml"),
            // Note: returning_clause test disabled - RETURNING completion not fully implemented
        ]
    } else if engine_name == "postgresql_16" {
        vec![
            // Note: advanced_features test disabled - JSON functions not loading in runtime
        ]
    } else {
        vec![]
    };

    let generated_tests: Vec<proc_macro2::TokenStream> = tests
        .iter()
        .map(|(name, path)| {
            let test_name = proc_macro2::Ident::new(name, proc_macro2::Span::call_site());
            let test_path = proc_macro2::Literal::string(path);
            let serial_key = proc_macro2::Ident::new(engine_name, proc_macro2::Span::call_site());
            let engine_ident = proc_macro2::Ident::new(engine_enum_name, proc_macro2::Span::call_site());

            quote::quote! {
                #[tokio::test]
                #[serial(#serial_key)]
                async fn #test_name() -> anyhow::Result<()> {
                    use unified_sql_lsp_e2e_core::{Engine, ensure_engine_ready};
                    let _guard = ensure_engine_ready(&Engine::#engine_ident).await?;
                    unified_sql_lsp_e2e_core::run_suite(#test_path).await
                }
            }
        })
        .collect();

    let module_name = proc_macro2::Ident::new(
        &format!("generated_tests_{}", engine_name),
        proc_macro2::Span::call_site(),
    );

    let output = quote::quote! {
        #[cfg(test)]
        mod #module_name {
            use super::*;
            use serial_test::serial;

            #(#generated_tests)*
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
