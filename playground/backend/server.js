import express from 'express'
import { WebSocketServer, WebSocket } from 'ws'
import { createServer } from 'http'
import cors from 'cors'
import { spawn } from 'child_process'
import path from 'path'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const app = express()
const server = createServer(app)

app.use(cors())
app.use(express.json())

// Environment configuration
const MYSQL_PORT = process.env.MYSQL_PORT || '3307'
const PG_PORT = process.env.PG_PORT || '5433'
const MYSQL_HOST = process.env.MYSQL_HOST || 'localhost'
const PG_HOST = process.env.PG_HOST || 'localhost'

// Store active LSP processes per client
const lspProcesses = new Map()

function getConnectionString(dialect) {
  if (dialect === 'mysql') {
    return `mysql://lsp_user:lsp_password@${MYSQL_HOST}:${MYSQL_PORT}/northwind`
  } else if (dialect === 'postgresql') {
    return `postgresql://lsp_user:lsp_password@${PG_HOST}:${PG_PORT}/northwind`
  }
  return null
}

function startLSPProcess(dialect, ws) {
  const projectRoot = path.resolve(__dirname, '../../..')
  const lspPath = path.join(projectRoot, 'target/release/unified-sql-lsp')

  const connectionString = getConnectionString(dialect)

  console.log(`Starting LSP process for dialect: ${dialect}`)
  console.log(`LSP path: ${lspPath}`)
  console.log(`Connection: ${connectionString}`)

  const env = {
    ...process.env,
    RUST_LOG: 'debug',
    RUST_BACKTRACE: '1',
  }

  // Pass connection string via environment that LSP can read
  if (connectionString) {
    env.UNIFIED_SQL_LSP_CONNECTION = connectionString
    env.UNIFIED_SQL_LSP_DIALECT = dialect
  }

  const lsp = spawn(lspPath, [], {
    stdio: ['pipe', 'pipe', 'pipe'],
    env
  })

  let buffer = ''

  lsp.on('error', (err) => {
    console.error(`LSP process error for ${dialect}:`, err)
    ws.send(JSON.stringify({
      jsonrpc: '2.0',
      method: 'error',
      error: {
        code: -32700,
        message: `LSP process error: ${err.message}`
      }
    }))
  })

  lsp.on('exit', (code, signal) => {
    console.log(`LSP process exited for ${dialect}: code=${code}, signal=${signal}`)
  })

  lsp.stderr.on('data', (data) => {
    console.error(`LSP stderr [${dialect}]:`, data.toString())
  })

  // Handle LSP stdout with Content-Length header parsing
  lsp.stdout.on('data', (data) => {
    buffer += data.toString()

    // Process complete messages
    while (true) {
      // Look for Content-Length header
      const lengthMatch = buffer.match(/Content-Length: (\d+)\r\n\r\n/)
      if (!lengthMatch) {
        break // No complete message yet
      }

      const contentLength = parseInt(lengthMatch[1], 10)
      const headerEnd = buffer.indexOf('\r\n\r\n') + 4
      const messageStart = headerEnd

      // Check if we have the full message
      if (buffer.length < messageStart + contentLength) {
        break // Message not complete yet
      }

      // Extract the JSON message
      const jsonStr = buffer.substring(messageStart, messageStart + contentLength)
      buffer = buffer.substring(messageStart + contentLength)

      try {
        const json = JSON.parse(jsonStr)
        console.log(`LSP -> WebSocket [${dialect}]:`, JSON.stringify(json).substring(0, 100) + '...')
        ws.send(jsonStr)
      } catch (err) {
        console.error(`Failed to parse LSP message:`, err)
        console.error(`Raw message:`, jsonStr)
      }
    }
  })

  return lsp
}

const wss = new WebSocketServer({ server, path: '/lsp' })

wss.on('connection', (ws, req) => {
  const url = new URL(req.url || '', `http://${req.headers.host}`)
  const dialect = url.searchParams.get('dialect') || 'mysql'

  console.log(`WebSocket connected for dialect: ${dialect}`)

  // Start LSP process for this connection
  const lsp = startLSPProcess(dialect, ws)
  lspProcesses.set(ws, { lsp, dialect })

  // Forward messages from WebSocket to LSP process
  ws.on('message', (message) => {
    const msgStr = message.toString()
    console.log(`WebSocket -> LSP [${dialect}]:`, msgStr.substring(0, 100) + '...')

    try {
      // Write Content-Length header + message
      const buffer = Buffer.from(msgStr)
      const header = `Content-Length: ${buffer.length}\r\n\r\n`
      lsp.stdin.write(header)
      lsp.stdin.write(buffer)
    } catch (err) {
      console.error(`Error writing to LSP stdin:`, err)
    }
  })

  ws.on('close', () => {
    console.log(`WebSocket disconnected for dialect: ${dialect}`)
    const proc = lspProcesses.get(ws)
    if (proc) {
      proc.lsp.kill()
      lspProcesses.delete(ws)
    }
  })

  ws.on('error', (err) => {
    console.error(`WebSocket error:`, err)
  })
})

// Health check endpoint
app.get('/health', (req, res) => {
  res.json({
    status: 'ok',
    connections: lspProcesses.size,
    mysql: `mysql://lsp_user:****@${MYSQL_HOST}:${MYSQL_PORT}/northwind`,
    postgres: `postgresql://lsp_user:****@${PG_HOST}:${PG_PORT}/northwind`,
    timestamp: new Date().toISOString()
  })
})

// Get catalog info
app.get('/api/catalog', async (req, res) => {
  const { dialect = 'mysql' } = req.query

  // Query the actual database catalog
  try {
    let result
    if (dialect === 'mysql') {
      // This would query MySQL directly
      // For now, return known schema
      result = {
        dialect: 'mysql',
        database: 'northwind',
        tables: [
          { name: 'customers', columns: ['customer_id', 'company_name', 'contact_name', 'contact_title', 'address', 'city', 'region', 'postal_code', 'country', 'phone', 'fax'] },
          { name: 'employees', columns: ['employee_id', 'last_name', 'first_name', 'title', 'birth_date', 'hire_date'] },
          { name: 'categories', columns: ['category_id', 'category_name', 'description'] },
          { name: 'products', columns: ['product_id', 'product_name', 'supplier_id', 'category_id', 'unit_price', 'units_in_stock'] },
          { name: 'shippers', columns: ['shipper_id', 'company_name', 'phone'] },
          { name: 'orders', columns: ['order_id', 'customer_id', 'employee_id', 'order_date', 'required_date', 'shipped_date', 'ship_via', 'freight'] },
          { name: 'order_details', columns: ['order_id', 'product_id', 'unit_price', 'quantity', 'discount'] }
        ]
      }
    } else {
      result = {
        dialect: 'postgresql',
        database: 'northwind',
        tables: [
          { name: 'customers', columns: ['customer_id', 'company_name', 'contact_name', 'contact_title', 'address', 'city', 'region', 'postal_code', 'country', 'phone', 'fax'] },
          { name: 'employees', columns: ['employee_id', 'last_name', 'first_name', 'title', 'birth_date', 'hire_date'] },
          { name: 'categories', columns: ['category_id', 'category_name', 'description'] },
          { name: 'products', columns: ['product_id', 'product_name', 'supplier_id', 'category_id', 'unit_price', 'units_in_stock'] },
          { name: 'shippers', columns: ['shipper_id', 'company_name', 'phone'] },
          { name: 'orders', columns: ['order_id', 'customer_id', 'employee_id', 'order_date', 'required_date', 'shipped_date', 'ship_via', 'freight'] },
          { name: 'order_details', columns: ['order_id', 'product_id', 'unit_price', 'quantity', 'discount'] }
        ]
      }
    }
    res.json(result)
  } catch (error) {
    res.status(500).json({ error: error.message })
  }
})

const PORT = process.env.PORT || 8080

server.listen(PORT, () => {
  console.log(`Backend server listening on port ${PORT}`)
  console.log(`WebSocket endpoint: ws://localhost:${PORT}/lsp`)
  console.log(`HTTP endpoint: http://localhost:${PORT}/health`)
  console.log(`MySQL: ${MYSQL_HOST}:${MYSQL_PORT}`)
  console.log(`PostgreSQL: ${PG_HOST}:${PG_PORT}`)
})
