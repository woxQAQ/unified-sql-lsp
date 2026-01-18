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
