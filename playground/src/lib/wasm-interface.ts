// WASM interface with JavaScript fallback
let wasmInstance: any = null
let initPromise: Promise<any> | null = null

// Mock LSP Server implementation in JavaScript
class MockLspServer {
  constructor(_dialect: string) {}

  completion(text: string, _line: number, _col: number): string {
    const items = [
      {
        label: 'SELECT',
        kind: 14, // Keyword
        detail: 'Keyword',
        documentation: 'Retrieves data from one or more tables',
        insertText: 'SELECT ',
      },
      {
        label: 'FROM',
        kind: 14,
        detail: 'Keyword',
        documentation: 'Specifies the table to query from',
        insertText: 'FROM ',
      },
      {
        label: 'WHERE',
        kind: 14,
        detail: 'Keyword',
        documentation: 'Filters rows based on a condition',
        insertText: 'WHERE ',
      },
      {
        label: 'users',
        kind: 5, // Field
        detail: 'Table',
        documentation: 'User accounts table',
        insertText: 'users',
      },
      {
        label: 'orders',
        kind: 5,
        detail: 'Table',
        documentation: 'Customer orders table',
        insertText: 'orders',
      },
      {
        label: 'order_items',
        kind: 5,
        detail: 'Table',
        documentation: 'Order line items table',
        insertText: 'order_items',
      },
      {
        label: 'id',
        kind: 5,
        detail: 'INT',
        documentation: 'Primary key column',
        insertText: 'id',
      },
      {
        label: 'name',
        kind: 5,
        detail: 'VARCHAR(100)',
        documentation: 'User name column',
        insertText: 'name',
      },
      {
        label: 'email',
        kind: 5,
        detail: 'VARCHAR(255)',
        documentation: 'User email column',
        insertText: 'email',
      },
      {
        label: 'created_at',
        kind: 5,
        detail: 'TIMESTAMP',
        documentation: 'Creation timestamp column',
        insertText: 'created_at',
      },
    ]

    // Context-aware: suggest FROM after SELECT
    if (text.includes('SELECT') && !text.includes('FROM')) {
      items.push({
        label: 'FROM',
        kind: 14,
        detail: 'Keyword',
        documentation: 'Specifies the table to query from',
        insertText: '\nFROM ',
      })
    }

    return JSON.stringify(items)
  }

  hover(_text: string, _line: number, _col: number): string {
    return JSON.stringify({
      contents: {
        kind: 'markdown',
        value: '### SQL Element\n\nHover information for this SQL element.',
      },
    })
  }

  diagnostics(text: string): string {
    const diagnostics: any[] = []

    // Simple mock diagnostics
    if (text.includes('SELEC') && !text.includes('SELECT')) {
      diagnostics.push({
        severity: 1,
        message: "Did you mean 'SELECT'?",
        range: {
          start: { line: 0, character: 0 },
          end: { line: 0, character: 5 },
        },
      })
    }

    return JSON.stringify(diagnostics)
  }
}

export async function initWasm(dialect: string = 'mysql'): Promise<any> {
  if (wasmInstance) {
    return wasmInstance
  }

  if (initPromise) {
    return initPromise
  }

  initPromise = (async () => {
    // For now, always use JavaScript mock
    // TODO: Build and load WASM module when available
    console.log('Using JavaScript mock LSP implementation')
    wasmInstance = new MockLspServer(dialect)
    initPromise = null
    return wasmInstance
  })()

  return initPromise
}

export function getWasmInstance(): any {
  return wasmInstance
}
