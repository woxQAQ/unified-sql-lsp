/**
 * PostgreSQL 14 Dialect
 *
 * PostgreSQL 14 extends the PostgreSQL 12 dialect with new features:
 * - JSON subscripting (jsonb['key']['subkey'])
 * - Stored procedures with OUT parameters
 * - SQL-standard SEARCH and CYCLE for CTEs
 * - Multirange data types
 *
 * Source: PostgreSQL 14 Release Notes
 * https://www.postgresql.org/docs/release/14.0/
 */

const basePg12 = require('./postgresql-12.js');

module.exports = {
  // Merge PostgreSQL 12 features as base
  ...basePg12,

  // =============================================================================
  // PostgreSQL 14-Specific: JSON Subscripting
  // https://www.postgresql.org/docs/release/14.0.html
  // "Allow subscripting jsonb and json data"
  // =============================================================================

  // Extend expression to support JSON subscripting
  // NEW in PG 14: json_column['key']['subkey'] syntax
  // Note: This is a simplified version - full implementation would be more complex

  // =============================================================================
  // PostgreSQL 14-Specific: SEARCH and CYCLE for CTEs
  // https://www.postgresql.org/docs/release/14.0.html
  // "Add SEARCH and CYCLE clauses for recursive queries"
  // =============================================================================

  // Extend cte_clause to support SEARCH and CYCLE
  cte_clause: $ => seq(
    'WITH',
    optional('RECURSIVE'),
    $.cte_definition,
    repeat(seq(',', $.cte_definition)),
    // NEW: SEARCH clause for tracking order in recursive CTEs
    optional(seq('SEARCH',
      choice('DEPTH', 'BREADTH'),
      'FIRST', 'BY',
      $.expression,
      repeat(seq(',', $.expression)),
      'SET',
      $.column_name
    )),
    // NEW: CYCLE clause to detect cycles in recursive CTEs
    optional(seq('CYCLE',
      $.column_name,
      repeat(seq(',', $.column_name)),
      'SET',
      $.column_name,
      'USING',
      $.column_name
    ))
  ),

  // =============================================================================
  // PostgreSQL 14-Specific: Multirange Types
  // https://www.postgresql.org/docs/release/14.0.html
  // "Add multirange types"
  // =============================================================================

  // Multirange types would be handled in type definitions
  // This is a placeholder for future implementation

  // =============================================================================
  // PostgreSQL 14-Specific: Stored Procedures with OUT Parameters
  // https://www.postgresql.org/docs/release/14.0.html
  // =============================================================================

  // Procedure creation with OUT parameters
  // This would be part of CREATE PROCEDURE statement
  // Placeholder for future implementation
};
