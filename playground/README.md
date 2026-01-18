# Unified SQL LSP Playground

A web-based SQL editor demonstrating the Unified SQL LSP capabilities with real-time code completion, hover tooltips, and diagnostics.

## Features

- **Multi-dialect SQL Support**: MySQL, PostgreSQL, TiDB, MariaDB, CockroachDB
- **LSP Features**:
  - Code completion (Ctrl+Space)
  - Hover tooltips (hover over SQL elements)
  - Real-time diagnostics with error/warning panel
- **Schema Browser**: View available tables and columns
- **Example Queries**: Load pre-written SQL examples to test features
- **Dialect Switching**: Change SQL dialect on-the-fly

## Development

### Prerequisites

- Node.js 20+
- pnpm (recommended) or npm

### Getting Started

```bash
# Install dependencies
pnpm install

# Start development server
pnpm dev
```

The playground will open at `http://localhost:3001`

### Build

```bash
# Build for production
pnpm build
```

The built files will be in `dist/`

### Preview Production Build

```bash
# Preview the production build locally
pnpm preview
```

## Deployment

### Automatic Deployment (GitHub Pages)

The playground is automatically deployed to GitHub Pages when changes are pushed to the `main` branch. The workflow is triggered by:

- Push to `main` branch
- Changes to `playground/` directory
- Manual trigger via workflow_dispatch

Access the deployed playground at:
```
https://woxqaa.github.io/unified-sql-lsp/
```

### Manual Deployment

For manual deployment to GitHub Pages:

```bash
# Install gh-pages (if not already installed)
npm install -g gh-pages

# Deploy to GitHub Pages
pnpm deploy
```

### Custom Deployment

To deploy to a different location or with a custom base path:

1. Update `base` in `vite.config.ts`:
   ```ts
   export default defineConfig({
     base: '/your-custom-path/',
     // ... rest of config
   })
   ```

2. Build and deploy as needed.

## Architecture

```
playground/
├── src/
│   ├── components/      # React components
│   │   ├── SchemaBrowser.tsx
│   │   └── DiagnosticsPanel.tsx
│   ├── lib/             # Utilities
│   │   ├── wasm-interface.ts    # WASM loader
│   │   └── lsp-bridge.ts        # Monaco LSP adapter
│   ├── App.tsx          # Main application
│   └── main.tsx         # Entry point
├── wasm/                # Compiled WASM (generated)
├── index.html
├── vite.config.ts
└── package.json
```

## Tech Stack

- **Frontend**: React 18 + TypeScript + Vite
- **Editor**: Monaco Editor (VS Code's editor)
- **LSP Backend**: Rust compiled to WebAssembly
- **Deployment**: GitHub Pages

## Bundle Size

- Main app: **154 KB** (49 KB gzipped)
- Monaco Editor: **3.1 MB** (797 KB gzipped, cached separately)

## Future Enhancements

- [ ] Connect to real databases for live schema introspection
- [ ] Execute SQL queries against actual databases
- [ ] Add more example queries for different dialects
- [ ] Export queries to clipboard/file
- [ ] Dark/light theme toggle
- [ ] Query history
