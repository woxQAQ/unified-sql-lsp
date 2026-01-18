# SQL LSP Playground Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a web-based playground that demonstrates the SQL LSP's completions, hover, and diagnostics capabilities by running the LSP server in WebAssembly.

**Architecture:** Frontend uses Monaco Editor + React + TypeScript. LSP server compiled to WASM via wasm-bindgen. LSP bridge layer handles protocol conversion between Monaco and WASM.

**Tech Stack:** React 18, TypeScript, Vite, Monaco Editor, wasm-bindgen, wasm-pack, Rust (WASM target)

---

## Phase 1: Foundation

### Task 1: Create playground directory structure

**Files:**
- Create: `playground/package.json`
- Create: `playground/tsconfig.json`
- Create: `playground/vite.config.ts`
- Create: `playground/index.html`
- Create: `playground/src/main.tsx`
- Create: `playground/src/App.tsx`
- Create: `playground/src/index.css`

**Step 1: Create package.json**

```bash
cat > playground/package.json << 'EOF'
{
  "name": "unified-sql-lsp-playground",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "monaco-editor": "^0.45.0"
  },
  "devDependencies": {
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "@vitejs/plugin-react": "^4.2.0",
    "typescript": "^5.3.0",
    "vite": "^5.0.0"
  }
}
EOF
```

**Step 2: Create tsconfig.json**

```bash
cat > playground/tsconfig.json << 'EOF'
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
EOF
```

**Step 3: Create tsconfig.node.json**

```bash
cat > playground/tsconfig.node.json << 'EOF'
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true
  },
  "include": ["vite.config.ts"]
}
EOF
```

**Step 4: Create vite.config.ts**

```bash
cat > playground/vite.config.ts << 'EOF'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3000,
    open: true
  }
})
EOF
```

**Step 5: Create index.html**

```bash
cat > playground/index.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Unified SQL LSP Playground</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
EOF
```

**Step 6: Create src/main.tsx**

```bash
mkdir -p playground/src
cat > playground/src/main.tsx << 'EOF'
import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
EOF
```

**Step 7: Create src/App.tsx**

```bash
cat > playground/src/App.tsx << 'EOF'
export default function App() {
  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <header style={{ padding: '1rem', borderBottom: '1px solid #ccc' }}>
        <h1>Unified SQL LSP Playground</h1>
      </header>
      <main style={{ flex: 1, display: 'flex' }}>
        <div style={{ width: '250px', borderRight: '1px solid #ccc', padding: '1rem' }}>
          <h2>Schema Browser</h2>
          <p>Coming soon...</p>
        </div>
        <div style={{ flex: 1 }}>
          <div id="monaco-editor" style={{ height: '100%' }}></div>
        </div>
      </main>
    </div>
  )
}
EOF
```

**Step 8: Create src/index.css**

```bash
cat > playground/src/index.css << 'EOF'
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen',
    'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue',
    sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#root {
  height: 100vh;
  width: 100vw;
}
EOF
```

**Step 9: Install dependencies**

```bash
cd playground && npm install
```

Expected: npm installs packages successfully

**Step 10: Verify dev server starts**

```bash
npm run dev
```

Expected: Server starts on http://localhost:3000 and shows header with "Unified SQL LSP Playground"

**Step 11: Stop dev server and commit**

```bash
# Press Ctrl+C to stop server
git add playground/
git commit -m "feat(playground): add basic React + Vite setup"
```

---

### Task 2: Integrate Monaco Editor

**Files:**
- Modify: `playground/src/App.tsx`

**Step 1: Install Monaco Editor dependency**

```bash
cd playground && npm install monaco-editor
```

**Step 2: Replace App.tsx with Monaco integration**

```bash
cat > playground/src/App.tsx << 'EOF'
import { useEffect, useRef } from 'react'
import * as monaco from 'monaco-editor'

export default function App() {
  const editorRef = useRef<HTMLDivElement>(null)
  const editorInstanceRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)

  useEffect(() => {
    if (!editorRef.current) return

    // Initialize Monaco Editor
    editorInstanceRef.current = monaco.editor.create(editorRef.current, {
      value: 'SELECT * FROM users WHERE id = 1;',
      language: 'sql',
      theme: 'vs-dark',
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: 14,
      lineNumbers: 'on',
      scrollBeyondLastLine: false,
    })

    return () => {
      editorInstanceRef.current?.dispose()
    }
  }, [])

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <header style={{ padding: '1rem', borderBottom: '1px solid #30363d', background: '#0d1117' }}>
        <h1 style={{ color: '#c9d1d9', fontSize: '1.5rem' }}>Unified SQL LSP Playground</h1>
      </header>
      <main style={{ flex: 1, display: 'flex' }}>
        <aside style={{ width: '250px', borderRight: '1px solid #30363d', background: '#0d1117', padding: '1rem' }}>
          <h2 style={{ color: '#c9d1d9', fontSize: '1rem', marginBottom: '1rem' }}>Schema Browser</h2>
          <div style={{ color: '#8b949e', fontSize: '0.875rem' }}>
            <p>Coming soon...</p>
          </div>
        </aside>
        <div style={{ flex: 1 }}>
          <div ref={editorRef} style={{ height: '100%' }} />
        </div>
      </main>
    </div>
  )
}
EOF
```

**Step 3: Verify Monaco Editor renders**

```bash
npm run dev
```

Expected: Browser shows dark-themed Monaco Editor with SQL syntax highlighting

**Step 4: Stop dev server and commit**

```bash
# Press Ctrl+C to stop server
git add playground/src/App.tsx playground/package.json
git commit -m "feat(playground): integrate Monaco Editor for SQL editing"
```

---

## Phase 2: WASM Infrastructure

### Task 3: Add WASM target to workspace

**Files:**
- Modify: `crates/lsp/Cargo.toml`
- Modify: `crates/lsp/src/lib.rs` (create if doesn't exist)

**Step 1: Read existing Cargo.toml**

```bash
cat crates/lsp/Cargo.toml
```

**Step 2: Add WASM dependencies to Cargo.toml**

```bash
# Add to crates/lsp/Cargo.toml [dependencies] section:
# wasm-bindgen = "0.2"
# serde_json = "1.0"
# console_error_panic_hook = "0.1"
```

Expected: Cargo.toml updated with WASM dependencies

**Step 3: Create crates/lsp/src/lib.rs with module exports**

```bash
cat > crates/lsp/src/lib.rs << 'EOF'
pub mod backend;
pub mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;
EOF
```

**Step 4: Create wasm module directory**

```bash
mkdir -p crates/lsp/src/wasm
```

**Step 5: Create crates/lsp/src/wasm/mod.rs**

```bash
cat > crates/lsp/src/wasm/mod.rs << 'EOF'
mod exports;

pub use exports::*;
EOF
```

**Step 6: Create crates/lsp/src/wasm/exports.rs**

```bash
cat > crates/lsp/src/wasm/exports.rs << 'EOF'
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct LspServer {
    // TODO: Add backend field in next task
}

#[wasm_bindgen]
impl LspServer {
    #[wasm_bindgen(constructor)]
    pub fn new(_dialect: &str) -> Self {
        // TODO: Initialize backend in next task
        Self {}
    }

    #[wasm_bindgen]
    pub fn completion(&self, _text: &str, _line: u32, _col: u32) -> JsValue {
        // TODO: Implement in next task
        JsValue::from_str("[]")
    }
}
EOF
```

**Step 7: Verify WASM can compile**

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown -p unified-sql-lsp-lsp
```

Expected: Compilation succeeds (may have warnings, but no errors)

**Step 8: Commit WASM infrastructure**

```bash
git add crates/lsp/Cargo.toml crates/lsp/src/lib.rs crates/lsp/src/wasm/
git commit -m "feat(lsp): add WASM module skeleton"
```

---

### Task 4: Extract shared LSP core logic

**Files:**
- Create: `crates/lsp/src/core.rs`
- Modify: `crates/lsp/src/backend.rs`
- Modify: `crates/lsp/src/wasm/exports.rs`

**Step 1: Read current backend.rs**

```bash
head -100 crates/lsp/src/backend.rs
```

**Step 2: Identify shared logic**

Look for business logic that's independent of tower-lsp transport layer (e.g., completion handling, hover logic)

**Step 3: Create crates/lsp/src/core.rs with shared logic**

```bash
# This is a placeholder - actual content depends on backend.rs structure
cat > crates/lsp/src/core.rs << 'EOF'
// TODO: Extract shared completion/hover/diagnostic logic from backend.rs
// This is the business logic that both tower-lsp and WASM backends will use

pub struct LspCore {
    // TODO: Add fields for catalog, semantic analyzer, etc.
}

impl LspCore {
    pub fn new() -> Self {
        Self {}
    }

    pub fn completion(&self, _text: &str, _line: u32, _col: u32) -> Vec<lsp_types::CompletionItem> {
        // TODO: Implement completion logic
        vec![]
    }

    pub fn hover(&self, _text: &str, _line: u32, _col: u32) -> Option<lsp_types::Hover> {
        // TODO: Implement hover logic
        None
    }

    pub fn diagnostics(&self, _text: &str) -> Vec<lsp_types::Diagnostic> {
        // TODO: Implement diagnostic logic
        vec![]
    }
}
EOF
```

**Step 4: Update backend.rs to use shared core**

```bash
# Modify backend.rs to delegate to LspCore instead of implementing logic directly
# This depends on the actual structure of backend.rs
```

**Step 5: Update wasm/exports.rs to use shared core**

```bash
cat > crates/lsp/src/wasm/exports.rs << 'EOF'
use crate::core::LspCore;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct LspServer {
    core: LspCore,
}

#[wasm_bindgen]
impl LspServer {
    #[wasm_bindgen(constructor)]
    pub fn new(_dialect: &str) -> Self {
        Self {
            core: LspCore::new(),
        }
    }

    #[wasm_bindgen]
    pub fn completion(&self, text: &str, line: u32, col: u32) -> JsValue {
        let items = self.core.completion(text, line, col);
        serde_json::to_string(&items).unwrap().into()
    }

    #[wasm_bindgen]
    pub fn hover(&self, text: &str, line: u32, col: u32) -> JsValue {
        let hover = self.core.hover(text, line, col);
        serde_json::to_string(&hover).unwrap().into()
    }

    #[wasm_bindgen]
    pub fn diagnostics(&self, text: &str) -> JsValue {
        let diags = self.core.diagnostics(text);
        serde_json::to_string(&diags).unwrap().into()
    }
}
EOF
```

**Step 6: Verify everything still compiles**

```bash
cargo build --workspace
```

Expected: No errors

**Step 7: Run tests to ensure no regressions**

```bash
cargo test --workspace
```

Expected: All tests pass

**Step 8: Commit core extraction**

```bash
git add crates/lsp/src/core.rs crates/lsp/src/backend.rs crates/lsp/src/wasm/exports.rs
git commit -m "refactor(lsp): extract shared LSP core logic"
```

---

### Task 5: Configure wasm-pack build

**Files:**
- Create: `playground/src/wasm/` (will be generated)
- Modify: `Makefile`

**Step 1: Install wasm-pack**

```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

Expected: wasm-pack installed successfully

**Step 2: Build WASM module**

```bash
wasm-pack build crates/lsp --target web --out-dir playground/src/wasm
```

Expected: Creates `playground/src/wasm/unified_sql_lsp_lsp.js` and `.wasm` files

**Step 3: Add playground WASM build script to Makefile**

```bash
# Add to Makefile after the existing targets

## Playground
.PHONY: playground-wasm playground-dev playground-build

playground-wasm:
	wasm-pack build crates/lsp --target web --out-dir playground/src/wasm

playground-dev:
	cd playground && npm run dev

playground-build:
	cd playground && npm run build
EOF
```

**Step 4: Test WASM build**

```bash
make playground-wasm
```

Expected: WASM files generated in `playground/src/wasm/`

**Step 5: Commit WASM build configuration**

```bash
git add Makefile
git add -f playground/src/wasm/  # Add generated files
git commit -m "feat(playground): add wasm-pack build configuration"
```

---

## Phase 3: LSP Bridge Layer

### Task 6: Create WASM interface loader

**Files:**
- Create: `playground/src/lib/wasm-interface.ts`

**Step 1: Create lib directory**

```bash
mkdir -p playground/src/lib
```

**Step 2: Create wasm-interface.ts**

```bash
cat > playground/src/lib/wasm-interface.ts << 'EOF'
import init, { LspServer } from '../wasm/unified_sql_lsp_lsp.js'

let wasmInstance: LspServer | null = null

export async function initWasm(dialect: string = 'mysql'): Promise<LspServer> {
  if (wasmInstance) {
    return wasmInstance
  }

  await init()
  wasmInstance = new LspServer(dialect)
  return wasmInstance
}

export function getWasmInstance(): LspServer | null {
  return wasmInstance
}
EOF
```

**Step 3: Update App.tsx to initialize WASM**

```bash
cat > playground/src/App.tsx << 'EOF'
import { useEffect, useRef, useState } from 'react'
import * as monaco from 'monaco-editor'
import { initWasm } from './lib/wasm-interface'

export default function App() {
  const editorRef = useRef<HTMLDivElement>(null)
  const editorInstanceRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
  const [wasmReady, setWasmReady] = useState(false)

  useEffect(() => {
    async function init() {
      try {
        await initWasm('mysql')
        setWasmReady(true)
        console.log('WASM initialized successfully')
      } catch (error) {
        console.error('Failed to initialize WASM:', error)
      }
    }
    init()
  }, [])

  useEffect(() => {
    if (!editorRef.current) return

    editorInstanceRef.current = monaco.editor.create(editorRef.current, {
      value: 'SELECT * FROM users WHERE id = 1;',
      language: 'sql',
      theme: 'vs-dark',
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: 14,
      lineNumbers: 'on',
      scrollBeyondLastLine: false,
    })

    return () => {
      editorInstanceRef.current?.dispose()
    }
  }, [])

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <header style={{ padding: '1rem', borderBottom: '1px solid #30363d', background: '#0d1117' }}>
        <h1 style={{ color: '#c9d1d9', fontSize: '1.5rem' }}>
          Unified SQL LSP Playground
          {!wasmReady && <span style={{ fontSize: '0.875rem', marginLeft: '1rem', color: '#f85149' }}>Loading WASM...</span>}
          {wasmReady && <span style={{ fontSize: '0.875rem', marginLeft: '1rem', color: '#3fb950' }}>✓ WASM Ready</span>}
        </h1>
      </header>
      <main style={{ flex: 1, display: 'flex' }}>
        <aside style={{ width: '250px', borderRight: '1px solid #30363d', background: '#0d1117', padding: '1rem' }}>
          <h2 style={{ color: '#c9d1d9', fontSize: '1rem', marginBottom: '1rem' }}>Schema Browser</h2>
          <div style={{ color: '#8b949e', fontSize: '0.875rem' }}>
            <p>Coming soon...</p>
          </div>
        </aside>
        <div style={{ flex: 1 }}>
          <div ref={editorRef} style={{ height: '100%' }} />
        </div>
      </main>
    </div>
  )
}
EOF
```

**Step 4: Test WASM initialization**

```bash
npm run dev
```

Expected: Browser shows "✓ WASM Ready" status indicator

**Step 5: Commit WASM interface**

```bash
git add playground/src/lib/wasm-interface.ts playground/src/App.tsx
git commit -m "feat(playground): add WASM initialization and status indicator"
```

---

### Task 7: Implement LSP protocol adapter

**Files:**
- Create: `playground/src/lib/lsp-bridge.ts`

**Step 1: Create lsp-bridge.ts**

```bash
cat > playground/src/lib/lsp-bridge.ts << 'EOF'
import * as monaco from 'monaco-editor'
import { getWasmInstance } from './wasm-interface'

export class LspBridge {
  private editor: monaco.editor.IStandaloneCodeEditor
  private debounceTimer: ReturnType<typeof setTimeout> | null = null

  constructor(editor: monaco.editor.IStandaloneCodeEditor) {
    this.editor = editor
    this.setupProviders()
  }

  private setupProviders() {
    // Register completion provider
    monaco.languages.registerCompletionItemProvider('sql', {
      provideCompletionItems: async (model, position) => {
        const text = model.getValue()
        const line = position.lineNumber
        const col = position.column

        const wasm = getWasmInstance()
        if (!wasm) return { suggestions: [] }

        try {
          const result = wasm.completion(text, line, col)
          const items = JSON.parse(result)
          return { suggestions: this.convertCompletionItems(items) }
        } catch (error) {
          console.error('Completion error:', error)
          return { suggestions: [] }
        }
      }
    })

    // Register hover provider
    monaco.languages.registerHoverProvider('sql', {
      provideHover: async (model, position) => {
        const text = model.getValue()
        const line = position.lineNumber
        const col = position.column

        const wasm = getWasmInstance()
        if (!wasm) return null

        try {
          const result = wasm.hover(text, line, col)
          const hover = JSON.parse(result)
          if (!hover) return null
          return {
            range: new monaco.Range(line, col, line, col),
            contents: [{ value: hover.contents.value }]
          }
        } catch (error) {
          console.error('Hover error:', error)
          return null
        }
      }
    })
  }

  private convertCompletionItems(items: any[]): monaco.languages.CompletionItem[] {
    return items.map(item => ({
      label: item.label,
      kind: this.convertCompletionKind(item.kind),
      detail: item.detail,
      documentation: item.documentation,
      insertText: item.insertText || item.label,
      range: undefined // Monaco will calculate
    }))
  }

  private convertCompletionKind(kind: number): monaco.languages.CompletionItemKind {
    // Map LSP CompletionItemKind to Monaco CompletionItemKind
    const kindMap: Record<number, monaco.languages.CompletionItemKind> = {
      1: monaco.languages.CompletionItemKind.Text,
      2: monaco.languages.CompletionItemKind.Method,
      3: monaco.languages.CompletionItemKind.Function,
      4: monaco.languages.CompletionItemKind.Constructor,
      5: monaco.languages.CompletionItemKind.Field,
      6: monaco.languages.CompletionItemKind.Variable,
      7: monaco.languages.CompletionItemKind.Class,
      8: monaco.languages.CompletionItemKind.Interface,
      9: monaco.languages.CompletionItemKind.Module,
      10: monaco.languages.CompletionItemKind.Property,
      11: monaco.languages.CompletionItemKind.Unit,
      12: monaco.languages.CompletionItemKind.Value,
      13: monaco.languages.CompletionItemKind.Enum,
      14: monaco.languages.CompletionItemKind.Keyword,
      15: monaco.languages.CompletionItemKind.Snippet,
      16: monaco.languages.CompletionItemKind.Color,
      17: monaco.languages.CompletionItemKind.File,
      18: monaco.languages.CompletionItemKind.Reference,
    }
    return kindMap[kind] || monaco.languages.CompletionItemKind.Text
  }

  updateDiagnostics(model: monaco.editor.ITextModel) {
    const wasm = getWasmInstance()
    if (!wasm) return

    // Debounce diagnostics
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer)
    }

    this.debounceTimer = setTimeout(() => {
      const text = model.getValue()
      try {
        const result = wasm.diagnostics(text)
        const diagnostics = JSON.parse(result)
        monaco.editor.setModelMarkers(model, 'lsp', diagnostics.map((d: any) => ({
          severity: d.severity === 1 ? monaco.MarkerSeverity.Error : monaco.MarkerSeverity.Warning,
          message: d.message,
          startLineNumber: d.range.start.line,
          startColumn: d.range.start.character,
          endLineNumber: d.range.end.line,
          endColumn: d.range.end.character,
        })))
      } catch (error) {
        console.error('Diagnostics error:', error)
      }
    }, 300)
  }

  dispose() {
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer)
    }
  }
}
EOF
```

**Step 2: Update App.tsx to use LspBridge**

```bash
cat > playground/src/App.tsx << 'EOF'
import { useEffect, useRef, useState } from 'react'
import * as monaco from 'monaco-editor'
import { initWasm } from './lib/wasm-interface'
import { LspBridge } from './lib/lsp-bridge'

export default function App() {
  const editorRef = useRef<HTMLDivElement>(null)
  const editorInstanceRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
  const lspBridgeRef = useRef<LspBridge | null>(null)
  const [wasmReady, setWasmReady] = useState(false)

  useEffect(() => {
    async function init() {
      try {
        await initWasm('mysql')
        setWasmReady(true)
        console.log('WASM initialized successfully')
      } catch (error) {
        console.error('Failed to initialize WASM:', error)
      }
    }
    init()
  }, [])

  useEffect(() => {
    if (!editorRef.current) return

    editorInstanceRef.current = monaco.editor.create(editorRef.current, {
      value: 'SELECT * FROM users WHERE id = 1;',
      language: 'sql',
      theme: 'vs-dark',
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: 14,
      lineNumbers: 'on',
      scrollBeyondLastLine: false,
    })

    // Initialize LSP bridge when WASM is ready
    if (wasmReady && editorInstanceRef.current) {
      lspBridgeRef.current = new LspBridge(editorInstanceRef.current)

      // Trigger initial diagnostics
      const model = editorInstanceRef.current.getModel()
      if (model) {
        lspBridgeRef.current.updateDiagnostics(model)

        // Listen for content changes
        model.onDidChangeContent(() => {
          lspBridgeRef.current?.updateDiagnostics(model)
        })
      }
    }

    return () => {
      lspBridgeRef.current?.dispose()
      editorInstanceRef.current?.dispose()
    }
  }, [wasmReady])

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <header style={{ padding: '1rem', borderBottom: '1px solid #30363d', background: '#0d1117' }}>
        <h1 style={{ color: '#c9d1d9', fontSize: '1.5rem' }}>
          Unified SQL LSP Playground
          {!wasmReady && <span style={{ fontSize: '0.875rem', marginLeft: '1rem', color: '#f85149' }}>Loading WASM...</span>}
          {wasmReady && <span style={{ fontSize: '0.875rem', marginLeft: '1rem', color: '#3fb950' }}>✓ WASM Ready</span>}
        </h1>
      </header>
      <main style={{ flex: 1, display: 'flex' }}>
        <aside style={{ width: '250px', borderRight: '1px solid #30363d', background: '#0d1117', padding: '1rem' }}>
          <h2 style={{ color: '#c9d1d9', fontSize: '1rem', marginBottom: '1rem' }}>Schema Browser</h2>
          <div style={{ color: '#8b949e', fontSize: '0.875rem' }}>
            <p>Coming soon...</p>
          </div>
        </aside>
        <div style={{ flex: 1 }}>
          <div ref={editorRef} style={{ height: '100%' }} />
        </div>
      </main>
    </div>
  )
}
EOF
```

**Step 3: Test LSP bridge**

```bash
npm run dev
```

Expected: Completions and hover providers registered (functionality depends on LSP core implementation)

**Step 4: Commit LSP bridge**

```bash
git add playground/src/lib/lsp-bridge.ts playground/src/App.tsx
git commit -m "feat(playground): add LSP protocol adapter for Monaco"
```

---

## Phase 4: Feature Integration

### Task 8: Implement mock completion data for testing

**Files:**
- Modify: `crates/lsp/src/core.rs`

**Step 1: Add mock completion logic to LspCore**

```bash
# Update LspCore::completion to return mock data
cat >> crates/lsp/src/core.rs << 'EOF'

// Mock implementation for testing
pub fn completion(&self, text: &str, _line: u32, _col: u32) -> Vec<lsp_types::CompletionItem> {
    let mut items = vec![
        lsp_types::CompletionItem {
            label: "SELECT".into(),
            kind: Some(lsp_types::CompletionItemKind::KEYWORD),
            detail: Some("Keyword".into()),
            ..Default::default()
        },
        lsp_types::CompletionItem {
            label: "FROM".into(),
            kind: Some(lsp_types::CompletionItemKind::KEYWORD),
            detail: Some("Keyword".into()),
            ..Default::default()
        },
        lsp_types::CompletionItem {
            label: "WHERE".into(),
            kind: Some(lsp_types::CompletionItemKind::KEYWORD),
            detail: Some("Keyword".into()),
            ..Default::default()
        },
        lsp_types::CompletionItem {
            label: "users".into(),
            kind: Some(lsp_types::CompletionItemKind::CLASS),
            detail: Some("Table".into()),
            ..Default::default()
        },
        lsp_types::CompletionItem {
            label: "id".into(),
            kind: Some(lsp_types::CompletionItemKind::FIELD),
            detail: Some("Column (INT)".into()),
            ..Default::default()
        },
        lsp_types::CompletionItem {
            label: "name".into(),
            kind: Some(lsp_types::CompletionItemKind::FIELD),
            detail: Some("Column (VARCHAR)".into()),
            ..Default::default()
        },
    ];

    // Simple context: suggest FROM after SELECT
    if text.contains("SELECT") && !text.contains("FROM") {
        items.push(lsp_types::CompletionItem {
            label: "FROM".into(),
            kind: Some(lsp_types::CompletionItemKind::KEYWORD),
            detail: Some("Keyword".into()),
            insert_text: Some(" FROM ".into()),
            ..Default::default()
        });
    }

    items
}
EOF
```

**Step 2: Rebuild WASM**

```bash
make playground-wasm
```

**Step 3: Test completions in browser**

```bash
npm run dev
```

Expected: Typing in editor shows completion suggestions

**Step 4: Commit mock completions**

```bash
git add crates/lsp/src/core.rs playground/src/wasm/
git commit -m "feat(lsp): add mock completion data for playground testing"
```

---

### Task 9: Add static schema browser

**Files:**
- Create: `playground/src/components/SchemaBrowser.tsx`
- Modify: `playground/src/App.tsx`

**Step 1: Create components directory**

```bash
mkdir -p playground/src/components
```

**Step 2: Create SchemaBrowser component**

```bash
cat > playground/src/components/SchemaBrowser.tsx << 'EOF'
import { useState } from 'react'

interface Table {
  name: string
  columns: Column[]
}

interface Column {
  name: string
  type: string
}

const mockSchema: Table[] = [
  {
    name: 'users',
    columns: [
      { name: 'id', type: 'INT' },
      { name: 'name', type: 'VARCHAR(100)' },
      { name: 'email', type: 'VARCHAR(255)' },
      { name: 'created_at', type: 'TIMESTAMP' },
    ]
  },
  {
    name: 'orders',
    columns: [
      { name: 'id', type: 'INT' },
      { name: 'user_id', type: 'INT' },
      { name: 'total', type: 'DECIMAL(10,2)' },
      { name: 'status', type: 'ENUM' },
    ]
  },
  {
    name: 'order_items',
    columns: [
      { name: 'id', type: 'INT' },
      { name: 'order_id', type: 'INT' },
      { name: 'product_id', type: 'INT' },
      { name: 'quantity', type: 'INT' },
    ]
  },
]

export function SchemaBrowser() {
  const [expandedTables, setExpandedTables] = useState<Set<string>>(new Set())

  const toggleTable = (tableName: string) => {
    const newExpanded = new Set(expandedTables)
    if (newExpanded.has(tableName)) {
      newExpanded.delete(tableName)
    } else {
      newExpanded.add(tableName)
    }
    setExpandedTables(newExpanded)
  }

  return (
    <div>
      <h2 style={{ color: '#c9d1d9', fontSize: '1rem', marginBottom: '1rem' }}>
        Schema Browser
      </h2>
      <div style={{ fontSize: '0.875rem' }}>
        {mockSchema.map((table) => (
          <div key={table.name} style={{ marginBottom: '0.5rem' }}>
            <div
              onClick={() => toggleTable(table.name)}
              style={{
                color: '#58a6ff',
                cursor: 'pointer',
                userSelect: 'none',
                padding: '0.25rem 0.5rem',
                background: expandedTables.has(table.name) ? '#161b22' : 'transparent',
                borderRadius: '4px',
              }}
            >
              {expandedTables.has(table.name) ? '▼' : '▶'} {table.name}
            </div>
            {expandedTables.has(table.name) && (
              <div style={{ marginLeft: '1rem', marginTop: '0.25rem' }}>
                {table.columns.map((column) => (
                  <div
                    key={column.name}
                    style={{
                      color: '#8b949e',
                      padding: '0.125rem 0.5rem',
                      cursor: 'pointer',
                    }}
                    title={column.type}
                  >
                    {column.name}
                    <span style={{ fontSize: '0.75rem', marginLeft: '0.5rem', color: '#6e7681' }}>
                      {column.type}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  )
}
EOF
```

**Step 3: Update App.tsx to use SchemaBrowser**

```bash
# Update the import and usage in App.tsx
# Replace the sidebar with <SchemaBrowser />
```

**Step 4: Test schema browser**

```bash
npm run dev
```

Expected: Expandable/collapsible schema tree in sidebar

**Step 5: Commit schema browser**

```bash
git add playground/src/components/SchemaBrowser.tsx playground/src/App.tsx
git commit -m "feat(playground): add schema browser component"
```

---

### Task 10: Add diagnostics panel

**Files:**
- Create: `playground/src/components/DiagnosticsPanel.tsx`
- Modify: `playground/src/App.tsx`

**Step 1: Create DiagnosticsPanel component**

```bash
cat > playground/src/components/DiagnosticsPanel.tsx << 'EOF'
import { useEffect, useState } from 'react'
import * as monaco from 'monaco-editor'

interface Diagnostic {
  severity: string
  message: string
  line: number
  column: number
}

export function DiagnosticsPanel({ editor }: { editor: monaco.editor.IStandaloneCodeEditor | null }) {
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([])

  useEffect(() => {
    if (!editor) return

    const updateDiagnostics = () => {
      const model = editor.getModel()
      if (!model) return

      const markers = monaco.editor.getModelMarkers({ resource: model.uri })
      setDiagnostics(
        markers.map((m) => ({
          severity: m.severity === monaco.MarkerSeverity.Error ? 'Error' : 'Warning',
          message: m.message,
          line: m.startLineNumber,
          column: m.startColumn,
        }))
      )
    }

    // Initial update
    updateDiagnostics()

    // Listen for marker changes
    const disposable = editor.onDidChangeModelDecorations(updateDiagnostics)

    return () => disposable.dispose()
  }, [editor])

  const errorCount = diagnostics.filter((d) => d.severity === 'Error').length
  const warningCount = diagnostics.filter((d) => d.severity === 'Warning').length

  return (
    <div
      style={{
        borderTop: '1px solid #30363d',
        background: '#0d1117',
        padding: '1rem',
        maxHeight: '200px',
        overflowY: 'auto',
      }}
    >
      <div style={{ marginBottom: '0.5rem', fontSize: '0.875rem', color: '#c9d1d9' }}>
        <strong>Diagnostics</strong>
        <span style={{ marginLeft: '1rem' }}>
          {errorCount > 0 && <span style={{ color: '#f85149' }}>{errorCount} errors</span>}
          {errorCount > 0 && warningCount > 0 && ' • '}
          {warningCount > 0 && <span style={{ color: '#d29922' }}>{warningCount} warnings</span>}
          {errorCount === 0 && warningCount === 0 && <span style={{ color: '#3fb950' }}>No problems</span>}
        </span>
      </div>
      {diagnostics.length > 0 && (
        <div style={{ fontSize: '0.875rem' }}>
          {diagnostics.map((diag, i) => (
            <div
              key={i}
              style={{
                padding: '0.25rem 0.5rem',
                marginBottom: '0.25rem',
                background: diag.severity === 'Error' ? '#4c1818' : '#3d2a00',
                borderRadius: '4px',
                cursor: 'pointer',
              }}
              onClick={() => {
                if (editor) {
                  editor.setPosition({ lineNumber: diag.line, column: diag.column })
                  editor.focus()
                }
              }}
            >
              <span style={{ color: diag.severity === 'Error' ? '#f85149' : '#d29922' }}>
                {diag.severity === 'Error' ? '✖' : '⚠'}
              </span>
              <span style={{ marginLeft: '0.5rem', color: '#c9d1d9' }}>{diag.message}</span>
              <span style={{ marginLeft: '0.5rem', color: '#6e7681', fontSize: '0.75rem' }}>
                Line {diag.line}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
EOF
```

**Step 2: Update App.tsx to include DiagnosticsPanel**

```bash
# Add DiagnosticsPanel component to App.tsx
# Pass editor instance as prop
```

**Step 3: Test diagnostics panel**

```bash
npm run dev
```

Expected: Panel shows errors/warnings, click to jump to location

**Step 4: Commit diagnostics panel**

```bash
git add playground/src/components/DiagnosticsPanel.tsx playground/src/App.tsx
git commit -m "feat(playground): add diagnostics panel"
```

---

### Task 11: Add dialect switcher

**Files:**
- Modify: `playground/src/App.tsx`

**Step 1: Add dialect selector to header**

```bash
# Update App.tsx header to include dialect dropdown
# Add state for current dialect
# Add function to handle dialect change
```

**Step 2: Test dialect switching**

```bash
npm run dev
```

Expected: Changing dialect resets editor and updates WASM instance

**Step 3: Commit dialect switcher**

```bash
git add playground/src/App.tsx
git commit -m "feat(playground): add dialect switcher"
```

---

## Phase 5: Polish

### Task 12: Add example queries

**Files:**
- Create: `playground/src/lib/example-queries.ts`
- Modify: `playground/src/App.tsx`

**Step 1: Create example queries module**

```bash
cat > playground/src/lib/example-queries.ts << 'EOF'
export const exampleQueries = {
  'Simple SELECT': 'SELECT * FROM users;',
  'SELECT with WHERE': 'SELECT * FROM users WHERE id = 1;',
  'JOIN example': `SELECT u.name, o.total
FROM users u
JOIN orders o ON u.id = o.user_id;`,
  'Aggregation': 'SELECT COUNT(*) FROM orders;',
  'Show all completions': 'SELECT  FROM users WHERE  = 1;',
}
EOF
```

**Step 2: Add example query dropdown to UI**

```bash
# Update App.tsx to include example selector
# On change, update editor value
```

**Step 3: Test example queries**

```bash
npm run dev
```

Expected: Selecting example updates editor content

**Step 4: Commit example queries**

```bash
git add playground/src/lib/example-queries.ts playground/src/App.tsx
git commit -m "feat(playground): add example queries dropdown"
```

---

### Task 13: Add loading states and polish UI

**Files:**
- Modify: `playground/src/App.tsx`
- Modify: `playground/src/components/*.tsx`

**Step 1: Add loading spinners**

```bash
# Add loading indicator while WASM initializes
# Add loading indicator during completion requests
```

**Step 2: Improve styling**

```bash
# Enhance CSS for better visual hierarchy
# Add hover effects
# Improve responsive design
```

**Step 3: Test UI polish**

```bash
npm run dev
```

Expected: Smooth transitions, clear feedback

**Step 4: Commit UI polish**

```bash
git add playground/src/
git commit -m "style(playground): add loading states and UI improvements"
```

---

### Task 14: Performance optimization

**Files:**
- Modify: `playground/src/lib/lsp-bridge.ts`
- Modify: `crates/lsp/src/core.rs`

**Step 1: Optimize debounce timing**

```bash
# Adjust debounce delays for better responsiveness
# Add request queuing to prevent overlapping requests
```

**Step 2: Optimize WASM bundle size**

```bash
# Enable lto for WASM builds
# Strip debug symbols
# Add to crates/lsp/Cargo.toml:
[profile.release.package.unified-sql-lsp-lsp]
opt-level = "z"
lto = true
codegen-units = 1
EOF
```

**Step 3: Rebuild WASM with optimizations**

```bash
make playground-wasm
```

**Step 4: Test performance**

```bash
npm run build
npm run preview
```

Expected: Smaller bundle size, responsive UI

**Step 5: Commit optimizations**

```bash
git add crates/lsp/Cargo.toml playground/src/lib/lsp-bridge.ts playground/src/wasm/
git commit -m "perf(playground): optimize bundle size and responsiveness"
```

---

## Phase 6: Deployment

### Task 15: Set up deployment

**Files:**
- Create: `.github/workflows/playground.yml`
- Modify: `Makefile`

**Step 1: Create GitHub Actions workflow**

```bash
mkdir -p .github/workflows
cat > .github/workflows/playground.yml << 'EOF'
name: Deploy Playground

on:
  push:
    branches: [main]
    paths:
      - 'playground/**'
      - 'crates/lsp/**'

permissions:
  contents: read
  pages: write
  id-token: write

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

      - name: Install Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Build WASM
        run: make playground-wasm

      - name: Install dependencies
        run: cd playground && npm install

      - name: Build
        run: cd playground && npm run build

      - name: Setup Pages
        uses: actions/configure-pages@v4

      - name: Upload artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: playground/dist

      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
EOF
```

**Step 2: Update Makefile with deploy target**

```bash
# Add to Makefile:
playground-deploy:
	@echo "Deploy to GitHub Pages via GitHub Actions"
```

**Step 3: Test production build locally**

```bash
cd playground && npm run build
```

Expected: Build succeeds, dist/ directory created

**Step 4: Commit deployment configuration**

```bash
git add .github/workflows/playground.yml Makefile
git commit -m "ci(playground): add GitHub Pages deployment workflow"
```

**Step 15: Final verification**

**Step 1: Run full test suite**

```bash
cargo test --workspace
```

Expected: All tests pass

**Step 2: Test playground end-to-end**

```bash
cd playground && npm run build && npm run preview
```

Expected: Playground works in production mode

**Step 3: Document usage in README**

```bash
# Add section to README.md explaining the playground
# Include link to deployed version
```

**Step 4: Final commit**

```bash
git add README.md
git commit -m "docs: add playground documentation to README"
```

**Step 5: Push to remote**

```bash
git push origin feature/playground
```

**Step 6: Create pull request**

```bash
gh pr create --title "feat: Add SQL LSP Playground" --body "Implements web-based playground for LSP evaluation"
```

---

## Success Criteria Verification

After completing all tasks, verify:

- ✅ Developers can write SQL queries and see real-time completions
- ✅ Hover tooltips show accurate type/signature information
- ✅ Diagnostics correctly identify syntax and semantic errors
- ✅ Dialect switching works seamlessly
- ✅ Performance feels responsive (<300ms latency)
- ✅ Works offline after initial load
- ✅ Bundle size is reasonable (<5MB gzipped)

---

## Notes

- This plan assumes LSP core logic exists in `backend.rs` - adjust Task 4 based on actual structure
- Mock completion data in Task 8 should be replaced with real LSP logic when ready
- All file paths are relative to workspace root
- Worktree location: `.worktrees/playground/`
- Branch: `feature/playground`
