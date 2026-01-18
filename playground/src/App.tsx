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
