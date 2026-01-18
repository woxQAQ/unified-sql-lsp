import * as monaco from 'monaco-editor'
import { getWasmInstance } from './wasm-interface'

// Extend LspServer interface with LSP methods
interface WasmLspServer {
  completion?(text: string, line: number, col: number): string
  hover?(text: string, line: number, col: number): string
  diagnostics?(text: string): string
}

export class LspBridge {
  // @ts-expect-error - Editor stored for future use (spec compliance)
  private editor: monaco.editor.IStandaloneCodeEditor
  private debounceTimer: ReturnType<typeof setTimeout> | null = null
  private disposables: monaco.IDisposable[] = []

  constructor(editor: monaco.editor.IStandaloneCodeEditor) {
    this.editor = editor
    this.setupProviders()
  }

  private setupProviders() {
    // Register completion provider
    const completionDisposable = monaco.languages.registerCompletionItemProvider('sql', {
      provideCompletionItems: async (model, position) => {
        const text = model.getValue()
        const line = position.lineNumber
        const col = position.column

        const wasm = getWasmInstance() as unknown as WasmLspServer | null
        if (!wasm || !wasm.completion) return { suggestions: [] }

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

    this.disposables.push(completionDisposable)

    // Register hover provider
    const hoverDisposable = monaco.languages.registerHoverProvider('sql', {
      provideHover: async (model, position) => {
        const text = model.getValue()
        const line = position.lineNumber
        const col = position.column

        const wasm = getWasmInstance() as unknown as WasmLspServer | null
        if (!wasm || !wasm.hover) return null

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

    this.disposables.push(hoverDisposable)
  }

  private convertCompletionItems(items: any[]): monaco.languages.CompletionItem[] {
    return items.map(item => {
      const completionItem: monaco.languages.CompletionItem = {
        label: item.label,
        kind: this.convertCompletionKind(item.kind),
        detail: item.detail,
        documentation: item.documentation,
        insertText: item.insertText || item.label,
        sortText: item.label,
        range: null as any, // Monaco will calculate
      }
      return completionItem
    })
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
    const wasm = getWasmInstance() as unknown as WasmLspServer | null
    if (!wasm || !wasm.diagnostics) return

    // Debounce diagnostics
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer)
    }

    this.debounceTimer = setTimeout(() => {
      const text = model.getValue()
      try {
        const result = wasm.diagnostics!(text)
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
    this.disposables.forEach((d) => d.dispose())
    this.disposables = []
  }
}
