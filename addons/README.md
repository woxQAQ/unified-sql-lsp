# Database Engine Add-ons

This directory contains Wasm-based add-ons for different SQL database engines.

Each add-on is a self-contained module that implements:
- Engine-specific SQL parser
- Completion logic
- Schema introspection

## Structure

```
addons/
├── postgresql/          # PostgreSQL add-on
│   ├── manifest.yaml   # Add-on metadata
│   ├── parser.go       # Parser implementation
│   ├── completion.go   # Completion logic
│   └── grammar.js      # Tree-sitter grammar
└── mysql/              # MySQL add-on
    └── ...
```

## Building Add-ons

See `docs/技术设计文档.md` Section 3 for add-on development guide.
