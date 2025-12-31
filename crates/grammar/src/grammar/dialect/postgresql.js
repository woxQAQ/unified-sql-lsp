/**
 * PostgreSQL Dialect
 *
 * PostgreSQL-specific grammar extensions.
 * This file ONLY contains PostgreSQL-specific additions/overrides.
 * Base SQL rules are inherited from dialect/base.js
 */

module.exports = {
  // PostgreSQL-specific features

  // RETURNING clause (PostgreSQL-specific, can be added to INSERT/UPDATE/DELETE)
  returning_clause: $ => seq(
    'RETURNING',
    $.expression,
    repeat(seq(',', $.expression))
  ),

  // DEFAULT VALUES (PostgreSQL-specific INSERT syntax)
  default_values: $ => 'DEFAULT VALUES',

  // Dollar-quoted string literals (PostgreSQL-specific)
  // Format: $$string$$ or $tag$string$tag$
  string_literal: $ => choice(
    /'([^']|'')*'/,
    seq(/\$[A-Za-z0-9_]*\$/, /.*/, /\$[A-Za-z0-9_]*\$/)
  ),

  // PostgreSQL-style identifiers (double-quote notation)
  identifier: $ => choice(
    /[a-zA-Z_][a-zA-Z0-9_]*/,
    /"[^"]+"/
  ),
};
