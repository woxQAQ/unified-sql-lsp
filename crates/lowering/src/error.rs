// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Error types and handling strategy for the lowering layer

use serde::Serialize;

/// Result type alias for lowering operations
pub type LoweringResult<T> = Result<T, LoweringError>;

/// Outcome of a lowering operation
///
/// Represents the three possible states after attempting to lower CST to IR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoweringOutcome {
    /// Complete success - all nodes converted successfully
    Success,

    /// Partial success - some nodes converted, others have placeholders
    /// Contains a vector of errors that occurred during conversion
    Partial(Vec<LoweringError>),

    /// Complete failure - critical error prevented conversion
    Failed(LoweringError),
}

/// Errors that can occur during CST → IR lowering
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq, Serialize)]
pub enum LoweringError {
    /// The CST node is missing a required child
    #[error("Missing required child node: expected '{expected}', but not found in {context}")]
    MissingChild { context: String, expected: String },

    /// Unexpected node type encountered
    #[error("Unexpected node type: expected '{expected}', found '{found}'")]
    UnexpectedNodeType { expected: String, found: String },

    /// Invalid literal value
    #[error("Invalid literal value: {value} cannot be parsed as {type_name}")]
    InvalidLiteral { value: String, type_name: String },

    /// Syntax feature not supported by the dialect
    #[error("Syntax not supported by {dialect}: {feature}. {suggestion}")]
    UnsupportedSyntax {
        dialect: String,
        feature: String,
        suggestion: String,
    },

    /// Ambiguous syntax that requires disambiguation
    #[error("Ambiguous syntax: {message}. Consider {suggestion}")]
    AmbiguousSyntax { message: String, suggestion: String },

    /// Recursion limit exceeded (e.g., deeply nested subqueries)
    #[error("Recursion limit exceeded: {context} (depth: {depth}, limit: {limit})")]
    RecursionLimitExceeded {
        context: String,
        depth: usize,
        limit: usize,
    },

    /// Source mapping failure (cannot determine original location)
    #[error("Source mapping failed: {message}")]
    SourceMappingFailed { message: String },

    /// Generic lowering error for other cases
    #[error("Lowering error: {message}")]
    Generic { message: String },
}

impl LoweringError {
    /// Check if this error is recoverable (allows partial success)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            LoweringError::UnsupportedSyntax { .. }
                | LoweringError::InvalidLiteral { .. }
                | LoweringError::AmbiguousSyntax { .. }
        )
    }

    /// Get the severity level of this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            LoweringError::MissingChild { .. } => ErrorSeverity::Error,
            LoweringError::UnexpectedNodeType { .. } => ErrorSeverity::Error,
            LoweringError::RecursionLimitExceeded { .. } => ErrorSeverity::Error,
            LoweringError::UnsupportedSyntax { .. } => ErrorSeverity::Warning,
            LoweringError::InvalidLiteral { .. } => ErrorSeverity::Warning,
            LoweringError::AmbiguousSyntax { .. } => ErrorSeverity::Info,
            LoweringError::SourceMappingFailed { .. } => ErrorSeverity::Error,
            LoweringError::Generic { .. } => ErrorSeverity::Error,
        }
    }
}

/// Severity level for lowering errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Informational note (e.g., ambiguous syntax)
    Info,
    /// Warning (e.g., unsupported feature that was skipped)
    Warning,
    /// Error (e.g., critical structural issue)
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_missing_child() {
        let err = LoweringError::MissingChild {
            context: "SELECT statement".to_string(),
            expected: "FROM clause".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Missing required child"));
        assert!(msg.contains("SELECT statement"));
        assert!(msg.contains("FROM clause"));
    }

    #[test]
    fn test_error_display_unsupported_syntax() {
        let err = LoweringError::UnsupportedSyntax {
            dialect: "MySQL".to_string(),
            feature: "LATERAL JOIN".to_string(),
            suggestion: "Use a subquery instead".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("MySQL"));
        assert!(msg.contains("LATERAL JOIN"));
        assert!(msg.contains("subquery"));
    }

    #[test]
    fn test_recoverable_errors() {
        assert!(
            LoweringError::UnsupportedSyntax {
                dialect: "MySQL".to_string(),
                feature: "CTE".to_string(),
                suggestion: "Upgrade to MySQL 8.0+".to_string(),
            }
            .is_recoverable()
        );

        assert!(
            !LoweringError::MissingChild {
                context: "query".to_string(),
                expected: "SELECT".to_string(),
            }
            .is_recoverable()
        );
    }

    #[test]
    fn test_error_severity() {
        let unsupported = LoweringError::UnsupportedSyntax {
            dialect: "MySQL".to_string(),
            feature: "CTE".to_string(),
            suggestion: "Upgrade".to_string(),
        };
        assert_eq!(unsupported.severity(), ErrorSeverity::Warning);

        let missing = LoweringError::MissingChild {
            context: "query".to_string(),
            expected: "SELECT".to_string(),
        };
        assert_eq!(missing.severity(), ErrorSeverity::Error);

        let ambiguous = LoweringError::AmbiguousSyntax {
            message: "Column reference is ambiguous".to_string(),
            suggestion: "Use table qualifier".to_string(),
        };
        assert_eq!(ambiguous.severity(), ErrorSeverity::Info);
    }

    #[test]
    fn test_error_serialization() {
        let err = LoweringError::InvalidLiteral {
            value: "abc".to_string(),
            type_name: "integer".to_string(),
        };
        let json = serde_json::to_string(&err);
        assert!(json.is_ok());
    }

    #[test]
    fn test_outcome_failed() {
        let err = LoweringError::Generic {
            message: "Critical error".to_string(),
        };
        let outcome = LoweringOutcome::Failed(err);
        assert!(matches!(outcome, LoweringOutcome::Failed(_)));
    }
}
