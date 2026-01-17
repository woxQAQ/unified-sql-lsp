// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Test assertion helpers
//!
//! Provides assertions for completion, diagnostics, and hover.

use anyhow::{Result, bail};
use std::collections::HashSet;
use tower_lsp::lsp_types::*;

/// Assert completion contains specific items
pub fn assert_completion_contains(items: &[CompletionItem], expected: &[String]) -> Result<()> {
    let item_labels: HashSet<&str> = items.iter().map(|i| i.label.as_str()).collect();

    for expected_item in expected {
        if !item_labels.contains(expected_item.as_str()) {
            bail!(
                "Expected completion to contain '{}', but it was not found. Available items: {:?}",
                expected_item,
                item_labels
            );
        }
    }

    Ok(())
}

/// Assert completion does NOT contain specific items
pub fn assert_completion_not_contains(
    items: &[CompletionItem],
    unexpected: &[String],
) -> Result<()> {
    let item_labels: HashSet<&str> = items.iter().map(|i| i.label.as_str()).collect();

    for unexpected_item in unexpected {
        if item_labels.contains(unexpected_item.as_str()) {
            bail!(
                "Expected completion to NOT contain '{}', but it was found",
                unexpected_item
            );
        }
    }

    Ok(())
}

/// Assert exact completion count
pub fn assert_completion_count(items: &[CompletionItem], expected: usize) -> Result<()> {
    if items.len() != expected {
        bail!(
            "Expected {} completion items, but got {}. Items: {:?}",
            expected,
            items.len(),
            items.iter().map(|i| &i.label).collect::<Vec<_>>()
        );
    }
    Ok(())
}

/// Assert minimum completion count
pub fn assert_completion_min_count(items: &[CompletionItem], min: usize) -> Result<()> {
    if items.len() < min {
        bail!(
            "Expected at least {} completion items, but got {}",
            min,
            items.len()
        );
    }
    Ok(())
}

/// Assert completion items are in specific order (first N items)
pub fn assert_completion_order(items: &[CompletionItem], expected_order: &[String]) -> Result<()> {
    for (i, expected_label) in expected_order.iter().enumerate() {
        if i >= items.len() {
            bail!(
                "Expected item at position {} to be '{}', but only {} items available",
                i,
                expected_label,
                items.len()
            );
        }

        let actual_label = &items[i].label;
        if actual_label != expected_label {
            bail!(
                "Expected item at position {} to be '{}', but got '{}'",
                i,
                expected_label,
                actual_label
            );
        }
    }

    Ok(())
}

/// Assert diagnostics count and severity
pub fn assert_diagnostics(
    diagnostics: &[Diagnostic],
    error_count: usize,
    warning_count: usize,
) -> Result<()> {
    let actual_errors = diagnostics
        .iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .count();

    let actual_warnings = diagnostics
        .iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::WARNING))
        .count();

    if actual_errors != error_count {
        bail!(
            "Expected {} errors, but got {}. Diagnostics: {:?}",
            error_count,
            actual_errors,
            diagnostics
        );
    }

    if actual_warnings != warning_count {
        bail!(
            "Expected {} warnings, but got {}. Diagnostics: {:?}",
            warning_count,
            actual_warnings,
            diagnostics
        );
    }

    Ok(())
}

/// Assert diagnostics count, severity, and error codes
pub fn assert_diagnostics_with_codes(
    diagnostics: &[Diagnostic],
    error_count: usize,
    warning_count: usize,
    error_codes: Option<&[String]>,
) -> Result<()> {
    // First check counts
    assert_diagnostics(diagnostics, error_count, warning_count)?;

    // Then check error codes if provided
    if let Some(expected_codes) = error_codes {
        let actual_codes: Vec<String> = diagnostics
            .iter()
            .filter_map(|d| {
                if d.severity == Some(DiagnosticSeverity::ERROR) {
                    d.code.as_ref().map(|c| match c {
                        tower_lsp::lsp_types::NumberOrString::String(s) => s.clone(),
                        tower_lsp::lsp_types::NumberOrString::Number(n) => n.to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        for expected_code in expected_codes {
            if !actual_codes.contains(expected_code) {
                bail!(
                    "Expected error code '{}', but got codes: {:?}. Diagnostics: {:?}",
                    expected_code,
                    actual_codes,
                    diagnostics
                );
            }
        }
    }

    Ok(())
}

/// Assert hover contains specific text
pub fn assert_hover_contains(hover: Option<&Hover>, expected: &str) -> Result<()> {
    match hover {
        Some(h) => {
            let content = match &h.contents {
                HoverContents::Markup(markup) => markup.value.as_str(),
                HoverContents::Array(contents) => {
                    // Handle array of MarkupContent or String
                    contents
                        .first()
                        .and_then(|c| match c {
                            MarkedString::String(s) => Some(s.as_str()),
                            MarkedString::LanguageString(ls) => Some(ls.value.as_str()),
                        })
                        .unwrap_or("")
                }
                _ => "",
            };

            if !content.contains(expected) {
                bail!(
                    "Expected hover to contain '{}', but got: {}",
                    expected,
                    content
                );
            }
        }
        None => {
            bail!("Expected hover result, but got None");
        }
    }

    Ok(())
}

// ============================================================================
// ENHANCED ASSERTIONS (Phase 5)
// ============================================================================

/// Assert completion item has exact label, kind, and detail
pub fn assert_completion_item_exact(
    items: &[CompletionItem],
    label: &str,
    kind: CompletionItemKind,
    detail: &str,
) -> Result<()> {
    let found = items.iter().find(|item| {
        item.label == label && item.kind == Some(kind) && item.detail.as_deref() == Some(detail)
    });

    if found.is_none() {
        bail!(
            "Expected completion item: {} [{:?}] {}, but not found. Available items: {:?}",
            label,
            kind,
            detail,
            items
                .iter()
                .map(|i| (&i.label, i.kind, &i.detail))
                .collect::<Vec<_>>()
        );
    }

    Ok(())
}

/// Assert text edit range and insert text
/// Note: This is a simplified version that checks if text_edit exists
/// Full implementation requires understanding CompletionTextEdit structure
pub fn assert_completion_has_text_edit(item: &CompletionItem) -> Result<()> {
    if item.text_edit.is_none() {
        bail!("Expected text_edit to be present, but it was None");
    }
    Ok(())
}

/// Assert completion sort order
pub fn assert_completion_sort_order(
    items: &[CompletionItem],
    expected_order: &[&str],
) -> Result<()> {
    let actual_labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();

    for (i, expected) in expected_order.iter().enumerate() {
        if i >= actual_labels.len() {
            bail!(
                "Expected item '{}' at position {}, but only {} items",
                expected,
                i,
                actual_labels.len()
            );
        }
        if actual_labels[i] != *expected {
            bail!(
                "Expected '{}' at position {}, got '{}'",
                expected,
                i,
                actual_labels[i]
            );
        }
    }

    Ok(())
}

/// Assert completion item has specific kind
pub fn assert_completion_kind(
    items: &[CompletionItem],
    label: &str,
    expected_kind: CompletionItemKind,
) -> Result<()> {
    let item = items.iter().find(|i| i.label == label).ok_or_else(|| {
        anyhow::anyhow!(
            "Expected to find item '{}' in completion, but it was not found. Available: {:?}",
            label,
            items.iter().map(|i| &i.label).collect::<Vec<_>>()
        )
    })?;

    if item.kind != Some(expected_kind) {
        bail!(
            "Expected item '{}' to have kind {:?}, but got {:?}",
            label,
            expected_kind,
            item.kind
        );
    }

    Ok(())
}
