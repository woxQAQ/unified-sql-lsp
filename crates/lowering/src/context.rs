// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Lowering context for tracking state during conversion

use crate::error::{LoweringError, LoweringOutcome};
use std::collections::HashMap;
use unified_sql_lsp_ir::{Dialect, Expr};

/// Context for tracking state during CST → IR lowering
///
/// The context maintains:
/// - Accumulated errors for partial success mode
/// - Placeholder counter for generating unique placeholder names
/// - Recursion depth tracking
/// - Dialect information
/// - Source mappings from IR nodes back to CST nodes
pub struct LoweringContext {
    /// Target SQL dialect
    dialect: Dialect,

    /// Accumulated errors during lowering (for partial success)
    errors: Vec<LoweringError>,

    /// Placeholder counter for generating unique placeholder names
    placeholder_counter: usize,

    /// Current recursion depth (for detecting infinite loops)
    recursion_depth: usize,

    /// Maximum recursion depth allowed
    max_recursion_depth: usize,

    /// Source mappings from IR nodes to CST node locations
    /// Maps: "query:0" -> (line: 5, column: 10)
    source_mappings: HashMap<String, SourceLocation>,
}

/// Source location in the original SQL text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// Byte offset in the source (0-based)
    pub byte_offset: usize,
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based, UTF-8 bytes)
    pub column: usize,
}

impl LoweringContext {
    /// Create a new lowering context
    pub fn new(dialect: Dialect) -> Self {
        Self {
            dialect,
            errors: Vec::new(),
            placeholder_counter: 0,
            recursion_depth: 0,
            max_recursion_depth: 100,
            source_mappings: HashMap::new(),
        }
    }

    /// Create a new lowering context with custom max recursion depth
    pub fn with_max_depth(dialect: Dialect, max_depth: usize) -> Self {
        Self {
            dialect,
            errors: Vec::new(),
            placeholder_counter: 0,
            recursion_depth: 0,
            max_recursion_depth: max_depth,
            source_mappings: HashMap::new(),
        }
    }

    /// Get the target dialect
    pub fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// Check if the dialect supports a specific feature
    pub fn supports_feature(&self, feature: &str) -> bool {
        // This will interface with DialectExtensions in the future
        // For now, return true for core SQL features
        matches!(feature, "SELECT" | "FROM" | "WHERE" | "JOIN")
    }

    /// Add an error to the context (for partial success mode)
    pub fn add_error(&mut self, error: LoweringError) {
        self.errors.push(error);
    }

    /// Get all accumulated errors
    pub fn errors(&self) -> &[LoweringError] {
        &self.errors
    }

    /// Check if any errors were accumulated
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the lowering outcome based on accumulated errors
    pub fn outcome(&self) -> LoweringOutcome {
        if self.errors.is_empty() {
            LoweringOutcome::Success
        } else {
            LoweringOutcome::Partial(self.errors.clone())
        }
    }

    /// Create a placeholder expression for unsupported syntax
    ///
    /// This allows graceful degradation by inserting a placeholder
    /// when encountering unsupported or invalid syntax.
    ///
    /// # Arguments
    ///
    /// * `location` - Optional source location where the placeholder was created
    ///
    /// # Returns
    ///
    /// A placeholder expression with a unique name
    ///
    /// # Example
    ///
    /// ```ignore
    /// let placeholder = ctx.create_placeholder_with_location(Some(location));
    /// ```
    pub fn create_placeholder_with_location(&mut self, location: Option<SourceLocation>) -> Expr {
        let name = format!("__placeholder_{}", self.placeholder_counter);
        self.placeholder_counter += 1;

        // Store source mapping if provided
        if let Some(loc) = location {
            self.add_source_mapping(name.clone(), loc);
        }

        // Create a placeholder column reference
        Expr::Column(unified_sql_lsp_ir::ColumnRef::new(name))
    }

    /// Create a placeholder expression without source location
    ///
    /// Convenience method for creating placeholders when source location
    /// is not available or not needed.
    pub fn create_placeholder(&mut self) -> Expr {
        self.create_placeholder_with_location(None)
    }

    /// Add a source mapping from an IR node to a CST location
    pub fn add_source_mapping(&mut self, ir_id: String, location: SourceLocation) {
        self.source_mappings.insert(ir_id, location);
    }

    /// Get the source location for an IR node
    pub fn get_source_location(&self, ir_id: &str) -> Option<&SourceLocation> {
        self.source_mappings.get(ir_id)
    }

    /// Increment recursion depth and check for overflow
    pub fn enter_recursive_context(&mut self) -> Result<(), LoweringError> {
        self.recursion_depth += 1;
        if self.recursion_depth > self.max_recursion_depth {
            let error = LoweringError::RecursionLimitExceeded {
                context: "query lowering".to_string(),
                depth: self.recursion_depth,
                limit: self.max_recursion_depth,
            };
            self.add_error(error.clone());
            Err(error)
        } else {
            Ok(())
        }
    }

    /// Decrement recursion depth when exiting a recursive context
    pub fn exit_recursive_context(&mut self) {
        self.recursion_depth = self.recursion_depth.saturating_sub(1);
    }

    /// Clear all errors (useful for starting a new conversion)
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }
}

/// Create a SourceLocation from a tree-sitter node position
///
/// This utility function converts tree-sitter's Point structure
/// into our SourceLocation type for consistent error reporting.
///
/// # Arguments
///
/// * `byte_offset` - Byte offset of the node in the source
/// * `row` - Row number (0-based from tree-sitter)
/// * `column` - Column number (0-based from tree-sitter)
///
/// # Returns
///
/// A SourceLocation with 1-based line and column numbers
pub fn source_location_from_position(
    byte_offset: usize,
    row: usize,
    column: usize,
) -> SourceLocation {
    SourceLocation {
        byte_offset,
        line: row + 1,      // Convert to 1-based
        column: column + 1, // Convert to 1-based
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_accumulation() {
        let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

        ctx.add_error(LoweringError::Generic {
            message: "Test error 1".to_string(),
        });
        ctx.add_error(LoweringError::Generic {
            message: "Test error 2".to_string(),
        });

        assert!(ctx.has_errors());
        assert_eq!(ctx.errors().len(), 2);

        match ctx.outcome() {
            LoweringOutcome::Partial(errors) => {
                assert_eq!(errors.len(), 2);
            }
            _ => panic!("Expected Partial outcome"),
        }
    }

    #[test]
    fn test_placeholder_generation() {
        let mut ctx = LoweringContext::new(Dialect::MySQL);

        let p1 = ctx.create_placeholder();
        let p2 = ctx.create_placeholder();

        // Should generate unique placeholder names
        if let Expr::Column(col1) = p1 {
            assert!(col1.column.contains("__placeholder_0"));
        }
        if let Expr::Column(col2) = p2 {
            assert!(col2.column.contains("__placeholder_1"));
        }
    }

    #[test]
    fn test_recursion_tracking() {
        let mut ctx = LoweringContext::new(Dialect::MySQL);

        ctx.enter_recursive_context().unwrap();
        assert_eq!(ctx.recursion_depth, 1);

        ctx.exit_recursive_context();
        assert_eq!(ctx.recursion_depth, 0);
    }

    #[test]
    fn test_recursion_limit() {
        let mut ctx = LoweringContext::with_max_depth(Dialect::MySQL, 5);

        // Should succeed up to the limit
        for _ in 0..5 {
            ctx.enter_recursive_context().unwrap();
        }

        // Exceeds limit
        let result = ctx.enter_recursive_context();
        assert!(result.is_err());
    }

    #[test]
    fn test_source_mapping() {
        let mut ctx = LoweringContext::new(Dialect::MySQL);

        let location = SourceLocation {
            byte_offset: 100,
            line: 5,
            column: 10,
        };

        ctx.add_source_mapping("query:0".to_string(), location.clone());

        let retrieved = ctx.get_source_location("query:0");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().line, 5);
    }

    #[test]
    fn test_clear_errors() {
        let mut ctx = LoweringContext::new(Dialect::MySQL);

        ctx.add_error(LoweringError::Generic {
            message: "Test error".to_string(),
        });
        assert!(ctx.has_errors());

        ctx.clear_errors();
        assert!(!ctx.has_errors());
        assert!(matches!(ctx.outcome(), LoweringOutcome::Success));
    }

    #[test]
    fn test_supports_feature() {
        let ctx = LoweringContext::new(Dialect::MySQL);

        assert!(ctx.supports_feature("SELECT"));
        assert!(ctx.supports_feature("FROM"));
        assert!(ctx.supports_feature("WHERE"));
        assert!(ctx.supports_feature("JOIN"));

        // Unknown features return false for now
        assert!(!ctx.supports_feature("CTE"));
        assert!(!ctx.supports_feature("WINDOW"));
    }
}
