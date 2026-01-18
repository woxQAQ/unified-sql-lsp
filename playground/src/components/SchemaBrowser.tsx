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
