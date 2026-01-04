# Unified SQL Grammar

Multi-dialect SQL grammar for the unified-sql-lsp project using tree-sitter.

## Overview

This crate provides tree-sitter grammar support for multiple SQL dialects with a unified interface. The grammars are compiled at build time using tree-sitter CLI and linked into the Rust library.

## Supported Dialects

- **base** - Core SQL grammar (common subset across all dialects)
- **mysql** - MySQL-specific extensions (REPLACE, LIMIT offset,count, backtick identifiers)
- **postgresql** - PostgreSQL-specific extensions (RETURNING, DISTINCT ON, dollar-quoted strings)

## Architecture

The grammar uses a **compile-time dialect merging** strategy:

```
┌─────────────────────────────────────────────────────────┐
│                   grammar.js                            │
│  - Reads DIALECT environment variable                   │
│  - Loads base SQL rules (inline)                        │
│  - Merges dialect-specific extensions from dialect/     │
│  - Exports unified grammar via grammar() function       │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
        ┌───────────────────┴───────────────────┐
        │             tree-sitter generate       │
        │   (DIALECT=mysql/postgresql/base)      │
        └───────────────────┬───────────────────┘
                            │
                            ▼
                   ┌────────┴────────┐
                   │   parser.c      │
                   │  (C code)       │
                   └────────┬────────┘
                            │
                            ▼
              ┌─────────────┴─────────────┐
              │      cc crate (Rust)      │
              │   compiles to .o files    │
              └─────────────┬─────────────┘
                            │
                            ▼
              ┌─────────────┴─────────────┐
              │   parser-mysql.o          │
              │   parser-postgresql.o     │
              │   parser-base.o           │
              └───────────────────────────┘
```

## Building

### Prerequisites

Install tree-sitter CLI (version 0.26+ recommended):
```bash
npm install -g tree-sitter-cli
```

### Automatic Build (Recommended)

The grammar is automatically built when running `cargo build`:

```bash
# Build the entire project (includes grammar compilation)
cargo build

# Build only the grammar crate
cargo build -p unified-sql-grammar
```

The `build.rs` script will:
1. Generate parser.c for each dialect using tree-sitter
2. Rename each to `parser-{dialect}.c` to avoid overwriting
3. Compile each parser.c to a dialect-specific object file
4. Link the object files into the final library

**Note**: Each dialect generates an independent parser file in `gen/`:
- `gen/parser-base.c` - Base SQL dialect
- `gen/parser-mysql.c` - MySQL dialect
- `gen/parser-postgresql.c` - PostgreSQL dialect

### Manual Build

For development and testing:

```bash
# Build all dialects using the build script
./build.sh

# Or using npm scripts
npm run build

# Build a specific dialect
npm run build:base
npm run build:mysql
npm run build:postgresql
```

Manual generation:
```bash
cd src/grammar
DIALECT=mysql tree-sitter generate
```

## Testing

```bash
# Run Rust tests
cargo test -p unified-sql-grammar

# Run tree-sitter corpus tests
npm test

# Test specific dialect
npm run test:mysql
```

## Directory Structure

```
crates/grammar/
├── src/
│   ├── grammar/               # Tree-sitter grammar files
│   │   ├── grammar.js         # Main grammar entry point
│   │   ├── dialect/           # Dialect-specific extensions
│   │   │   ├── mysql.js       # MySQL extensions
│   │   │   └── postgresql.js  # PostgreSQL extensions
│   │   └── gen/               # Generated parser files (gitignored)
│   │       ├── parser-base.c       # Base SQL parser
│   │       ├── parser-mysql.c      # MySQL parser
│   │       ├── parser-postgresql.c # PostgreSQL parser
│   │       ├── grammar.json        # Grammar metadata
│   │       ├── node-types.json     # Node type definitions
│   │       └── tree_sitter/        # Tree-sitter runtime headers
│   └── lib.rs                 # Rust library interface
├── test/
│   └── corpus/                # Tree-sitter test cases
│       └── base.txt           # Base SQL tests
├── examples/
│   └── parse_sql.rs           # Example usage
├── build.rs                   # Cargo build script
├── build.sh                   # Standalone build script
├── package.json               # npm scripts for manual building
└── Cargo.toml                 # Rust crate manifest
```

## Usage

### Getting a Parser for a Dialect

```rust
use unified_sql_grammar::language_for_dialect;
use unified_sql_lsp_ir::Dialect;

// Get the tree-sitter Language for MySQL
if let Some(lang) = language_for_dialect(Dialect::MySQL) {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(lang).unwrap();

    let sql = "SELECT * FROM users LIMIT 10";
    let tree = parser.parse(sql, None).unwrap();

    // Process the parse tree...
}
```

### Dialect Mapping

| IR Dialect | Grammar Used | Notes |
|------------|--------------|-------|
| `Dialect::MySQL` | MySQL parser | Native MySQL syntax |
| `Dialect::TiDB` | MySQL parser | TiDB is MySQL-compatible |
| `Dialect::MariaDB` | MySQL parser | MariaDB is MySQL-compatible |
| `Dialect::PostgreSQL` | PostgreSQL parser | Native PostgreSQL syntax |
| `Dialect::CockroachDB` | PostgreSQL parser | CockroachDB is PostgreSQL-compatible |

## Adding a New Dialect

1. **Create dialect file**: `src/grammar/dialect/<dialect>.js`
   ```javascript
   module.exports = {
     // Dialect-specific rules or overrides
     statement: $ => $.new_statement,
     // ...
   };
   ```

2. **Update build.rs**: Add to the dialects list
   ```rust
   let dialects = vec!["base", "mysql", "postgresql", "new_dialect"];
   ```

3. **Update build.sh**: Add to DIALECTS array
   ```bash
   DIALECTS=("base" "mysql" "postgresql" "new_dialect")
   ```

4. **Update package.json**: Add build/test scripts
   ```json
   "build:new_dialect": "cd src/grammar && DIALECT=new_dialect tree-sitter generate"
   ```

5. **Add test cases**: Create `test/corpus/<dialect>.txt`

6. **Update lib.rs**: Add dialect mapping in `language_for_dialect()`

## Grammar Rules

The base grammar supports:

- **Statements**: SELECT, INSERT, UPDATE, DELETE
- **Clauses**: WHERE, ORDER BY, GROUP BY, HAVING, LIMIT
- **Joins**: INNER, LEFT, RIGHT, FULL OUTER
- **Expressions**: Binary operators, functions, CASE expressions
- **Literals**: Strings, numbers, booleans, NULL
- **Identifiers**: Regular, MySQL-style (backtick), PostgreSQL-style (quotes)
- **Comments**: `--`, `#`, `/* */`

Dialect-specific extensions add additional syntax unique to each database.

## Development

### Regenerating Parsers

When modifying grammar files:

```bash
# Clean old generated files
rm -rf src/grammar/gen

# Regenerate for all dialects
./build.sh

# Or rebuild via Cargo (automatically regenerates)
cargo build -p unified-sql-grammar
```

### Tree-sitter Configuration

The grammar uses tree-sitter ABI version 15. To configure:

```bash
cd src/grammar
tree-sitter init  # Creates tree-sitter.json for ABI 15
```

## License

MIT OR Apache-2.0
