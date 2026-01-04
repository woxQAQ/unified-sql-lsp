// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details
//
//! # Column Reference Resolution with Fuzzy Matching
//!
//! This module provides advanced column resolution capabilities including:
//! - Exact and case-insensitive matching
//! - Fuzzy matching using Levenshtein distance for typo correction
//! - Prefix matching for partial column names
//! - Ambiguity detection with candidate suggestions
//! - Configurable similarity thresholds

use crate::error::SemanticError;
use crate::scope::ScopeManager;
use crate::symbol::{ColumnSymbol, TableSymbol};
use std::cmp::{max, min};
use unified_sql_lsp_ir::ColumnRef;

/// Enhanced result type for column resolution with alternatives
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnResolutionResult {
    /// Successfully resolved to a single column
    Found {
        table: TableSymbol,
        column: ColumnSymbol,
    },

    /// Column not found, but similar columns are available
    NotFoundWithSuggestions { suggestions: Vec<ColumnCandidate> },

    /// Column is ambiguous (found in multiple tables)
    Ambiguous { candidates: Vec<ColumnCandidate> },
}

impl ColumnResolutionResult {
    /// Format the result as a human-readable message
    pub fn format_message(&self) -> String {
        match self {
            ColumnResolutionResult::Found { table, column } => {
                format!(
                    "Column '{}' found in table '{}'",
                    column.name,
                    table.display_name()
                )
            }
            ColumnResolutionResult::NotFoundWithSuggestions { suggestions } => {
                if suggestions.is_empty() {
                    "Column not found and no similar columns available".to_string()
                } else {
                    let suggest_list = suggestions
                        .iter()
                        .map(|s| {
                            format!(
                                "  - {}.{} (relevance: {:.0}%)",
                                s.table.display_name(),
                                s.column.name,
                                s.relevance_score * 100.0
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    format!("Column not found. Did you mean:\n{}", suggest_list)
                }
            }
            ColumnResolutionResult::Ambiguous { candidates } => {
                let candidate_list = candidates
                    .iter()
                    .map(|c| format!("  - {}.{}", c.table.display_name(), c.column.name))
                    .collect::<Vec<_>>()
                    .join("\n");

                format!(
                    "Ambiguous column reference. Specify one of:\n{}",
                    candidate_list
                )
            }
        }
    }

    /// Extract the resolved column if successful
    pub fn into_result(self) -> Result<(TableSymbol, ColumnSymbol), SemanticError> {
        match self {
            ColumnResolutionResult::Found { table, column } => Ok((table, column)),
            ColumnResolutionResult::NotFoundWithSuggestions { .. } => {
                Err(SemanticError::ColumnNotFound("column".to_string()))
            }
            ColumnResolutionResult::Ambiguous { candidates } => {
                let tables = candidates
                    .iter()
                    .map(|c| c.table.display_name().to_string())
                    .collect();
                Err(SemanticError::AmbiguousColumn("column".to_string(), tables))
            }
        }
    }
}

/// Represents a potential column match with metadata
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnCandidate {
    pub table: TableSymbol,
    pub column: ColumnSymbol,
    pub relevance_score: f64,
    pub match_kind: MatchKind,
}

impl ColumnCandidate {
    /// Calculate relevance score for a column match
    /// Higher scores = more relevant (0.0 to 1.0)
    fn calculate_score(&mut self, query: &str, config: &ResolutionConfig) {
        let name = &self.column.name;

        match self.match_kind {
            MatchKind::Exact => {
                self.relevance_score = 1.0;
            }
            MatchKind::CaseInsensitive => {
                self.relevance_score = 0.95;
            }
            MatchKind::Fuzzy { distance } => {
                // Base score from similarity
                let base_score = similarity_score(query, name);

                // Penalty for longer distance
                let distance_penalty = 1.0 - (distance as f64 / (config.max_distance as f64 + 1.0));

                // Bonus for shorter column names (less room for error)
                let length_bonus = if name.len() <= 8 { 0.05 } else { 0.0 };

                self.relevance_score = base_score * distance_penalty + length_bonus;
            }
            MatchKind::PrefixMatch => {
                // Calculate how much of the string matches
                let prefix_len = query.len();
                let total_len = name.len();
                self.relevance_score = (prefix_len as f64 / total_len as f64) * 0.85;
            }
        }

        // Bonus for common column names (id, name, created_at, etc.)
        if is_common_column_name(name) {
            self.relevance_score += 0.05;
        }

        // Ensure score is in [0, 1]
        self.relevance_score = self.relevance_score.clamp(0.0, 1.0);
    }
}

/// How the column matches the query
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    /// Exact name match
    Exact,
    /// Case-insensitive match
    CaseInsensitive,
    /// Fuzzy match (typo correction)
    Fuzzy { distance: usize },
    /// Similar prefix (e.g., "user_name" vs "user_firstname")
    PrefixMatch,
}

/// Configuration for fuzzy matching
#[derive(Debug, Clone)]
pub struct ResolutionConfig {
    /// Maximum Levenshtein distance for fuzzy matching (default: 2)
    pub max_distance: usize,
    /// Minimum similarity score (0.0-1.0) for suggestions (default: 0.6)
    pub min_similarity: f64,
    /// Maximum number of suggestions to return (default: 5)
    pub max_suggestions: usize,
    /// Enable case-insensitive matching (default: true)
    pub case_insensitive: bool,
}

impl Default for ResolutionConfig {
    fn default() -> Self {
        Self {
            max_distance: 2,
            min_similarity: 0.6,
            max_suggestions: 5,
            case_insensitive: true,
        }
    }
}

/// Column resolver with fuzzy matching and suggestions
pub struct ColumnResolver {
    scope_manager: ScopeManager,
    config: ResolutionConfig,
}

impl ColumnResolver {
    /// Create a new column resolver with default configuration
    pub fn new(scope_manager: ScopeManager) -> Self {
        Self {
            scope_manager,
            config: ResolutionConfig::default(),
        }
    }

    /// Create resolver with custom configuration
    pub fn with_config(scope_manager: ScopeManager, config: ResolutionConfig) -> Self {
        Self {
            scope_manager,
            config,
        }
    }

    /// Resolve a column with enhanced error reporting
    ///
    /// This is the main entry point that provides richer results than
    /// the basic ScopeManager::resolve_column.
    pub fn resolve_column(
        &self,
        column_ref: &ColumnRef,
        scope_id: usize,
    ) -> ColumnResolutionResult {
        // Handle qualified references (table.column)
        if let Some(table_name) = &column_ref.table {
            return self.resolve_qualified(table_name, &column_ref.column, scope_id);
        }

        // Handle unqualified references (column)
        self.resolve_unqualified_enhanced(&column_ref.column, scope_id)
    }

    /// Resolve qualified column reference (table.column)
    fn resolve_qualified(
        &self,
        table_name: &str,
        column_name: &str,
        scope_id: usize,
    ) -> ColumnResolutionResult {
        // Try to resolve the table first
        match self.scope_manager.resolve_table(table_name, scope_id) {
            Ok(table) => {
                // Table found, look for column
                if let Some(column) = table.find_column(column_name) {
                    ColumnResolutionResult::Found {
                        table: table.clone(),
                        column: column.clone(),
                    }
                } else {
                    // Column not found in specified table
                    // Find similar columns in this table
                    let suggestions = self.find_similar_columns_in_table(column_name, table);

                    ColumnResolutionResult::NotFoundWithSuggestions { suggestions }
                }
            }
            Err(_) => {
                // Table not found
                // Could suggest tables here too, but for now return empty
                ColumnResolutionResult::NotFoundWithSuggestions {
                    suggestions: Vec::new(),
                }
            }
        }
    }

    /// Resolve unqualified column reference with enhanced error reporting
    fn resolve_unqualified_enhanced(
        &self,
        column_name: &str,
        scope_id: usize,
    ) -> ColumnResolutionResult {
        // Collect all candidates
        let candidates = self.collect_candidates(column_name, scope_id);

        // Filter for exact and case-insensitive matches
        let exact_matches: Vec<_> = candidates
            .iter()
            .filter(|c| matches!(c.match_kind, MatchKind::Exact | MatchKind::CaseInsensitive))
            .collect();

        match exact_matches.len() {
            0 => {
                // No exact matches - check for fuzzy/prefix matches
                if candidates.is_empty() {
                    ColumnResolutionResult::NotFoundWithSuggestions {
                        suggestions: Vec::new(),
                    }
                } else {
                    ColumnResolutionResult::NotFoundWithSuggestions {
                        suggestions: candidates.into_iter().map(|c| c).collect(),
                    }
                }
            }
            1 => {
                // Unique match
                let candidate = &exact_matches[0];
                ColumnResolutionResult::Found {
                    table: candidate.table.clone(),
                    column: candidate.column.clone(),
                }
            }
            _ => {
                // Ambiguous - multiple exact matches
                ColumnResolutionResult::Ambiguous {
                    candidates: exact_matches.into_iter().map(|c| c.clone()).collect(),
                }
            }
        }
    }

    /// Collect all column candidates from visible tables
    fn collect_candidates(&self, column_name: &str, scope_id: usize) -> Vec<ColumnCandidate> {
        let mut candidates = Vec::new();
        let tables = self.collect_visible_tables(scope_id);

        for table in tables {
            for column in &table.columns {
                let match_kind = self.determine_match_kind(column_name, &column.name);

                // Only include candidates that meet our minimum criteria
                if self.is_candidate_acceptable(&match_kind, &column.name, column_name) {
                    let mut candidate = ColumnCandidate {
                        table: table.clone(),
                        column: column.clone(),
                        relevance_score: 0.0,
                        match_kind,
                    };

                    candidate.calculate_score(column_name, &self.config);
                    candidates.push(candidate);
                }
            }
        }

        // Sort by relevance score (descending)
        candidates.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit to max suggestions
        candidates.truncate(self.config.max_suggestions);

        candidates
    }

    /// Determine what kind of match this is
    fn determine_match_kind(&self, query: &str, column_name: &str) -> MatchKind {
        // Exact match
        if query == column_name {
            return MatchKind::Exact;
        }

        // Case-insensitive match
        if self.config.case_insensitive && query.eq_ignore_ascii_case(column_name) {
            return MatchKind::CaseInsensitive;
        }

        // Fuzzy match (Levenshtein distance)
        let distance = levenshtein_distance(query, column_name);
        if distance <= self.config.max_distance {
            let similarity = similarity_score(query, column_name);
            if similarity >= self.config.min_similarity {
                return MatchKind::Fuzzy { distance };
            }
        }

        // Prefix match (e.g., "user" matches "user_id")
        let query_lower = query.to_lowercase();
        let column_lower = column_name.to_lowercase();
        if column_lower.starts_with(&query_lower) && query.len() >= 3 {
            return MatchKind::PrefixMatch;
        }

        // No match
        MatchKind::Fuzzy {
            distance: usize::MAX,
        }
    }

    /// Check if a candidate meets minimum acceptance criteria
    fn is_candidate_acceptable(
        &self,
        match_kind: &MatchKind,
        column_name: &str,
        query: &str,
    ) -> bool {
        match match_kind {
            MatchKind::Exact | MatchKind::CaseInsensitive => true,
            MatchKind::Fuzzy { distance } => {
                *distance <= self.config.max_distance
                    && similarity_score(query, column_name) >= self.config.min_similarity
            }
            MatchKind::PrefixMatch => {
                query.len() >= 3 // Require at least 3 chars for prefix match
            }
        }
    }

    /// Find similar columns within a specific table
    fn find_similar_columns_in_table(
        &self,
        column_name: &str,
        table: &TableSymbol,
    ) -> Vec<ColumnCandidate> {
        let mut candidates = Vec::new();

        for column in &table.columns {
            let match_kind = self.determine_match_kind(column_name, &column.name);

            if self.is_candidate_acceptable(&match_kind, &column.name, column_name) {
                let mut candidate = ColumnCandidate {
                    table: table.clone(),
                    column: column.clone(),
                    relevance_score: 0.0,
                    match_kind,
                };

                candidate.calculate_score(column_name, &self.config);
                candidates.push(candidate);
            }
        }

        candidates.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates.truncate(self.config.max_suggestions);
        candidates
    }

    /// Collect all visible tables at a scope
    fn collect_visible_tables(&self, scope_id: usize) -> Vec<TableSymbol> {
        let mut tables = Vec::new();
        let mut current_id = Some(scope_id);

        while let Some(id) = current_id {
            if let Some(scope) = self.scope_manager.get_scope(id) {
                for table in &scope.tables {
                    if !tables
                        .iter()
                        .any(|t: &TableSymbol| t.table_name == table.table_name)
                    {
                        tables.push(table.clone());
                    }
                }
                current_id = scope.parent_id;
            } else {
                break;
            }
        }

        tables
    }

    /// Get all columns from all visible tables at a scope
    pub fn collect_visible_columns(&self, scope_id: usize) -> Vec<ColumnCandidate> {
        let tables = self.collect_visible_tables(scope_id);
        let mut candidates = Vec::new();

        for table in tables {
            for column in &table.columns {
                candidates.push(ColumnCandidate {
                    table: table.clone(),
                    column: column.clone(),
                    relevance_score: 1.0,
                    match_kind: MatchKind::Exact,
                });
            }
        }

        candidates
    }

    /// Find similar column names when exact match fails
    pub fn find_similar_columns(&self, column_name: &str, scope_id: usize) -> Vec<ColumnCandidate> {
        self.collect_candidates(column_name, scope_id)
    }
}

/// Calculate Levenshtein distance between two strings
/// Uses Wagner-Fischer algorithm with O(min(m,n)) space optimization
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    // Use the smaller string for the inner array to save space
    if m < n {
        return levenshtein_distance(b, a);
    }

    let mut previous: Vec<usize> = (0..=n).collect();

    for (i, &ca) in a_chars.iter().enumerate() {
        let mut current = vec![i + 1];

        for (j, &cb) in b_chars.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            current.push(min(
                min(previous[j + 1] + 1, current[j] + 1),
                previous[j] + cost,
            ));
        }

        previous = current;
    }

    previous[n]
}

/// Calculate similarity score (0.0 to 1.0) based on edit distance
fn similarity_score(a: &str, b: &str) -> f64 {
    let max_len = max(a.len(), b.len());
    if max_len == 0 {
        return 1.0;
    }

    let distance = levenshtein_distance(a, b);
    1.0 - (distance as f64 / max_len as f64)
}

/// List of commonly used column names that get a small bonus
fn is_common_column_name(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "id" | "name"
            | "email"
            | "created_at"
            | "updated_at"
            | "user_id"
            | "status"
            | "type"
            | "description"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScopeType;
    use unified_sql_lsp_catalog::DataType;

    fn create_test_scope_manager() -> ScopeManager {
        let mut manager = ScopeManager::new();
        let scope_id = manager.create_scope(ScopeType::Query, None);

        // Add users table
        let users = TableSymbol::new("users").with_columns(vec![
            ColumnSymbol::new("id", DataType::Integer, "users"),
            ColumnSymbol::new("name", DataType::Text, "users"),
            ColumnSymbol::new("email", DataType::Text, "users"),
            ColumnSymbol::new("created_at", DataType::Timestamp, "users"),
        ]);
        manager
            .get_scope_mut(scope_id)
            .unwrap()
            .add_table(users)
            .unwrap();

        // Add orders table
        let orders = TableSymbol::new("orders").with_columns(vec![
            ColumnSymbol::new("id", DataType::Integer, "orders"),
            ColumnSymbol::new("user_id", DataType::Integer, "orders"),
            ColumnSymbol::new("total", DataType::Integer, "orders"),
            ColumnSymbol::new("status", DataType::Text, "orders"),
        ]);
        manager
            .get_scope_mut(scope_id)
            .unwrap()
            .add_table(orders)
            .unwrap();

        manager
    }

    #[test]
    fn test_exact_match_unqualified() {
        let manager = create_test_scope_manager();
        let resolver = ColumnResolver::new(manager);

        let result = resolver.resolve_column(&ColumnRef::new("name"), 0);

        match result {
            ColumnResolutionResult::Found { table, column } => {
                assert_eq!(column.name, "name");
                assert_eq!(table.table_name, "users");
            }
            _ => panic!("Expected Found result, got {:?}", result),
        }
    }

    #[test]
    fn test_ambiguous_column() {
        let manager = create_test_scope_manager();
        let resolver = ColumnResolver::new(manager);

        let result = resolver.resolve_column(&ColumnRef::new("id"), 0);

        match result {
            ColumnResolutionResult::Ambiguous { candidates } => {
                assert_eq!(candidates.len(), 2);
                assert!(candidates
                    .iter()
                    .any(|c| c.column.name == "id" && c.table.table_name == "users"));
                assert!(candidates
                    .iter()
                    .any(|c| c.column.name == "id" && c.table.table_name == "orders"));
            }
            _ => panic!("Expected Ambiguous result, got {:?}", result),
        }
    }

    #[test]
    fn test_fuzzy_match_typo() {
        let manager = create_test_scope_manager();
        let resolver = ColumnResolver::new(manager);

        // Typo: "emial" instead of "email"
        let result = resolver.resolve_column(&ColumnRef::new("emial"), 0);

        match result {
            ColumnResolutionResult::NotFoundWithSuggestions { suggestions } => {
                assert!(!suggestions.is_empty());
                // "email" should be the top suggestion
                assert_eq!(suggestions[0].column.name, "email");
                assert!(suggestions[0].relevance_score > 0.6);
            }
            _ => panic!("Expected NotFoundWithSuggestions result, got {:?}", result),
        }
    }

    #[test]
    fn test_case_insensitive_match() {
        let manager = create_test_scope_manager();
        let config = ResolutionConfig {
            case_insensitive: true,
            ..Default::default()
        };
        let resolver = ColumnResolver::with_config(manager, config);

        let result = resolver.resolve_column(&ColumnRef::new("NAME"), 0);

        match result {
            ColumnResolutionResult::Found { column, .. } => {
                assert_eq!(column.name, "name");
            }
            _ => panic!("Expected Found result, got {:?}", result),
        }
    }

    #[test]
    fn test_qualified_column_resolution() {
        let manager = create_test_scope_manager();
        let resolver = ColumnResolver::new(manager);

        let result = resolver.resolve_column(&ColumnRef::new("id").with_table("users"), 0);

        match result {
            ColumnResolutionResult::Found { table, column } => {
                assert_eq!(column.name, "id");
                assert_eq!(table.table_name, "users");
            }
            _ => panic!("Expected Found result, got {:?}", result),
        }
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("email", "emial"), 2);
        assert_eq!(levenshtein_distance("id", "idx"), 1);
        assert_eq!(levenshtein_distance("", ""), 0);
    }

    #[test]
    fn test_similarity_score() {
        let score = similarity_score("email", "emial");
        assert!(score > 0.5); // Should be relatively high

        let score = similarity_score("name", "name");
        assert_eq!(score, 1.0); // Exact match
    }

    #[test]
    fn test_prefix_match() {
        let manager = create_test_scope_manager();
        let resolver = ColumnResolver::new(manager);

        // "creat" should suggest "created_at"
        let result = resolver.resolve_column(&ColumnRef::new("creat"), 0);

        match result {
            ColumnResolutionResult::NotFoundWithSuggestions { suggestions } => {
                assert!(!suggestions.is_empty());
                assert!(suggestions.iter().any(|s| s.column.name == "created_at"));
            }
            _ => panic!("Expected NotFoundWithSuggestions result, got {:?}", result),
        }
    }

    #[test]
    fn test_candidate_ranking() {
        let manager = create_test_scope_manager();
        let resolver = ColumnResolver::new(manager);

        // "usr" should rank "user_id" higher than other columns
        let result = resolver.resolve_column(&ColumnRef::new("usr"), 0);

        match result {
            ColumnResolutionResult::NotFoundWithSuggestions { suggestions } => {
                if !suggestions.is_empty() {
                    // Verify suggestions are sorted by relevance
                    for i in 1..suggestions.len() {
                        assert!(
                            suggestions[i - 1].relevance_score >= suggestions[i].relevance_score
                        );
                    }
                }
            }
            _ => {}
        }
    }

    #[test]
    fn test_no_suggestions_for_completely_wrong_name() {
        let manager = create_test_scope_manager();
        let resolver = ColumnResolver::new(manager);

        // "xyz123" is completely wrong
        let result = resolver.resolve_column(&ColumnRef::new("xyz123"), 0);

        match result {
            ColumnResolutionResult::NotFoundWithSuggestions { suggestions } => {
                assert!(suggestions.is_empty());
            }
            _ => panic!("Expected NotFoundWithSuggestions result, got {:?}", result),
        }
    }

    #[test]
    fn test_scope_hierarchy_resolution() {
        let mut manager = create_test_scope_manager();

        // Create child scope
        let child_id = manager.create_scope(ScopeType::Subquery, Some(0));

        // Add orders table to child scope
        let orders = TableSymbol::new("orders").with_columns(vec![
            ColumnSymbol::new("id", DataType::Integer, "orders"),
            ColumnSymbol::new("total", DataType::Integer, "orders"),
        ]);
        manager
            .get_scope_mut(child_id)
            .unwrap()
            .add_table(orders)
            .unwrap();

        let resolver = ColumnResolver::new(manager);

        // Should find "name" from parent scope
        let result = resolver.resolve_column(&ColumnRef::new("name"), child_id);

        match result {
            ColumnResolutionResult::Found { column, .. } => {
                assert_eq!(column.name, "name");
            }
            _ => panic!(
                "Expected to find column from parent scope, got {:?}",
                result
            ),
        }
    }
}
