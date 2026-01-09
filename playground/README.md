# Unified SQL LSP Playground

This is a web-based playground for testing the Unified SQL LSP server with real database connections.

## Prerequisites

- Docker and Docker Compose
- Node.js 20+ (for local development)
- Rust toolchain (for building the LSP server)

## Quick Start with Docker

### 1. Build the LSP Server

First, build the Rust LSP server binary:

```bash
cd /path/to/unified-sql-lsp
cargo build --release
```

This creates the binary at `target/release/unified-sql-lsp`.

### 2. Start All Services

```bash
cd playground
./start.sh
```

This starts:
- MySQL 8.0 on port 3307 (user: `lsp_user`, password: `lsp_password`)
- PostgreSQL 16 on port 5433 (user: `lsp_user`, password: `lsp_password`)
- Backend server on port 8080
- Frontend on port 3000

### 3. Access the Playground

Open your browser to: http://localhost:3000

## Database Schema

The databases are pre-populated with the Northwind sample dataset containing:

- **customers** (9 rows) - Customer information
- **employees** (6 rows) - Employee records
- **categories** (8 rows) - Product categories
- **products** (14 rows) - Product catalog
- **shippers** (3 rows) - Shipping companies
- **orders** (5 rows) - Customer orders
- **order_details** (5 rows) - Order line items

## Local Development (without Docker)

### Start Databases Only

```bash
cd playground
docker-compose up -d mysql postgres
```

### Start Backend

```bash
cd playground/backend
npm install
MYSQL_PORT=3307 PG_PORT=5433 npm start
```

The backend will be available at http://localhost:8080

### Start Frontend

```bash
cd playground/frontend
npm install
npm run dev
```

The frontend will be available at http://localhost:3000

## Testing LSP Features

1. Open the playground in your browser
2. Select a dialect (MySQL or PostgreSQL)
3. Try these queries:

```sql
-- Table completion
SELECT * FROM [Ctrl+Space]

-- Column completion
SELECT customer_id, [Ctrl+Space] FROM customers;

-- JOIN completion
SELECT * FROM orders o
JOIN customers c ON o.[Ctrl+Space]

-- Function completion
SELECT C[Ctrl+Space] FROM customers;
```

## Architecture

```
┌─────────────┐     HTTP/WebSocket      ┌──────────────┐
│   Browser   │ ←─────────────────────→ │  Backend     │
│ (Monaco)    │                         │  (Node.js)   │
└─────────────┘                         └──────┬───────┘
                                                │ stdio
                                                ↓
                                        ┌──────────────┐
                                        │ LSP Server   │
                                        │ (Rust)       │
                                        └──────┬───────┘
                                               │
                                        ┌──────┴───────┐
                                        │              │
                                   ┌────┴───┐    ┌───┴────┐
                                   │ MySQL  │    │Postgres│
                                   │ :3307  │    │ :5433  │
                                   └────────┘    └────────┘
```

## Troubleshooting

### Backend can't connect to databases

Wait for databases to be fully healthy:
```bash
docker-compose ps
```

### Frontend shows "Disconnected"

Check if the backend is running:
```bash
curl http://localhost:8080/health
```

### LSP not responding

Check the backend logs:
```bash
docker-compose logs backend
```

Or check backend logs:
```bash
cat playground/backend/logs/server.log
```

## Stopping Services

```bash
cd playground
./stop.sh
```

Or manually:

```bash
# Stop Node.js processes
pkill -f "node.*server.js"
pkill -f "vite"

# Stop databases
docker-compose down
```

To remove database data as well:
```bash
docker-compose down -v
```
