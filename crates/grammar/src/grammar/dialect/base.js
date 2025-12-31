/**
 * Base SQL Dialect
 *
 * This module defines the core SQL grammar rules that are common across most dialects.
 * It provides the foundation that specific dialects (MySQL, PostgreSQL, etc.) can extend.
 *
 * Core SQL statements supported:
 * - SELECT (basic queries)
 * - INSERT (insert records)
 * - UPDATE (update records)
 * - DELETE (delete records)
 */

module.exports = {
  // =============================================================================
  // Keywords - Common SQL keywords
  // =============================================================================

  keywords: $ => [
    'SELECT', 'FROM', 'WHERE',
    'INSERT', 'INTO', 'VALUES',
    'UPDATE', 'SET',
    'DELETE',
    'AND', 'OR', 'NOT',
    'NULL', 'IS',
    'AS', 'ON',
    'JOIN', 'INNER', 'LEFT', 'RIGHT', 'OUTER',
    'ORDER', 'BY', 'ASC', 'DESC',
    'GROUP', 'HAVING',
    'LIMIT', 'OFFSET',
    'DISTINCT',
    'CASE', 'WHEN', 'THEN', 'ELSE', 'END',
  ],

  // =============================================================================
  // Core Rules - Statement types
  // =============================================================================

  // Top-level statement rule
  statement: $ => choice(
    $.select_statement,
    $.insert_statement,
    $.update_statement,
    $.delete_statement
  ),

  // =============================================================================
  // SELECT Statement
  // =============================================================================

  select_statement: $ => seq(
    optional($.cte_clause),
    'SELECT',
    optional($.select_modifier),  // MySQL extension
    optional($.set_quantifier),
    $.projection,
    optional($.from_clause),
    optional($.where_clause),
    optional($.group_by_clause),
    optional($.having_clause),
    optional($.order_by_clause),
    optional($.limit_clause)
  ),

  projection: $ => choice(
    '*',
    seq($.expression, repeat(seq(',', $.expression))),
    seq($.expression, 'AS', $.alias)
  ),

  // Placeholder for select_modifier (defined in dialects that support it, e.g., MySQL)
  select_modifier: $ => seq(),

  set_quantifier: $ => choice('DISTINCT', 'ALL'),

  from_clause: $ => seq(
    'FROM',
    $.table_reference,
    repeat(seq(',', $.table_reference))
  ),

  // =============================================================================
  // INSERT Statement
  // =============================================================================

  insert_statement: $ => seq(
    'INSERT',
    'INTO',
    $.table_name,
    optional($.column_list),
    'VALUES',
    $.value_list,
    repeat(seq(',', $.value_list)),
    optional($.returning_clause)  // PostgreSQL extension
  ),

  column_list: $ => seq(
    '(',
    $.column_name,
    repeat(seq(',', $.column_name)),
    ')'
  ),

  value_list: $ => seq(
    '(',
    $.expression,
    repeat(seq(',', $.expression)),
    ')'
  ),

  // =============================================================================
  // UPDATE Statement
  // =============================================================================

  update_statement: $ => seq(
    'UPDATE',
    $.table_name,
    'SET',
    $.assignment,
    repeat(seq(',', $.assignment)),
    optional($.where_clause),
    optional($.returning_clause)  // PostgreSQL extension
  ),

  assignment: $ => seq(
    $.column_name,
    '=',
    $.expression
  ),

  // =============================================================================
  // DELETE Statement
  // =============================================================================

  delete_statement: $ => seq(
    'DELETE',
    'FROM',
    $.table_name,
    optional($.where_clause),
    optional($.returning_clause)  // PostgreSQL extension
  ),

  // Placeholder for returning_clause (defined in dialects that support it)
  returning_clause: $ => seq(),

  // =============================================================================
  // Clauses
  // =============================================================================

  where_clause: $ => seq('WHERE', $.expression),

  group_by_clause: $ => seq(
    'GROUP',
    'BY',
    $.expression,
    repeat(seq(',', $.expression))
  ),

  having_clause: $ => seq('HAVING', $.expression),

  order_by_clause: $ => seq(
    'ORDER',
    'BY',
    $.order_by_element,
    repeat(seq(',', $.order_by_element))
  ),

  order_by_element: $ => seq(
    $.expression,
    optional(choice('ASC', 'DESC'))
  ),

  limit_clause: $ => seq('LIMIT', $.expression),

  offset_clause: $ => seq('OFFSET', $.expression),

  // =============================================================================
  // Joins
  // =============================================================================

  table_reference: $ => choice(
    $.table_name,
    $.table_name,
    seq($.table_name, optional($.alias)),
    seq($.table_name, 'AS', $.alias),
    $.join_clause
  ),

  join_clause: $ => seq(
    optional($.join_type),
    'JOIN',
    $.table_name,
    optional('AS'),
    optional($.alias),
    'ON',
    $.expression
  ),

  join_type: $ => choice(
    'INNER',
    seq('LEFT', optional('OUTER')),
    seq('RIGHT', optional('OUTER')),
    seq('FULL', optional('OUTER'))
  ),

  // =============================================================================
  // Common Table Expressions (CTE)
  // =============================================================================

  cte_clause: $ => seq(
    'WITH',
    $.cte_definition,
    repeat(seq(',', $.cte_definition))
  ),

  cte_definition: $ => seq(
    $.table_name,
    optional('AS'),
    '(',
    $.select_statement,
    ')'
  ),

  // =============================================================================
  // Expressions
  // =============================================================================

  expression: $ => choice(
    $.binary_expression,
    $.unary_expression,
    $.column_reference,
    $.literal,
    $.function_call,
    $.case_expression,
    '(', $.expression, ')',
    '*'
  ),

  binary_expression: $ => prec.left(1, seq(
    field('left', $.expression),
    field('operator', choice(
      '=', '!=', '<>', '<', '>', '<=', '>=',
      'AND', 'OR',
      '+', '-', '*', '/', '%'
    )),
    field('right', $.expression)
  )),

  unary_expression: $ => prec(2, seq(
    choice('-', '+', 'NOT'),
    $.expression
  )),

  column_reference: $ => choice(
    $.column_name,
    seq($.table_name, '.', $.column_name)
  ),

  function_call: $ => seq(
    $.function_name,
    '(',
    optional(seq($.expression, repeat(seq(',', $.expression)))),
    ')'
  ),

  case_expression: $ => seq(
    'CASE',
    repeat(seq('WHEN', $.expression, 'THEN', $.expression)),
    optional(seq('ELSE', $.expression)),
    'END'
  ),

  // =============================================================================
  // Literals
  // =============================================================================

  literal: $ => choice(
    $.string_literal,
    $.number_literal,
    $.boolean_literal,
    'NULL'
  ),

  string_literal: $ => /'([^']|'')*'/,

  number_literal: $ => /\d+(\.\d+)?/,

  boolean_literal: $ => choice('TRUE', 'FALSE'),

  // =============================================================================
  // Identifiers
  // =============================================================================

  table_name: $ => $.identifier,

  column_name: $ => $.identifier,

  function_name: $ => $.identifier,

  alias: $ => $.identifier,

  identifier: $ => choice(
    /[a-zA-Z_][a-zA-Z0-9_]*/,
    /`[^`]+`/,        // MySQL style
    /"[^"]+"/,        // PostgreSQL style
    /\[[^\]]+\]/      // SQL Server style
  ),
};
