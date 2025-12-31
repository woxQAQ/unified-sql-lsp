# Unified SQL Grammar

Multi-dialect SQL grammar for the unified-sql-lsp project.

## Overview

This crate provides tree-sitter grammar support for multiple SQL dialects with a unified interface.

## Supported Dialects

- **base** - Core SQL grammar (common subset across all dialects)
- **mysql** - MySQL-specific extensions
- **postgresql** - PostgreSQL-specific extensions

## Building

### Prerequisites

Install tree-sitter CLI:
```bash
npm install -g tree-sitter-cli
```

### Build all dialects

```bash
# Using the build script
./build.sh

# Or using npm
npm run build
```

### Build a specific dialect

```bash
npm run build:base
npm run build:mysql
npm run build:postgresql
```

Or manually:
```bash
cd src/grammar
DIALECT=mysql tree-sitter generate --no-bindings
```

## Testing

```bash
# Test all dialects
npm test

# Test specific dialect
npm run test:mysql
```

## Directory Structure

```
crates/grammar/
├── src/
│   ├── grammar/           # Tree-sitter grammar files
│   │   ├── grammar.js     # Main grammar with dialect merging
│   │   ├── dialect/       # Dialect-specific extensions
│   │   │   ├── base.js
│   │   │   ├── mysql.js
│   │   │   └── postgresql.js
│   │   └── src/           # Generated parser files
│   └── lib.rs            # Rust library
├── test/
│   └── corpus/           # Test cases
├── build.rs              # Cargo build script
├── build.sh              # Standalone build script
└── Cargo.toml
```

## Adding a New Dialect

1. Create `src/grammar/dialect/<dialect>.js` with dialect-specific rules
2. Add the dialect to `build.rs` and `build.sh`
3. Add test cases in `test/corpus/<dialect>.txt`
4. Update this README with the new dialect

## Grammar Architecture

The grammar uses a **dialect inheritance** pattern:

1. **Base dialect** (`dialect/base.js`): Defines common SQL rules
2. **Specific dialects**: Override/extend base rules
3. **Main grammar** (`grammar.js`): Merges base + dialect rules at compile time

This allows:
- Code reuse across dialects
- Consistent node naming
- Easy addition of new dialects

## License

MIT OR Apache-2.0
