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
use serde::Deserialize;
use std::path::{Path, PathBuf};
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

    // Parse engine name and directory name
    let (engine_name, engine_dir_name) = if input_str.contains("MySQL57") {
        ("mysql_57", "mysql-5.7")
    } else if input_str.contains("MySQL80") {
        ("mysql_80", "mysql-8.0")
    } else if input_str.contains("PostgreSQL12") {
        ("postgresql_12", "postgresql-12")
    } else if input_str.contains("PostgreSQL16") {
        ("postgresql_16", "postgresql-16")
    } else {
        ("unknown", "unknown")
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

    let module_name = proc_macro2::Ident::new(
        &format!("test_{}", engine_name),
        proc_macro2::Span::call_site(),
    );
    let engine_ident = proc_macro2::Ident::new(engine_enum_name, proc_macro2::Span::call_site());
    let serial_key = proc_macro2::Ident::new(engine_name, proc_macro2::Span::call_site());
    let requested_test_types =
        parse_test_types_from_input(&input_str).unwrap_or_else(|| get_test_types_for_engine(engine_name));
    let test_cases = discover_test_cases(engine_dir_name, engine_name, &requested_test_types);

    let generated_case_tests: Vec<proc_macro2::TokenStream> = test_cases
        .iter()
        .map(|test_case| {
            let function_ident =
                proc_macro2::Ident::new(&test_case.function_name, proc_macro2::Span::call_site());
            let suite_path_literal = proc_macro2::Literal::string(&test_case.suite_path);
            let case_index_literal = proc_macro2::Literal::usize_unsuffixed(test_case.case_index);
            let label_literal = proc_macro2::Literal::string(&test_case.label);

            quote! {
                #[tokio::test]
                #[serial(#serial_key)]
                async fn #function_ident() -> anyhow::Result<()> {
                    // Disable anyhow backtrace for cleaner error output
                    unsafe { std::env::set_var("RUST_BACKTRACE", "0"); }

                    use unified_sql_lsp_e2e_core::{EngineManagerEngine, ensure_engine_ready};
                    let _guard = ensure_engine_ready(&EngineManagerEngine::#engine_ident).await?;

                    unified_sql_lsp_e2e_core::run_case(#suite_path_literal, #case_index_literal)
                        .await
                        .map_err(|e| anyhow::anyhow!("{} failed: {}", #label_literal, e))
                }
            }
        })
        .collect();

    let output = quote! {
        #[cfg(test)]
        mod #module_name {
            use super::*;
            use serial_test::serial;
            #(#generated_case_tests)*
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

#[derive(Debug, Deserialize)]
struct MacroSuite {
    tests: Vec<MacroCase>,
}

#[derive(Debug, Deserialize)]
struct MacroCase {
    name: String,
}

#[derive(Debug)]
struct DiscoveredCase {
    function_name: String,
    suite_path: String,
    case_index: usize,
    label: String,
}

fn discover_test_cases(
    engine_dir_name: &str,
    engine_name: &str,
    test_types: &[String],
) -> Vec<DiscoveredCase> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|e| panic!("CARGO_MANIFEST_DIR is not set: {e}"));
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .unwrap_or_else(|| panic!("Failed to find e2e-rs workspace root from {manifest_dir}"));

    let mut yaml_paths: Vec<PathBuf> = Vec::new();

    for test_type in test_types {
        let pattern = workspace_root
            .join(format!("tests/{}/{test_type}/*.yaml", engine_dir_name))
            .to_string_lossy()
            .into_owned();

        let entries = glob::glob(&pattern)
            .unwrap_or_else(|e| panic!("Invalid glob pattern `{pattern}`: {e}"));

        for entry in entries {
            let path = entry
                .unwrap_or_else(|e| panic!("Failed to read path for pattern `{pattern}`: {e}"));
            yaml_paths.push(path);
        }
    }

    yaml_paths.sort();

    let mut discovered_cases = Vec::new();

    for yaml_path in &yaml_paths {
        let category = yaml_path
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown-category".to_string());

        let file_stem = yaml_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown-file".to_string());
        let file_stem_ident = to_identifier(&file_stem);

        let yaml_content = std::fs::read_to_string(yaml_path)
            .unwrap_or_else(|e| panic!("Failed to read YAML file {}: {e}", yaml_path.display()));
        let suite: MacroSuite = serde_yaml::from_str(&yaml_content)
            .unwrap_or_else(|e| panic!("Failed to parse YAML file {}: {e}", yaml_path.display()));

        let suite_path = yaml_path.to_string_lossy().into_owned();

        for (case_index, case) in suite.tests.iter().enumerate() {
            let meta_case = normalize_meta_case_name(&case.name);
            let function_name = format!(
                "test_{}_{}_{}_{}_{}",
                engine_name,
                to_identifier(&category),
                file_stem_ident,
                case_index,
                to_identifier(&meta_case)
            );

            discovered_cases.push(DiscoveredCase {
                function_name,
                suite_path: suite_path.clone(),
                case_index,
                label: format!("{engine_dir_name}:{category}:{meta_case}"),
            });
        }
    }

    discovered_cases
}

fn parse_test_types_from_input(input: &str) -> Option<Vec<String>> {
    let test_types_idx = input.find("test_types")?;
    let after_test_types = &input[test_types_idx..];
    let bracket_start = after_test_types.find('[')?;
    let bracket_end = after_test_types.find(']')?;
    if bracket_end <= bracket_start {
        return None;
    }

    let content = &after_test_types[bracket_start + 1..bracket_end];
    let parsed: Vec<String> = content
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_matches('"').to_string())
        .collect();

    if parsed.is_empty() {
        None
    } else {
        Some(parsed)
    }
}

fn normalize_meta_case_name(name: &str) -> String {
    let mut output = String::with_capacity(name.len());
    let mut previous_was_dash = false;

    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch);
            previous_was_dash = false;
        } else if !output.is_empty() && !previous_was_dash {
            output.push('-');
            previous_was_dash = true;
        }
    }

    while output.ends_with('-') {
        output.pop();
    }

    if output.is_empty() {
        "unnamed-case".to_string()
    } else {
        output
    }
}

fn to_identifier(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut previous_was_underscore = false;

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            previous_was_underscore = false;
        } else if !previous_was_underscore {
            out.push('_');
            previous_was_underscore = true;
        }
    }

    out = out.trim_matches('_').to_string();

    if out.is_empty() {
        "unnamed".to_string()
    } else if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("case_{out}")
    } else {
        out
    }
}
