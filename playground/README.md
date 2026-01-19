# Unified SQL LSP Playground

A web-based SQL editor demonstrating the Unified SQL LSP capabilities with real-time code completion, hover tooltips, and diagnostics.

## Features

- **Real LSP Server Integration**: Connects to actual Rust LSP server via WebSocket
- **Multi-dialect SQL Support**: MySQL, PostgreSQL, TiDB, MariaDB, CockroachDB
- **LSP Features**:
  - Code completion (Ctrl+Space)
  - Hover tooltips (hover over SQL elements)
  - Real-time diagnostics with error/warning panel
- **Schema Browser**: View available tables and columns
- **Example Queries**: Load pre-written SQL examples to test features
- **Dialect Switching**: Change SQL dialect on-the-fly

## Quick Start

### Prerequisites

- Rust 2021+ (for building the LSP server)
- Node.js 20+
- pnpm (recommended) or npm

### Running the Playground

The easiest way to start the playground is using the provided start script:

```bash
# From the project root
make run-playground
```

This will:
1. Start the LSP server on TCP port 4137
2. Start the playground web UI on http://localhost:5173
3. Open your browser automatically

Press `Ctrl+C` to stop both servers.

### Manual Setup

If you prefer to start the servers manually:

```bash
# Terminal 1: Start LSP server
cargo run --bin unified-sql-lsp -- --tcp 4137 --catalog playground

# Terminal 2: Start web UI
cd playground
pnpm install  # First time only
pnpm dev
```

The playground will open at `http://localhost:5173`

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
┌─────────────────────────────────────────────────┐
│                  Browser                        │
│  ┌───────────────────────────────────────────┐ │
│  │  Monaco Editor (React + TypeScript)      │ │
│  │  - SQL editing with syntax highlighting   │ │
│  │  - LSP integration (completion, hover)    │ │
│  │  - Real-time diagnostics                 │ │
│  └───────────────┬───────────────────────────┘ │
│                  │ WebSocket                   │
└──────────────────┼─────────────────────────────┘
                   │ ws://localhost:4137
┌──────────────────┼─────────────────────────────┐
│                  ↓                              │
│  ┌───────────────────────────────────────────┐ │
│  │  LSP Server (Rust)                        │ │
│  │  - TCP/WebSocket transport               │ │
│  │  - Tree-sitter parsing                    │ │
│  │  - Semantic analysis (ScopeManager, etc) │ │
│  │  - StaticCatalog (playground schema)      │ │
│  └───────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

playground/
├── src/
│   ├── components/      # React components
│   │   ├── SchemaBrowser.tsx
│   │   └── DiagnosticsPanel.tsx
│   ├── lib/             # Utilities
│   │   ├── lsp-client.ts         # WebSocket LSP client
│   │   └── monaco-setup.ts       # Monaco LSP integration
│   ├── App.tsx          # Main application
│   └── main.tsx         # Entry point
├── fixtures/
│   └── schema.sql       # Playground test schema
├── start.sh            # Start script (LSP + web UI)
├── index.html
├── vite.config.ts
└── package.json
```

## Tech Stack

- **Frontend**: React 18 + TypeScript + Vite
- **Editor**: Monaco Editor (VS Code's editor)
- **LSP Backend**: Rust with TCP/WebSocket transport
- **Deployment**: GitHub Pages (static files only)

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
