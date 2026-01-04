/**
 * MySQL Dialect
 *
 * MySQL-specific grammar extensions.
 * This file extends the base SQL grammar with MySQL-specific features.
 */

module.exports = {
  // =============================================================================
  // Statement Extensions
  // =============================================================================

  // Override statement to include MySQL-specific REPLACE
  // We must include all base statements plus MySQL extensions
  statement: $ => choice(
    $.select_statement,
    $.insert_statement,
    $.update_statement,
    $.delete_statement,
    $.replace_statement  // MySQL-specific REPLACE statement
  ),

  // =============================================================================
  // MySQL-Specific Statements
  // =============================================================================

  replace_statement: $ => seq(
    'REPLACE',
    optional('IGNORE'),
    'INTO',
    $.table_name,
    optional($.column_list),
    'VALUES',
    $.value_list,
    repeat(seq(',', $.value_list))
  ),

  // =============================================================================
  // Clause Extensions
  // =============================================================================

  // Override select_modifier to add MySQL-specific options
  // Base select_modifier is empty (seq()), MySQL adds actual options
  select_modifier: $ => choice(
    'SQL_CALC_FOUND_ROWS',
    seq('SQL_CACHE', optional('SQL_CALC_FOUND_ROWS')),
    seq('SQL_NO_CACHE', optional('SQL_CALC_FOUND_ROWS'))
  ),

  // Override limit_clause to add MySQL's "LIMIT offset, count" syntax
  limit_clause: $ => choice(
    seq('LIMIT', $.expression),
    seq('LIMIT', $.expression, 'OFFSET', $.expression),
    seq('LIMIT', $.expression, ',', $.expression)  // MySQL-specific syntax
  ),

  // =============================================================================
  // Identifier Extensions
  // =============================================================================

  // Override identifier to prioritize MySQL's backtick notation
  identifier: $ => choice(
    /`[^`]+`/,        // MySQL backtick (highest priority)
    /[a-zA-Z_][a-zA-Z0-9_]*/,  // Regular identifier
    /"[^"]+"/,        // PostgreSQL style (still supported)
    /\[[^\]]+\]/      // SQL Server style (still supported)
  ),
};
