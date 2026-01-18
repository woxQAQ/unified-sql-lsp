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
