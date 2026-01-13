// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Test utility functions
//!
//! Helper functions for SQL parsing, position calculation, etc.

use tower_lsp::lsp_types::Position;

/// Calculate LSP Position from SQL string with cursor marker
pub fn calculate_position(sql: &str) -> Position {
    let cursor_char = '|';

    let mut line = 0u32;
    let mut character = 0u32;

    for ch in sql.chars() {
        if ch == cursor_char {
            return Position::new(line, character);
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }

    Position::new(0, 0) // Default if not found
}

/// Strip cursor marker from SQL
pub fn strip_cursor_marker(sql: &str) -> String {
    sql.replace('|', "")
}

/// Validate cursor position is within bounds
pub fn validate_position(sql: &str, position: Position) -> bool {
    let lines: Vec<&str> = sql.lines().collect();

    if position.line as usize >= lines.len() {
        return false;
    }

    let line = lines[position.line as usize];
    position.character as usize <= line.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_position_simple() {
        let sql = "SELECT |";
        let pos = calculate_position(sql);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 7);
    }

    #[test]
    fn test_calculate_position_multiline() {
        let sql = "SELECT *\nFROM |";
        let pos = calculate_position(sql);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 5);
    }

    #[test]
    fn test_strip_cursor_marker() {
        let sql = "SELECT * FROM |";
        let stripped = strip_cursor_marker(sql);
        assert_eq!(stripped, "SELECT * FROM ");
    }

    #[test]
    fn test_validate_position_valid() {
        let sql = "SELECT * FROM users";
        let pos = Position::new(0, 10);
        assert!(validate_position(sql, pos));
    }

    #[test]
    fn test_validate_position_invalid_line() {
        let sql = "SELECT * FROM users";
        let pos = Position::new(5, 0);
        assert!(!validate_position(sql, pos));
    }

    #[test]
    fn test_validate_position_invalid_character() {
        let sql = "SELECT";
        let pos = Position::new(0, 100);
        assert!(!validate_position(sql, pos));
    }
}
