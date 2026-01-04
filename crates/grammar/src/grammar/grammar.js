/**
 * Unified SQL Grammar
 *
 * A multi-dialect SQL grammar supporting MySQL, PostgreSQL, and other SQL dialects.
 *
 * Dialect selection is done at compile time via the DIALECT environment variable:
 *   DIALECT=mysql tree-sitter generate
 *   DIALECT=postgresql tree-sitter generate
 *
 * Each dialect extends the base SQL grammar with dialect-specific features.
 */

const fs = require('fs');
const path = require('path');

// Get the dialect from environment variable (default to 'base')
const DIALECT = process.env.DIALECT || 'base';

// Load the dialect-specific grammar extensions (if any)
let dialectGrammar = {};
if (DIALECT !== 'base') {
  const dialectPath = path.join(__dirname, 'dialect', `${DIALECT}.js`);
  if (!fs.existsSync(dialectPath)) {
    console.error(`Error: Dialect '${DIALECT}' not found at ${dialectPath}`);
    console.error('Available dialects must be defined in the dialect/ directory');
    process.exit(1);
  }
  dialectGrammar = require(dialectPath);
}

// Export the grammar using tree-sitter's grammar() function
module.exports = grammar({
  name: DIALECT === 'base' ? 'unified_sql' : `unified_sql_${DIALECT}`,

  // =============================================================================
  // Extras - Comments and whitespace
  // =============================================================================

  extras: $ => [
    $.comment,
    /\s/,  // whitespace
  ],

  // =============================================================================
  // Inline rules - Rules that don't produce nodes
  // =============================================================================

  inline: $ => [
    $._statement,
  ],

  // =============================================================================
  // Conflicts - Grammar ambiguities that tree-sitter should tolerate
  // =============================================================================

  conflicts: $ => [
    [$.projection, $.expression],
  ],

  // =============================================================================
  // Supertypes - Categories of node types
  // =============================================================================

  supertypes: $ => [
    $._statement,
    $.expression,
  ],

  // =============================================================================
  // Rules
  // =============================================================================

  rules: {
    // Source file (root rule)
    source_file: $ => repeat($._statement),

    _statement: $ => $.statement,

    // =============================================================================
    // Comments
    // =============================================================================

    comment: $ => choice(
      seq('--', /.*/),
      seq('#', /.*/),
      seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/')
    ),

    // =============================================================================
    // Core SQL Statements
    // =============================================================================

    statement: $ => choice(
      $.select_statement,
      $.insert_statement,
      $.update_statement,
      $.delete_statement
    ),

    select_statement: $ => seq(
      optional($.cte_clause),
      'SELECT',
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
      seq($.expression, repeat(seq(',', $.expression)))
    ),

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
      repeat(seq(',', $.value_list))
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
      optional($.where_clause)
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
      optional($.where_clause)
    ),

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

    _expression: $ => choice(
      $.binary_expression,
      $.unary_expression,
      $.column_reference,
      $.literal,
      $.function_call,
      $.case_expression,
      $.parenthesized_expression,
      '*'
    ),

    expression: $ => choice(
      $.binary_expression,
      $.unary_expression,
      $.column_reference,
      $.literal,
      $.function_call,
      $.case_expression,
      $.parenthesized_expression,
      '*'
    ),

    parenthesized_expression: $ => seq('(', $.expression, ')'),

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

    // Include dialect-specific rules (if any)
    ...dialectGrammar,
  },
});
