/**
 * PostgreSQL 12 Dialect
 *
 * PostgreSQL 12 is the BASE dialect for the PostgreSQL family.
 * This file contains all PostgreSQL-specific features up to version 12.
 * PostgreSQL 14+ extends this dialect with additional features.
 *
 * Features in this dialect:
 * - RETURNING clause
 * - DEFAULT VALUES
 * - Dollar-quoted string literals
 * - Double-quote identifiers
 *
 * Source: PostgreSQL 12 Documentation
 * https://www.postgresql.org/docs/release/12.0/
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
