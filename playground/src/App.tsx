import { useEffect, useRef, useState } from 'react'
import * as monaco from 'monaco-editor'
import { LspClient } from './lib/lsp-client'
import { setupMonacoWithLSP, updateDiagnostics } from './lib/monaco-setup'
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

// Helper to update Monaco diagnostics (avoid name collision)
function updateDiagnosticsMonaco(
  editor: monaco.editor.IStandaloneCodeEditor,
  diagnostics: any[]
) {
  updateDiagnostics(editor, diagnostics);
}

export default function App() {
  const editorRef = useRef<HTMLDivElement>(null)
  const editorInstanceRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
  const lspClientRef = useRef<LspClient | null>(null)
  const [connectionStatus, setConnectionStatus] = useState<'connecting' | 'connected' | 'disconnected'>('disconnected')
  const [connectionError, setConnectionError] = useState<string | null>(null)
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
        setConnectionError(null)
        setConnectionStatus('connecting')

        // Create LSP client
        lspClientRef.current = new LspClient()

        // Connect to server
        await lspClientRef.current.connect()
        setConnectionStatus('connected')
        console.log('[App] Connected to LSP server')
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error'
        setConnectionError(errorMessage)
        setConnectionStatus('disconnected')
        console.error('[App] Failed to connect to LSP server:', error)
      }
    }

    init()

    // Cleanup on unmount
    return () => {
      lspClientRef.current?.disconnect()
    }
  }, [])

  useEffect(() => {
    if (!editorRef.current || connectionStatus !== 'connected') return

    // Create Monaco editor with LSP integration
    editorInstanceRef.current = setupMonacoWithLSP(editorRef.current, lspClientRef.current!, {
      value: 'SELECT * FROM users WHERE id = 1;',
    })

    // Set up diagnostics on content change
    const model = editorInstanceRef.current.getModel()
    if (model) {
      const updateDiagnostics = async () => {
        if (!lspClientRef.current?.isConnected()) return

        const text = model.getValue()
        try {
          const diagnostics = await lspClientRef.current.diagnostics(text)
          updateDiagnosticsMonaco(editorInstanceRef.current!, diagnostics)
        } catch (error) {
          console.error('[App] Diagnostics error:', error)
        }
      }

      // Initial diagnostics
      updateDiagnostics()

      // Update on content changes with debounce
      let timeout: ReturnType<typeof setTimeout>
      model.onDidChangeContent(() => {
        clearTimeout(timeout)
        timeout = setTimeout(updateDiagnostics, 500)
      })
    }

    return () => {
      editorInstanceRef.current?.dispose()
    }
  }, [connectionStatus])

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <header style={{ padding: '1rem', borderBottom: '1px solid #30363d', background: '#0d1117' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <h1 style={{ color: '#c9d1d9', fontSize: '1.5rem', margin: 0 }}>
            Unified SQL LSP Playground
            {connectionStatus === 'connecting' && (
              <span style={{ fontSize: '0.875rem', marginLeft: '1rem', color: '#d29922' }}>
                Connecting to LSP server...
              </span>
            )}
            {connectionStatus === 'connected' && (
              <span style={{ fontSize: '0.875rem', marginLeft: '1rem', color: '#3fb950' }}>
                ✓ Connected to LSP
              </span>
            )}
            {connectionStatus === 'disconnected' && !connectionError && (
              <span style={{ fontSize: '0.875rem', marginLeft: '1rem', color: '#f85149' }}>
                ⚠ LSP server not running
              </span>
            )}
            {connectionError && (
              <span style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                <span
                  style={{ fontSize: '0.875rem', marginLeft: '1rem', color: '#f85149' }}
                  title={connectionError}
                >
                  ✗ Connection failed
                </span>
                <button
                  onClick={() => window.location.reload()}
                  style={{
                    padding: '0.25rem 0.75rem',
                    background: '#238636',
                    border: 'none',
                    borderRadius: '4px',
                    color: '#ffffff',
                    fontSize: '0.75rem',
                    cursor: 'pointer',
                  }}
                  onMouseEnter={(e) => e.currentTarget.style.background = '#2ea043'}
                  onMouseLeave={(e) => e.currentTarget.style.background = '#238636'}
                >
                  Retry
                </button>
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
