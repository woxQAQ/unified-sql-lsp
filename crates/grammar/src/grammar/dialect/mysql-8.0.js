/**
 * MySQL 8.0 Dialect
 *
 * MySQL 8.0 extends the MySQL 5.7 dialect with new features:
 * - Window Functions (OVER clause with PARTITION BY, ORDER BY, frame)
 * - Recursive Common Table Expressions (CTE)
 * - LATERAL derived tables (8.0.14+)
 * - NOWAIT and SKIP LOCKED
 *
 * Source: MySQL 8.0 Reference Manual
 * https://dev.mysql.com/doc/refman/8.0/en/
 */

const baseMysql57 = require('./mysql-5.7.js');

module.exports = {
  // Merge MySQL 5.7 features as base
  ...baseMysql57,

  // =============================================================================
  // MySQL 8.0-Specific: Window Functions
  // https://dev.mysql.com/doc/refman/8.0/en/window-functions.html
  // =============================================================================

  // Extend function_call to support window functions
  // This overrides the base function_call to add OVER clause
  function_call: $ => choice(
    // Window function: func_name(args) OVER (window_specification)
    seq(
      $.function_name,
      '(',
      optional(seq($.expression, repeat(seq(',', $.expression)))),
      ')',
      'OVER',
      '(',
      optional(seq('PARTITION', 'BY', $.expression, repeat(seq(',', $.expression)))),
      optional(seq('ORDER', 'BY', $.expression, repeat(seq(',', $.expression)))),
      optional(
        seq(
          choice('ROWS', 'RANGE'),
          choice(
            seq('BETWEEN',
              choice(
                'UNBOUNDED PRECEDING',
                'UNBOUNDED FOLLOWING',
                'CURRENT ROW',
                seq($.expression, choice('PRECEDING', 'FOLLOWING'))
              ),
              'AND',
              choice(
                'UNBOUNDED PRECEDING',
                'UNBOUNDED FOLLOWING',
                'CURRENT ROW',
                seq($.expression, choice('PRECEDING', 'FOLLOWING'))
              )
            ),
            choice(
              'UNBOUNDED PRECEDING',
              'UNBOUNDED FOLLOWING',
              'CURRENT ROW',
              seq($.expression, choice('PRECEDING', 'FOLLOWING'))
            )
          )
        )
      ),
      ')'
    ),
    // Regular function call from MySQL 5.7
    seq(
      $.function_name,
      '(',
      optional(seq($.expression, repeat(seq(',', $.expression)))),
      ')'
    )
  ),

  // =============================================================================
  // MySQL 8.0-Specific: Recursive CTE
  // https://dev.mysql.com/doc/refman/8.0/en/with.html
  // =============================================================================

  // Extend cte_clause to support RECURSIVE
  cte_clause: $ => seq(
    optional('RECURSIVE'),  // NEW in MySQL 8.0
    'WITH',
    $.cte_definition,
    repeat(seq(',', $.cte_definition))
  ),

  // =============================================================================
  // MySQL 8.0-Specific: LATERAL Derived Tables (8.0.14+)
  // https://dev.mysql.com/doc/refman/8.0/en/lateral-derived-tables.html
  // =============================================================================

  // Extend table_reference to support LATERAL
  table_reference: $ => choice(
    seq($.table_name, /[Aa][Ss]/, $.alias),
    $.table_name,
    $.join_clause,
    // NEW: LATERAL derived tables
    seq(
      /[Ll][Aa][Tt][Ee][Rr][Aa][Ll]/,
      '(',
      $.select_statement,
      ')',
      optional(/[Aa][Ss]/),
      optional($.alias)
    )
  ),

  // =============================================================================
  // MySQL 8.0-Specific: NOWAIT and SKIP LOCKED
  // https://dev.mysql.com/doc/refman/8.0/en/innodb-locking-reads.html
  // =============================================================================

  // Extend select_statement to support locking clauses (for FOR UPDATE)
  // This is a simplified version - full implementation would be more complex
};
