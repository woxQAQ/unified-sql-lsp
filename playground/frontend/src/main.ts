import * as monaco from 'monaco-editor'

let editor: monaco.editor.IStandaloneCodeEditor | null = null
let currentDialect: string = 'mysql'
let catalog: any = null

const statusElement = document.getElementById('status')!
const dialectSelect = document.getElementById('dialect') as HTMLSelectElement

function updateStatus(status: 'connected' | 'disconnected' | 'connecting') {
  statusElement.className = status
  statusElement.textContent = status.charAt(0).toUpperCase() + status.slice(1)
}

async function fetchCatalog(dialect: string) {
  try {
    const response = await fetch(`http://localhost:8080/api/catalog?dialect=${dialect}`)
    if (response.ok) {
      catalog = await response.json()
      console.log('Catalog loaded:', catalog)
      updateStatus('connected')
      return true
    }
  } catch (error) {
    console.error('Failed to fetch catalog:', error)
  }
  updateStatus('disconnected')
  return false
}

async function connectToLSP(dialect: string) {
  currentDialect = dialect
  updateStatus('connecting')
  await fetchCatalog(dialect)
}

function extractTableAliases(text: string): Map<string, string> {
  const aliases = new Map<string, string>()

  // Match FROM table alias or JOIN table alias patterns
  // Supports: FROM table alias, FROM table AS alias, JOIN table alias, JOIN table AS alias
  const fromPattern = /(?:FROM|JOIN|INNER\s+JOIN|LEFT\s+JOIN|RIGHT\s+JOIN)\s+(\w+)\s+(?:AS\s+)?(\w+)/gi
  let match

  while ((match = fromPattern.exec(text)) !== null) {
    const tableName = match[1]
    const aliasName = match[2]
    aliases.set(aliasName.toLowerCase(), tableName.toLowerCase())
  }

  return aliases
}

function resolveTableName(tableOrAlias: string, aliases: Map<string, string>): string | null {
  const lowerTableOrAlias = tableOrAlias.toLowerCase()

  // First check if it's a direct table name
  const directMatch = catalog.tables.find((t: any) => t.name.toLowerCase() === lowerTableOrAlias)
  if (directMatch) {
    return directMatch.name
  }

  // Then check if it's an alias
  const resolvedTableName = aliases.get(lowerTableOrAlias)
  if (resolvedTableName) {
    return resolvedTableName
  }

  return null
}

function parseContext(text: string) {
  const lines = text.split('\n')
  const currentLine = lines[lines.length - 1]

  // Extract table aliases from the entire query
  const aliases = extractTableAliases(text)

  // Check if we're in FROM clause
  const fromMatch = currentLine.match(/FROM\s+(\w*)$/i)
  if (fromMatch) {
    return { type: 'table', prefix: fromMatch[1] }
  }

  // Check if we're in JOIN clause
  const joinMatch = currentLine.match(/(?:JOIN|INNER\s+JOIN|LEFT\s+JOIN|RIGHT\s+JOIN)\s+(\w*)$/i)
  if (joinMatch) {
    return { type: 'table', prefix: joinMatch[1] }
  }

  // Check if we're doing qualified column completion (table.column or alias.column)
  const qualifiedMatch = currentLine.match(/(\w+)\.(\w*)$/)
  if (qualifiedMatch) {
    const tableOrAlias = qualifiedMatch[1]
    const resolvedTableName = resolveTableName(tableOrAlias, aliases)

    return {
      type: 'column',
      table: resolvedTableName,
      tableOrAlias: tableOrAlias,
      prefix: qualifiedMatch[2]
    }
  }

  // Check if we're in SELECT clause (column completion)
  const selectMatch = currentLine.match(/SELECT\s+(.*?)$/i)
  if (selectMatch) {
    const afterSelect = selectMatch[1]
    // Only suggest columns if we're not in a table name context
    if (!afterSelect.match(/\bFROM\b/i)) {
      return { type: 'column', table: null, prefix: afterSelect.split(/\s+/).pop() || '' }
    }
  }

  // Check if we're in WHERE clause
  const whereMatch = text.match(/WHERE\s+[\s\S]*$/i)
  if (whereMatch) {
    return { type: 'column', table: null, prefix: currentLine.split(/\s+/).pop() || '' }
  }

  return null
}

function initEditor() {
  editor = monaco.editor.create(document.getElementById('editor')!, {
    value: '-- Start typing SQL queries here\n-- SELECT * FROM ',
    language: 'sql',
    theme: 'vs-dark',
    automaticLayout: true,
    fontSize: 14,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    wordWrap: 'on',
    suggest: {
      showKeywords: true,
      showSnippets: true
    }
  })

  // Register SQL language
  monaco.languages.register({
    id: 'sql',
    extensions: ['.sql'],
    aliases: ['SQL', 'sql'],
    mimetypes: ['text/x-sql']
  })

  // Register completion provider
  monaco.languages.registerCompletionItemProvider('sql', {
    provideCompletionItems: async (model, position) => {
      const textUntilPosition = model.getValueInRange({
        startLineNumber: 1,
        startColumn: 1,
        endLineNumber: position.lineNumber,
        endColumn: position.column
      })

      console.log('Completion requested at position:', position)
      console.log('Text until position:', textUntilPosition)

      const suggestions: monaco.languages.CompletionItem[] = []

      if (!catalog) {
        return { suggestions }
      }

      const context = parseContext(textUntilPosition)
      console.log('Parsed context:', context)

      if (context?.type === 'table') {
        // Table completion
        catalog.tables.forEach((table: any) => {
          if (table.name.toLowerCase().startsWith(context.prefix.toLowerCase())) {
            suggestions.push({
              label: table.name,
              kind: monaco.languages.CompletionItemKind.Class,
              insertText: table.name,
              detail: 'Table',
              documentation: `Columns: ${table.columns.join(', ')}`
            })
          }
        })
      } else if (context?.type === 'column') {
        // Column completion
        if (context.table) {
          // Qualified column completion (table.column)
          const table = catalog.tables.find((t: any) => t.name.toLowerCase() === context.table?.toLowerCase())
          if (table) {
            table.columns.forEach((column: string) => {
              if (column.toLowerCase().startsWith(context.prefix.toLowerCase())) {
                suggestions.push({
                  label: column,
                  kind: monaco.languages.CompletionItemKind.Field,
                  insertText: column,
                  detail: `Column from ${table.name}`
                })
              }
            })
          }
        } else {
          // Unqualified column completion - suggest columns from all tables
          const allColumns = new Map<string, string>()
          catalog.tables.forEach((table: any) => {
            table.columns.forEach((column: string) => {
              allColumns.set(column, table.name)
            })
          })

          allColumns.forEach((table, column) => {
            if (column.toLowerCase().startsWith(context.prefix.toLowerCase())) {
              suggestions.push({
                label: column,
                kind: monaco.languages.CompletionItemKind.Field,
                insertText: column,
                detail: `Column from ${table}`
              })
            }
          })
        }
      }

      return { suggestions }
    }
  })
}

// Initialize on load
document.addEventListener('DOMContentLoaded', () => {
  initEditor()
  connectToLSP(dialectSelect.value)

  // Handle dialect change
  dialectSelect.addEventListener('change', () => {
    connectToLSP(dialectSelect.value)
  })
})

// Handle window resize
window.addEventListener('resize', () => {
  editor?.layout()
})
