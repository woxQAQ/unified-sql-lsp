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

// Load the base grammar
const baseGrammar = require('./dialect/base.js');

// Load the dialect-specific grammar (if not base)
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

// Helper function to merge dialect-specific rules with base rules
function mergeGrammar(baseRules, dialectRules) {
  const merged = { ...baseRules };

  // Merge rules from dialect
  for (const key in dialectRules) {
    if (key === 'conflicts') {
      // Merge conflicts specially
      merged.conflicts = merged.conflicts || [];
      merged.conflicts.push(...dialectRules.conflicts);
    } else if (key === 'extras') {
      // Merge extras specially
      merged.extras = merged.extras || [];
      merged.extras.push(...dialectRules.extras);
    } else if (key === 'inline') {
      // Merge inline rules specially
      merged.inline = merged.inline || [];
      merged.inline.push(...dialectRules.inline);
    } else {
      // Override or add the rule
      merged[key] = dialectRules[key];
    }
  }

  return merged;
}

// Base SQL rules (from dialect/base.js plus tree-sitter specific rules)
const baseRules = {
  // =============================================================================
  // Module exports
  // =============================================================================

  extras: $ => [
    $.comment,
    /\s/,  // whitespace
  ],

  inline: $ => [
    $._statement,
  ],

  // =============================================================================
  // Rules
  // =============================================================================

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

  // Include all base SQL grammar rules from dialect/base.js
  ...baseGrammar,
};

// Merge base rules with dialect-specific rules
const finalRules = mergeGrammar(baseRules, dialectGrammar);

// Export the grammar
module.exports = {
  name: DIALECT === 'base' ? 'unified_sql' : `unified_sql_${DIALECT}`,
  rules: finalRules,

  // Word tokens (for better tokenization)
  word: $ => $.identifier,

  // Conflicts (can be extended by dialects)
  conflicts: $ => [
    [$.expression, $.column_reference],
  ],

  // Supertype (optional, for better categorization)
  supertypes: $ => [
    $._statement,
    $.expression,
  ],
};

// Export dialect metadata
module.exports.dialect = DIALECT;
