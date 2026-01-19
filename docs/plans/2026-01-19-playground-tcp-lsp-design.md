# Playground TCP LSP Integration Design

**Date:** 2026-01-19
**Status:** Approved
**Author:** Claude (AI Assistant)
**Priority:** P0 - Replaces mock implementation with production LSP server

## Overview

Replace the playground's JavaScript mock LSP server with the actual Rust LSP server running as a TCP process. This eliminates the need for WASM compilation while providing full LSP functionality including Tree-sitter parsing, semantic analysis, table alias resolution, and multi-dialect support.

**Key Decision:** No WASM needed - run the existing LSP server as a local process with TCP transport.

### Problem Statement

Current playground implementation uses:
- JavaScript mock LSP server (`wasm-interface.ts`)
- Hardcoded completion items
- Simple string matching instead of parsing
- No table alias support
- Only SELECT statement completion

This fails to demonstrate the actual capabilities of the unified-sql-lsp server.

### Solution

Connect playground frontend to the real LSP server over TCP:
- **LSP Server:** Add TCP transport alongside existing stdio transport
- **Playground:** Remove all mock/WASM code, add lightweight WebSocket client
- **Startup:** Single script launches both services

**Benefits:**
- ✅ Zero code duplication - use exact same LSP core
- ✅ Full feature set immediately - all dialects, all SQL statement types
- ✅ No WASM size/complexity concerns
- ✅ Easy debugging - can run LSP server in terminal for logs
- ✅ Production-grade completions with real semantic analysis

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Start Script                             │
│                  make run-playground                         │
│  ┌──────────────────────┐  ┌──────────────────────┐        │
│  │  LSP Server (Rust)   │  │  Playground (Node)   │        │
│  │  unified-sql-lsp     │  │  pnpm run dev        │        │
│  │  --tcp 4137          │  │  localhost:5173      │        │
│  └──────────────────────┘  └──────────────────────┘        │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Browser                                   │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Monaco Editor                                        │  │
│  │  - SQL code editing                                   │  │
│  │  - Completion, hover, diagnostics                     │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  LSP Client (lsp-client.ts)                          │  │
│  │  - WebSocket to ws://localhost:4137                  │  │
│  │  - JSON-RPC protocol wrapper                         │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  LSP Server (Rust)                           │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  TCP Transport (new)                                 │  │
│  │  - TcpListener on port 4137                          │  │
│  │  - WebSocket via tokio-tungstenite                   │  │
│  │  - Multiple client support                           │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Existing LSP Core (unchanged)                       │  │
│  │  - Tree-sitter parsing                               │  │
│  │  - ScopeManager                                      │  │
│  │  - AliasResolver                                     │  │
│  │  - Completion, hover, diagnostics                    │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  StaticCatalog                                       │  │
│  │  - playground schema (users, orders, order_items)    │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### User Experience

```bash
make run-playground
# → Starts LSP server on port 4137
# → Starts playground dev server on port 5173
# → Opens browser to http://localhost:5173
# → Shows connection status: ✅ Connected to LSP server

# User edits SQL in Monaco editor
# → Real completions from actual LSP server
# → Full table alias resolution
# → Multi-dialect support (MySQL, PostgreSQL)
# → Semantic diagnostics

# User closes browser or Ctrl+C
# → Both processes terminate
```

## Components

### 1. LSP Server TCP Transport

**File:** `crates/lsp/src/transport.rs` (modified)

Add TCP transport alongside existing stdio transport:

```rust
use tokio_tungstenite::accept_hdr;
use tokio::net::TcpListener;

pub enum Transport {
    Stdio(StdioTransport),
    Tcp(TcpTransport),
}

pub struct TcpTransport {
    listener: TcpListener,
    clients: Vec<WebSocketConnection>,
}

impl TcpTransport {
    pub async fn listen(port: u16) -> Result<Self> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        Ok(Self {
            listener,
            clients: Vec::new(),
        })
    }

    pub async fn accept(&mut self) -> Result<ServerConnection> {
        let (stream, _) = self.listener.accept().await?;
        let ws_stream = accept_hdr(stream, |req, resp| {
            // WebSocket handshake
            Ok(resp)
        })?.await?;

        // Wrap in ServerConnection interface
        Ok(ServerConnection::WebSocket(ws_stream))
    }
}
```

**Dependencies to add:**
```toml
tokio-tungstenite = "0.21"
```

### 2. Playground LSP Client

**File:** `playground/src/lib/lsp-client.ts` (new)

Lightweight LSP protocol client over WebSocket:

```typescript
export interface CompletionItem {
  label: string;
  kind: number;
  detail?: string;
  documentation?: string;
  insertText?: string;
}

export class LspClient {
  private ws: WebSocket | null = null;
  private requestId = 0;
  private pendingRequests = new Map<number, {
    resolve: (value: any) => void;
    reject: (error: Error) => void;
  }>();

  async connect(url: string = 'ws://localhost:4137'): Promise<void> {
    this.ws = new WebSocket(url);

    return new Promise((resolve, reject) => {
      this.ws!.onopen = () => resolve();
      this.ws!.onerror = (e) => reject(e);
      this.ws!.onmessage = (e) => this.handleMessage(e.data);
    });
  }

  async completion(
    text: string,
    line: number,
    col: number
  ): Promise<CompletionItem[]> {
    return this.sendRequest('textDocument/completion', {
      textDocument: { uri: 'file:///playground.sql' },
      position: { line, character: col },
      context: { triggerKind: 1 }
    });
  }

  async hover(
    text: string,
    line: number,
    col: number
  ): Promise<{ contents: { kind: string; value: string } }> {
    return this.sendRequest('textDocument/hover', {
      textDocument: { uri: 'file:///playground.sql' },
      position: { line, character: col }
    });
  }

  async diagnostics(text: string): Promise<any[]> {
    return this.sendRequest('textDocument/diagnostic', {
      textDocument: { uri: 'file:///playground.sql' },
      content: text
    });
  }

  private sendRequest<T>(method: string, params: any): Promise<T> {
    return new Promise((resolve, reject) => {
      const id = ++this.requestId;
      this.pendingRequests.set(id, { resolve, reject });

      const message = JSON.stringify({
        jsonrpc: '2.0',
        id,
        method,
        params
      });

      this.ws?.send(message);
    });
  }

  private handleMessage(data: string) {
    const response = JSON.parse(data);

    if (response.id) {
      const pending = this.pendingRequests.get(response.id);
      if (pending) {
        if (response.error) {
          pending.reject(new Error(response.error.message));
        } else {
          pending.resolve(response.result);
        }
        this.pendingRequests.delete(response.id);
      }
    }
  }
}
```

### 3. Monaco Editor Integration

**File:** `playground/src/lib/monaco-setup.ts` (new)

```typescript
import * as monaco from 'monaco-editor';
import { LspClient, CompletionItem } from './lsp-client';

export function setupSqlEditor(
  container: HTMLElement,
  lspClient: LspClient
): monaco.editor.IStandaloneCodeEditor {
  // Register SQL language
  monaco.languages.register({ id: 'sql' });

  // Completion provider
  monaco.languages.registerCompletionItemProvider('sql', {
    triggerCharacters: ['.', ' '],

    async provideCompletionItems(model, position) {
      const text = model.getValue();
      const items = await lspClient.completion(
        text,
        position.lineNumber - 1,
        position.column - 1
      );

      return {
        suggestions: items.map((item: CompletionItem) => ({
          label: item.label,
          kind: item.kind,
          detail: item.detail,
          documentation: item.documentation,
          insertText: item.insertText || item.label
        }))
      };
    }
  });

  // Hover provider
  monaco.languages.registerHoverProvider('sql', {
    async provideHover(model, position) {
      const text = model.getValue();
      const hover = await lspClient.hover(
        text,
        position.lineNumber - 1,
        position.column - 1
      );

      return {
        contents: [
          { value: hover.contents.value }
        ]
      };
    }
  });

  // Create editor
  const editor = monaco.editor.create(container, {
    value: 'SELECT \nFROM users\nWHERE id = 1;',
    language: 'sql',
    theme: 'vs-dark',
    automaticLayout: true,
    minimap: { enabled: false }
  });

  return editor;
}
```

### 4. Static Catalog Schema

**File:** `playground/fixtures/schema.sql` (new)

```sql
-- Playground test schema

CREATE TABLE users (
  id INT PRIMARY KEY AUTO_INCREMENT,
  name VARCHAR(100) NOT NULL,
  email VARCHAR(255) UNIQUE NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE orders (
  id INT PRIMARY KEY AUTO_INCREMENT,
  user_id INT NOT NULL,
  total DECIMAL(10, 2) NOT NULL,
  status VARCHAR(20) DEFAULT 'pending',
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE order_items (
  id INT PRIMARY KEY AUTO_INCREMENT,
  order_id INT NOT NULL,
  product_name VARCHAR(255) NOT NULL,
  quantity INT NOT NULL,
  price DECIMAL(10, 2) NOT NULL,
  FOREIGN KEY (order_id) REFERENCES orders(id)
);

-- Sample data for testing
INSERT INTO users (name, email) VALUES
  ('Alice', 'alice@example.com'),
  ('Bob', 'bob@example.com'),
  ('Charlie', 'charlie@example.com');
```

**File:** `crates/lsp/src/main.rs` (modified)

Add `--catalog` CLI flag:

```rust
#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    dialect: Option<String>,

    #[arg(long)]
    tcp: Option<u16>,

    #[arg(long)]
    catalog: Option<String>,  // New
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load catalog based on flag
    let catalog = if let Some(catalog_name) = cli.catalog.as_ref() {
        match catalog_name.as_str() {
            "playground" => StaticCatalog::from_file("playground/fixtures/schema.sql")?,
            _ => StaticCatalog::in_memory(),
        }
    } else {
        StaticCatalog::in_memory()
    };

    // Start transport
    let transport = if let Some(port) = cli.tcp {
        Transport::Tcp(TcpTransport::listen(port).await?)
    } else {
        Transport::Stdio(StdioTransport::new())
    };

    // Run server
    run_server(transport, catalog).await?;

    Ok(())
}
```

### 5. Start Script

**File:** `playground/start.sh` (new)

```bash
#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}Starting Unified SQL LSP Playground...${NC}"

# Start LSP server in background
echo -e "${GREEN}→ Starting LSP server on TCP port 4137${NC}"
cargo run --bin unified-sql-lsp -- --tcp 4137 --catalog playground &
LSP_PID=$!

# Wait for LSP server to be ready
sleep 2

# Check if LSP server is running
if ! kill -0 $LSP_PID 2>/dev/null; then
    echo -e "${RED}✗ Failed to start LSP server${NC}"
    exit 1
fi

echo -e "${GREEN}✓ LSP server started (PID: $LSP_PID)${NC}"

# Cleanup function
cleanup() {
    echo -e "\n${GREEN}Shutting down...${NC}"
    kill $LSP_PID 2>/dev/null || true
    wait $LSP_PID 2>/dev/null || true
    echo -e "${GREEN}✓ Cleanup complete${NC}"
}

# Trap EXIT and INT signals
trap cleanup EXIT INT

# Start playground dev server
echo -e "${GREEN}→ Starting playground dev server${NC}"
cd "$(dirname "$0")"
pnpm run dev
```

**File:** `Makefile` (modified)

Add playground target:

```makefile
.PHONY: run-playground
run-playground:
	@chmod +x playground/start.sh
	@playground/start.sh
```

## Data Flow

### Startup Flow

```
User executes: make run-playground
    ↓
start.sh runs cargo run --bin unified-sql-lsp -- --tcp 4137 --catalog playground
    ↓
LSP server starts TcpListener on port 4137
    ↓
LSP server loads StaticCatalog from playground/fixtures/schema.sql
    ↓
start.sh runs pnpm run dev
    ↓
Vite dev server starts on http://localhost:5173
    ↓
Browser opens to playground
    ↓
playground's main.ts initializes LspClient
    ↓
LspClient.connect('ws://localhost:4137') establishes WebSocket connection
    ↓
Connection status shows: ✅ Connected
```

### Completion Request Flow

```
User types "SEL" in Monaco editor
    ↓
Monaco triggers provideCompletionItems(model, position)
    ↓
lsp-client.ts calls completion(model.getValue(), 0, 3)
    ↓
LspClient sends WebSocket message:
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "textDocument/completion",
  "params": {
    "textDocument": { "uri": "file:///playground.sql" },
    "position": { "line": 0, "character": 3 }
  }
}
    ↓
TcpTransport in LSP server receives message
    ↓
LSP server processes request using existing completion handler:
    ↓
    crates/lsp/src/completion.rs
        ↓
        Tree-sitter parses SQL text
        ↓
        CompletionContext detects position
        ↓
        ScopeManager provides visible tables
        ↓
        AliasResolver resolves table aliases
        ↓
        Catalog provides column information
        ↓
        Completion items generated
    ↓
LSP server sends WebSocket response:
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": [
    { "label": "SELECT", "kind": 14, "detail": "Keyword", ... },
    { "label": "users", "kind": 5, "detail": "Table", ... }
  ]
}
    ↓
LspClient receives response, resolves Promise
    ↓
Monaco integration converts to ISuggestion[] and displays
```

**Key Point:** All LSP core logic remains in `crates/lsp/` - unchanged. The playground is just a thin client.

### Error Handling Flow

```
LspClient.connect() fails
    ↓
Show error banner: "⚠ Cannot connect to LSP server"
    ↓
Disable completion/hover features
    ↓
Offer "Start LSP Server" button (runs make run-playground in new terminal)

Connection drops during editing
    ↓
LspClient detects WebSocket close event
    ↓
Attempt auto-reconnect (3 retries with 1s delay)
    ↓
If all fail, show disconnected status
```

## Implementation Plan

### Phase 1: TCP Transport (1-2 days)

**Tasks:**
1. Add `tokio-tungstenite` dependency to `crates/lsp/Cargo.toml`
2. Create `TcpTransport` struct in `crates/lsp/src/transport.rs`
3. Implement WebSocket message handling
4. Modify `crates/lsp/src/main.rs` to accept `--tcp` flag
5. Test with simple WebSocket client

**Acceptance Criteria:**
- LSP server starts with `--tcp 4137`
- Can connect via WebSocket client
- JSON-RPC messages work over WebSocket
- Multiple concurrent connections supported

**Testing:**
```bash
# Terminal 1
cargo run --bin unified-sql-lsp -- --tcp 4137

# Terminal 2 (using websocat or similar)
websocat ws://localhost:4137
# Send: {"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
# Should receive response
```

### Phase 2: Catalog Setup (0.5 day)

**Tasks:**
1. Create `playground/fixtures/schema.sql`
2. Add `--catalog` flag to `crates/lsp/src/main.rs`
3. Implement StaticCatalog loading from file
4. Verify catalog loads correctly

**Acceptance Criteria:**
- `--catalog playground` loads test schema
- LSP server can query tables from catalog
- Completion returns items from test schema

**Testing:**
```bash
cargo run --bin unified-sql-lsp -- --tcp 4137 --catalog playground
# Connect with WebSocket client
# Request completion for "SELECT * FROM "
# Should see: users, orders, order_items
```

### Phase 3: Playground Client (1 day)

**Tasks:**
1. Delete entire `lsp-wasm/` crate
2. Delete `playground/src/lib/wasm-interface.ts`
3. Create `playground/src/lib/lsp-client.ts`
4. Create `playground/src/lib/monaco-setup.ts`
5. Update `playground/src/main.ts`
6. Test integration

**Acceptance Criteria:**
- Monaco editor loads in browser
- Completion works with real LSP server
- Hover works
- Diagnostics work

**Testing:**
1. Start LSP server: `cargo run --bin unified-sql-lsp -- --tcp 4137 --catalog playground`
2. Start playground: `pnpm run dev`
3. Open browser to `http://localhost:5173`
4. Type `SELECT u.` → should see users table columns
5. Hover over table name → should see table info
6. Type invalid SQL → should see diagnostics

### Phase 4: Start Script & Polish (0.5 day)

**Tasks:**
1. Create `playground/start.sh`
2. Add `run-playground` target to Makefile
3. Add connection status UI component
4. Add error handling UI
5. Update README with usage instructions

**Acceptance Criteria:**
- `make run-playground` starts both services
- Connection status visible in UI
- Graceful shutdown on Ctrl+C
- Clean process management

### Phase 5: Testing (1 day)

**Tasks:**
1. Test all scenarios from PLAYGROUND_HANDOFF.md
2. Verify table alias resolution
3. Test all dialects (MySQL, PostgreSQL)
4. Test error scenarios (server not running, connection drops)
5. Performance check (no lag in completions)

**Test Cases:**

```sql
-- Scenario 1: Table alias completion
SELECT u.| FROM users u;
-- Expected: id, name, email, created_at

-- Scenario 2: JOIN with aliases
SELECT u.n, o.t| FROM users u JOIN orders o ON u.id = o.user_id
-- Expected: u.name, o.total

-- Scenario 3: Context-aware keywords
SEL|
-- Expected: SELECT

SELECT * FROM users W|
-- Expected: WHERE

-- Scenario 4: INSERT statement
INSERT INTO users (|)
-- Expected: id, name, email, created_at

-- Scenario 5: Error detection
SELEC * FROM users;
-- Expected: Diagnostic "Did you mean SELECT?"
```

**Dialect Testing:**
```bash
# MySQL
cargo run --bin unified-sql-lsp -- --tcp 4137 --dialect mysql

# PostgreSQL
cargo run --bin unified-sql-lsp -- --tcp 4137 --dialect postgresql
```

**Total Estimate:** 4-5 days

## File Changes Summary

### Files to Create

```
playground/fixtures/schema.sql
playground/src/lib/lsp-client.ts
playground/src/lib/monaco-setup.ts
playground/src/lib/connection-status.ts
playground/start.sh
docs/plans/2026-01-19-playground-tcp-lsp-design.md
```

### Files to Modify

```
crates/lsp/Cargo.toml              # Add tokio-tungstenite
crates/lsp/src/transport.rs        # Add TcpTransport
crates/lsp/src/main.rs             # Add --tcp and --catalog flags
playground/src/main.ts             # Use new LspClient
Makefile                           # Add run-playground target
playground/README.md               # Update usage instructions
```

### Files to Delete

```
lsp-wasm/                          # Entire crate
playground/src/lib/wasm-interface.ts
```

## Rollout Plan

1. **Development:** Implement in feature branch
2. **Testing:** Manual testing against scenarios in PLAYGROUND_HANDOFF.md
3. **Documentation:** Update playground README with new usage
4. **Merge:** Merge to main once all scenarios pass
5. **Cleanup:** Delete PLAYGROUND_HANDOFF.md (resolved)

## Success Criteria

All issues from PLAYGROUND_HANDOFF.md resolved:

- ✅ Support INSERT, UPDATE, DELETE, DDL statements (not just SELECT)
- ✅ Table alias resolution (u.id, o.total)
- ✅ Real parsing with Tree-sitter (not string matching)
- ✅ Position-aware completions
- ✅ Error recovery (partial success mode)
- ✅ Semantic analysis (type information, validation)
- ✅ Multi-dialect support
- ✅ Full LSP capabilities (hover, diagnostics, etc.)

## Alternatives Considered

### Alternative 1: WASM Compilation (Rejected)
**Why:** Too complex, large bundle size, async dependencies incompatible

### Alternative 2: Remote LSP Server (Rejected)
**Why:** Adds latency, requires server infrastructure, not better than local

### Alternative 3: Node.js Bridge (Rejected)
**Why:** More complex than direct TCP, requires native module compilation

## Open Questions

None - design is complete and approved.

## References

- PLAYGROUND_HANDOFF.md - Original problem analysis
- CLAUDE.md - Project architecture and constraints
- crates/lsp/ - Existing LSP server implementation
- LSP Specification: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
