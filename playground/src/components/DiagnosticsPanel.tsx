import { useEffect, useState } from 'react'
import * as monaco from 'monaco-editor'

interface DiagnosticItem {
  severity: string
  message: string
  line: number
  col: number
  startLineNumber: number
  startColumn: number
  endLineNumber: number
  endColumn: number
}

interface DiagnosticsPanelProps {
  editor: monaco.editor.IStandaloneCodeEditor | null
}

export function DiagnosticsPanel({ editor }: DiagnosticsPanelProps) {
  const [isCollapsed, setIsCollapsed] = useState(false)
  const [diagnostics, setDiagnostics] = useState<DiagnosticItem[]>([])
  const [activeLine, setActiveLine] = useState<number | null>(null)

  useEffect(() => {
    if (!editor) return

    const model = editor.getModel()
    if (!model) return

    const updateDiagnostics = () => {
      const markers = monaco.editor.getModelMarkers({ resource: model.uri })
      const items: DiagnosticItem[] = markers.map((marker) => ({
        severity: marker.severity === 8 ? 'error' : 'warning',
        message: marker.message,
        line: marker.startLineNumber,
        col: marker.startColumn,
        startLineNumber: marker.startLineNumber,
        startColumn: marker.startColumn,
        endLineNumber: marker.endLineNumber,
        endColumn: marker.endColumn,
      }))
      setDiagnostics(items)
    }

    const contentChangeDisposable = model.onDidChangeContent(() => {
      updateDiagnostics()
    })

    const cursorChangeDisposable = editor.onDidChangeCursorPosition((e) => {
      setActiveLine(e.position.lineNumber)
    })

    updateDiagnostics()

    return () => {
      contentChangeDisposable.dispose()
      cursorChangeDisposable.dispose()
    }
  }, [editor])

  const errorCount = diagnostics.filter(d => d.severity === 'error').length
  const warningCount = diagnostics.filter(d => d.severity === 'warning').length

  const handleDiagnosticClick = (item: DiagnosticItem) => {
    if (!editor) return

    editor.revealLineInCenter(item.line)
    editor.setPosition({
      lineNumber: item.line,
      column: item.col,
    })
  }

  if (!editor) return null

  return (
    <div
      style={{
        borderTop: '1px solid #30363d',
        background: '#0d1117',
        transition: 'max-height 0.3s ease',
        maxHeight: isCollapsed ? '40px' : '240px',
        overflow: 'hidden',
        display: 'flex',
        flexDirection: 'column',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          padding: '0.5rem 1rem',
          background: '#161b22',
          borderBottom: '1px solid #30363d',
          cursor: 'pointer',
          userSelect: 'none',
        }}
        onClick={() => setIsCollapsed(!isCollapsed)}
      >
        <span
          style={{
            marginRight: '0.5rem',
            fontSize: '0.75rem',
            transition: 'transform 0.2s ease',
            transform: isCollapsed ? 'rotate(-90deg)' : 'rotate(0deg)',
          }}
        >
          ▼
        </span>
        <span style={{ color: '#c9d1d9', fontWeight: 600, fontSize: '0.875rem' }}>
          Diagnostics
        </span>
        <span style={{ flex: 1 }} />
        {errorCount > 0 && (
          <span
            style={{
              color: '#f85149',
              fontSize: '0.75rem',
              fontWeight: 500,
              marginRight: '0.5rem',
            }}
          >
            {errorCount} error{errorCount !== 1 ? 's' : ''}
          </span>
        )}
        {warningCount > 0 && (
          <span
            style={{
              color: '#d29922',
              fontSize: '0.75rem',
              fontWeight: 500,
            }}
          >
            {warningCount} warning{warningCount !== 1 ? 's' : ''}
          </span>
        )}
      </div>
      <div
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: '0.5rem',
        }}
      >
        {diagnostics.length === 0 ? (
          <div
            style={{
              padding: '1rem',
              textAlign: 'center',
              color: '#8b949e',
              fontSize: '0.875rem',
            }}
          >
            ✓ No diagnostics - your SQL looks good!
          </div>
        ) : (
          <div>
            {diagnostics.map((item, index) => (
              <div
                key={index}
                onClick={() => handleDiagnosticClick(item)}
                style={{
                  padding: '0.5rem 0.75rem',
                  marginBottom: '0.25rem',
                  borderRadius: '4px',
                  cursor: 'pointer',
                  display: 'flex',
                  alignItems: 'flex-start',
                  gap: '0.5rem',
                  background:
                    activeLine === item.line ? '#161b22' : 'transparent',
                  borderLeft:
                    activeLine === item.line
                      ? `2px solid ${item.severity === 'error' ? '#f85149' : '#d29922'}`
                      : '2px solid transparent',
                  transition: 'background 0.15s ease, border-color 0.15s ease',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = '#21262d'
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background =
                    activeLine === item.line ? '#161b22' : 'transparent'
                }}
              >
                {item.severity === 'error' ? (
                  <svg
                    width="16"
                    height="16"
                    viewBox="0 0 16 16"
                    fill="none"
                    style={{ flexShrink: 0, marginTop: '2px' }}
                  >
                    <circle cx="8" cy="8" r="7" fill="#f85149" fillOpacity={0.2} />
                    <circle cx="8" cy="8" r="7" stroke="#f85149" strokeWidth={1} />
                    <text
                      x="8"
                      y="11"
                      textAnchor="middle"
                      fill="#f85149"
                      fontSize="10"
                      fontWeight="bold"
                    >
                      ✕
                    </text>
                  </svg>
                ) : (
                  <svg
                    width="16"
                    height="16"
                    viewBox="0 0 16 16"
                    fill="none"
                    style={{ flexShrink: 0, marginTop: '2px' }}
                  >
                    <circle cx="8" cy="8" r="7" fill="#d29922" fillOpacity={0.2} />
                    <circle cx="8" cy="8" r="7" stroke="#d29922" strokeWidth={1} />
                    <text
                      x="8"
                      y="12"
                      textAnchor="middle"
                      fill="#d29922"
                      fontSize="10"
                      fontWeight="bold"
                    >
                      !
                    </text>
                  </svg>
                )}
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div
                    style={{
                      color:
                        item.severity === 'error' ? '#f85149' : '#d29922',
                      fontSize: '0.875rem',
                      fontWeight: 500,
                      wordBreak: 'break-word',
                    }}
                  >
                    {item.message}
                  </div>
                  <div
                    style={{
                      color: '#8b949e',
                      fontSize: '0.75rem',
                      marginTop: '0.25rem',
                    }}
                  >
                    Line {item.line}, Column {item.col}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
