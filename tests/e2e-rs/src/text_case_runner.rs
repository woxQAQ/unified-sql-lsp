// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Text-based test case runner
//!
//! This module provides a runner for the text-based test case format defined in
//! tests/e2e/fixtures/cases/*.txt. It bridges the text-based format with the
//! existing e2e-rs test framework.

use crate::client::LspConnection;
use crate::db::DatabaseAdapter;
use crate::runner::LspRunner;
use anyhow::Result;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::OnceCell;
use tracing::{debug, info};

use unified_sql_lsp_test_utils::{
    parse_test_file, validate_completion, TestCase, Dialect,
};

/// Global test database adapter (initialized once)
static DB_ADAPTER: LazyLock<OnceCell<Arc<dyn DatabaseAdapter>>> = LazyLock::new(OnceCell::new);

/// Initialize test database
pub async fn init_database() -> Result<Arc<dyn DatabaseAdapter>> {
    let adapter = Arc::new(crate::db::MySQLAdapter::from_default_config()) as Arc<dyn DatabaseAdapter>;

    info!("Initializing test database adapter for text-based tests...");

    DB_ADAPTER.set(adapter.clone())
        .map_err(|_| anyhow::anyhow!("Database already initialized"))?;

    info!("Database adapter initialized successfully");
    Ok(adapter)
}

/// Run a single text-based test case
pub async fn run_test(test: &TestCase) -> Result<()> {
    info!("Running test: {}", test.description);

    // Check if test should run for current dialect
    let should_run = match test.dialect {
        Dialect::All => true,
        Dialect::MySQL => {
            // TODO: Check current database dialect
            true
        }
        Dialect::PostgreSQL => {
            // TODO: Check current database dialect
            false
        }
    };

    if !should_run {
        info!("Skipping test (dialect mismatch): {}", test.description);
        return Ok(());
    }

    // 1. Get database adapter
    let adapter = DB_ADAPTER.get()
        .ok_or_else(|| anyhow::anyhow!("Database not initialized. Call init_database() first."))?;

    // 2. Load schema/data files if specified in context
    if let Some(context) = &test.context {
        debug!("Test context: {}", context);
        // TODO: Parse context and load appropriate schema/data
    }

    // 3. Spawn LSP server
    let mut lsp_runner = LspRunner::from_crate()?;
    lsp_runner.spawn().await?;

    // 4. Establish LSP connection
    let stdin = lsp_runner.stdin()?;
    let stdout = lsp_runner.stdout()?;
    let mut conn = LspConnection::new(stdin, stdout);

    // 5. Initialize server
    let _init_result = conn.initialize().await?;

    // 6. Get cursor position and strip marker
    let sql_without_cursor = test.input.replace('|', "");
    let cursor_offset = test.input.find('|')
        .ok_or_else(|| anyhow::anyhow!("No cursor marker '|' found in test input"))?;

    // Convert cursor offset to LSP Position
    let position = offset_to_position(&sql_without_cursor, cursor_offset)?;

    // 7. Open document
    let uri = tower_lsp::lsp_types::Url::parse(&format!(
        "file:///test_{}.sql",
        test.description.replace(' ', "_")
    ))?;

    // Determine dialect from test case
    let dialect = match test.dialect {
        Dialect::MySQL => "mysql".to_string(),
        Dialect::PostgreSQL => "postgresql".to_string(),
        Dialect::All => "mysql".to_string(), // Default to MySQL for "all"
    };

    conn.did_open(uri.clone(), dialect, sql_without_cursor.clone()).await?;

    // Give server time to parse
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 8. Request completion
    let completion_items = conn.completion(uri.clone(), position).await?
        .unwrap_or_default();

    debug!("Received {} completion items", completion_items.len());

    // 9. Validate results (convert tower_lsp types to lsp_types)
    let lsp_items: Vec<lsp_types::CompletionItem> = completion_items
        .into_iter()
        .map(|item| lsp_types::CompletionItem {
            label: item.label,
            kind: item.kind.map(|k| match k {
                tower_lsp::lsp_types::CompletionItemKind::TEXT => lsp_types::CompletionItemKind::TEXT,
                tower_lsp::lsp_types::CompletionItemKind::METHOD => lsp_types::CompletionItemKind::METHOD,
                tower_lsp::lsp_types::CompletionItemKind::FUNCTION => lsp_types::CompletionItemKind::FUNCTION,
                tower_lsp::lsp_types::CompletionItemKind::CONSTRUCTOR => lsp_types::CompletionItemKind::CONSTRUCTOR,
                tower_lsp::lsp_types::CompletionItemKind::FIELD => lsp_types::CompletionItemKind::FIELD,
                tower_lsp::lsp_types::CompletionItemKind::VARIABLE => lsp_types::CompletionItemKind::VARIABLE,
                tower_lsp::lsp_types::CompletionItemKind::CLASS => lsp_types::CompletionItemKind::CLASS,
                tower_lsp::lsp_types::CompletionItemKind::INTERFACE => lsp_types::CompletionItemKind::INTERFACE,
                tower_lsp::lsp_types::CompletionItemKind::MODULE => lsp_types::CompletionItemKind::MODULE,
                tower_lsp::lsp_types::CompletionItemKind::PROPERTY => lsp_types::CompletionItemKind::PROPERTY,
                tower_lsp::lsp_types::CompletionItemKind::UNIT => lsp_types::CompletionItemKind::UNIT,
                tower_lsp::lsp_types::CompletionItemKind::VALUE => lsp_types::CompletionItemKind::VALUE,
                tower_lsp::lsp_types::CompletionItemKind::ENUM => lsp_types::CompletionItemKind::ENUM,
                tower_lsp::lsp_types::CompletionItemKind::KEYWORD => lsp_types::CompletionItemKind::KEYWORD,
                tower_lsp::lsp_types::CompletionItemKind::SNIPPET => lsp_types::CompletionItemKind::SNIPPET,
                tower_lsp::lsp_types::CompletionItemKind::COLOR => lsp_types::CompletionItemKind::COLOR,
                tower_lsp::lsp_types::CompletionItemKind::FILE => lsp_types::CompletionItemKind::FILE,
                tower_lsp::lsp_types::CompletionItemKind::REFERENCE => lsp_types::CompletionItemKind::REFERENCE,
                tower_lsp::lsp_types::CompletionItemKind::FOLDER => lsp_types::CompletionItemKind::FOLDER,
                tower_lsp::lsp_types::CompletionItemKind::ENUM_MEMBER => lsp_types::CompletionItemKind::ENUM_MEMBER,
                tower_lsp::lsp_types::CompletionItemKind::CONSTANT => lsp_types::CompletionItemKind::CONSTANT,
                tower_lsp::lsp_types::CompletionItemKind::STRUCT => lsp_types::CompletionItemKind::STRUCT,
                tower_lsp::lsp_types::CompletionItemKind::EVENT => lsp_types::CompletionItemKind::EVENT,
                tower_lsp::lsp_types::CompletionItemKind::OPERATOR => lsp_types::CompletionItemKind::OPERATOR,
                tower_lsp::lsp_types::CompletionItemKind::TYPE_PARAMETER => lsp_types::CompletionItemKind::TYPE_PARAMETER,
                _ => lsp_types::CompletionItemKind::TEXT,
            }),
            detail: item.detail,
            ..Default::default()
        })
        .collect();

    validate_completion(&lsp_items, test)?;

    // 10. Cleanup
    drop(conn);
    lsp_runner.kill().await?;
    adapter.cleanup().await?;

    info!("Test passed: {}", test.description);
    Ok(())
}

/// Run all test cases from a text file
pub async fn run_test_file(path: impl AsRef<std::path::Path>) -> Result<()> {
    let tests = parse_test_file(path.as_ref())?;

    info!("Running {} tests from {:?}", tests.len(), path.as_ref());

    for test in &tests {
        run_test(test).await?;
    }

    Ok(())
}

/// Run all test case files in a directory
pub async fn run_test_directory(dir_path: impl AsRef<std::path::Path>) -> Result<()> {
    let dir = std::fs::read_dir(dir_path)?;

    let mut test_files: Vec<_> = dir
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension().map(|ext| ext == "txt").unwrap_or(false)
        })
        .collect();

    test_files.sort_by_key(|entry| entry.path());

    info!("Found {} test files in directory", test_files.len());

    for entry in test_files {
        let path = entry.path();
        info!("Running test file: {:?}", path);

        if let Err(e) = run_test_file(&path).await {
            tracing::error!("Failed to run test file {:?}: {}", path, e);
            // Continue with other test files
        }
    }

    Ok(())
}

/// Convert byte offset to LSP Position
fn offset_to_position(text: &str, offset: usize) -> Result<tower_lsp::lsp_types::Position> {
    let mut line = 0;
    let mut character = 0;
    let mut current_offset = 0;

    for ch in text.chars() {
        if current_offset == offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }

        current_offset += ch.len_utf8();
    }

    Ok(tower_lsp::lsp_types::Position {
        line,
        character: character as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_to_position_simple() {
        let text = "SELECT * FROM users";
        let pos = offset_to_position(text, 7).unwrap();
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 7);
    }

    #[test]
    fn test_offset_to_position_multiline() {
        let text = "SELECT *\nFROM users";
        let pos = offset_to_position(text, 10).unwrap();
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 1);
    }
}
