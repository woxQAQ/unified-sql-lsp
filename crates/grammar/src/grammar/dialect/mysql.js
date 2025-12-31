/**
 * MySQL Dialect
 *
 * MySQL-specific grammar extensions.
 * This file ONLY contains MySQL-specific additions/overrides.
 * Base SQL rules are inherited from dialect/base.js
 */

module.exports = {
  // MySQL-specific additional statement (REPLACE is added to base statements)
  statement: $ => choice(
    $.replace_statement  // MySQL-specific REPLACE statement
  ),

  // REPLACE statement (MySQL-specific)
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

  // MySQL SELECT modifiers (extends base select_statement)
  select_modifier: $ => choice(
    'SQL_CALC_FOUND_ROWS',
    seq('SQL_CACHE', optional('SQL_CALC_FOUND_ROWS')),
    seq('SQL_NO_CACHE', optional('SQL_CALC_FOUND_ROWS'))
  ),

  // MySQL-specific LIMIT syntax: LIMIT count OFFSET offset or LIMIT offset, count
  limit_clause: $ => choice(
    seq('LIMIT', $.expression),
    seq('LIMIT', $.expression, 'OFFSET', $.expression),
    seq('LIMIT', $.expression, ',', $.expression)
  ),

  // MySQL-style identifiers (backtick notation)
  identifier: $ => choice(
    /[a-zA-Z_][a-zA-Z0-9_]*/,
    /`[^`]+`/
  ),
};
