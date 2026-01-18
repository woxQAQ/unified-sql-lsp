import { useEffect, useRef, useState } from 'react'
import * as monaco from 'monaco-editor'
import { initWasm } from './lib/wasm-interface'
import { LspBridge } from './lib/lsp-bridge'
import { SchemaBrowser } from './components/SchemaBrowser'
import { DiagnosticsPanel } from './components/DiagnosticsPanel'

const DIALECTS = ['MySQL', 'PostgreSQL', 'TiDB', 'MariaDB', 'CockroachDB'] as const
type Dialect = (typeof DIALECTS)[number]

const EXAMPLE_QUERIES = {
  'Basic SELECT': `SELECT * FROM users WHERE id = 1;`,
  'JOIN Query': `SELECT u.name, o.total
FROM users u
JOIN orders o ON u.id = o.user_id
WHERE o.status = 'pending'
LIMIT 10;`,
  'Aggregation': `SELECT
  COUNT(*) as total_users,
  AVG(age) as average_age,
  MAX(created_at) as latest_signup
FROM users
WHERE active = true;`,
  'Subquery': `SELECT name, email
FROM users
WHERE id IN (
  SELECT user_id
  FROM orders
  WHERE total > 1000
);`,
  'CTE': `WITH user_orders AS (
  SELECT user_id, COUNT(*) as order_count, SUM(total) as total_spent
  FROM orders
  GROUP BY user_id
)
SELECT u.name, uo.order_count, uo.total_spent
FROM users u
JOIN user_orders uo ON u.id = uo.user_id
ORDER BY uo.total_spent DESC
LIMIT 5;`,
  'Window Function': `SELECT
  name,
  total,
  ROW_NUMBER() OVER (ORDER BY total DESC) as rank,
  LAG(total) OVER (ORDER BY created_at) as prev_total
FROM orders
WHERE created_at >= '2024-01-01';`,
} as const

type ExampleQuery = keyof typeof EXAMPLE_QUERIES

export default function App() {
  const editorRef = useRef<HTMLDivElement>(null)
  const editorInstanceRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
  const lspBridgeRef = useRef<LspBridge | null>(null)
  const [wasmReady, setWasmReady] = useState(false)
  const [wasmError, setWasmError] = useState<string | null>(null)
  const [currentDialect, setCurrentDialect] = useState<Dialect>('MySQL')

  const loadExampleQuery = (queryName: ExampleQuery) => {
    if (!editorInstanceRef.current) return

    const query = EXAMPLE_QUERIES[queryName]
    const model = editorInstanceRef.current.getModel()
    if (model) {
      editorInstanceRef.current.executeEdits('load-example', [
        {
          range: model.getFullModelRange(),
          text: query,
        },
      ])
    }
  }

  useEffect(() => {
    async function init() {
      try {
        setWasmError(null)
        const dialect = currentDialect.toLowerCase()
        await initWasm(dialect)
        setWasmReady(true)
        console.log(`WASM initialized successfully for ${currentDialect}`)
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error'
        setWasmError(errorMessage)
        setWasmReady(false)
        console.error('Failed to initialize WASM:', error)
      }
    }
    init()
  }, [currentDialect])

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
      suggest: {
        showIcons: true,
        showSnippets: true,
      },
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
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <h1 style={{ color: '#c9d1d9', fontSize: '1.5rem', margin: 0 }}>
            Unified SQL LSP Playground
            {!wasmReady && !wasmError && (
              <span style={{ fontSize: '0.875rem', marginLeft: '1rem', color: '#f85149' }}>
                Loading WASM...
              </span>
            )}
            {wasmReady && (
              <span style={{ fontSize: '0.875rem', marginLeft: '1rem', color: '#3fb950' }}>
                ✓ WASM Ready
              </span>
            )}
            {wasmError && (
              <span
                style={{ fontSize: '0.875rem', marginLeft: '1rem', color: '#f85149' }}
                title={wasmError}
              >
                ✗ WASM Error (see console)
              </span>
            )}
          </h1>
          <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
            <label style={{ color: '#8b949e', fontSize: '0.875rem', fontWeight: 500 }}>
              Examples:
              <select
                onChange={(e) => loadExampleQuery(e.target.value as ExampleQuery)}
                defaultValue=""
                style={{
                  marginLeft: '0.5rem',
                  padding: '0.25rem 0.5rem',
                  background: '#21262d',
                  border: '1px solid #30363d',
                  borderRadius: '4px',
                  color: '#c9d1d9',
                  fontSize: '0.875rem',
                  cursor: 'pointer',
                }}
              >
                <option value="" disabled>
                  Load example...
                </option>
                {Object.keys(EXAMPLE_QUERIES).map((query) => (
                  <option key={query} value={query}>
                    {query}
                  </option>
                ))}
              </select>
            </label>
            <label style={{ color: '#8b949e', fontSize: '0.875rem', fontWeight: 500 }}>
              Dialect:
              <select
                value={currentDialect}
                onChange={(e) => setCurrentDialect(e.target.value as Dialect)}
                style={{
                  marginLeft: '0.5rem',
                  padding: '0.25rem 0.5rem',
                  background: '#21262d',
                  border: '1px solid #30363d',
                  borderRadius: '4px',
                  color: '#c9d1d9',
                  fontSize: '0.875rem',
                  cursor: 'pointer',
                }}
              >
                {DIALECTS.map((dialect) => (
                  <option key={dialect} value={dialect}>
                    {dialect}
                  </option>
                ))}
              </select>
            </label>
          </div>
        </div>
      </header>
      <main style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
        <div style={{ flex: 1, display: 'flex' }}>
          <aside style={{ width: '250px', borderRight: '1px solid #30363d', background: '#0d1117', padding: '1rem' }}>
            <SchemaBrowser />
          </aside>
          <div style={{ flex: 1 }}>
            <div ref={editorRef} style={{ height: '100%' }} />
          </div>
        </div>
        <DiagnosticsPanel editor={editorInstanceRef.current} />
      </main>
    </div>
  )
}
