#!/bin/bash
set -e

echo "🏗️  Unified SQL LSP Playground - Setup & Start"
echo "=============================================="

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

# Step 1: Build Rust LSP server
echo ""
echo "${YELLOW}Step 1: Building Rust LSP server...${NC}"
if [ ! -f "target/release/unified-sql-lsp" ]; then
    cargo build --release
    echo "${GREEN}✓ LSP server built successfully${NC}"
else
    echo "${GREEN}✓ LSP server already built${NC}"
fi

# Step 2: Install backend dependencies
echo ""
echo "${YELLOW}Step 2: Installing backend dependencies...${NC}"
cd playground/backend
if [ ! -d "node_modules" ]; then
    npm install
    echo "${GREEN}✓ Backend dependencies installed${NC}"
else
    echo "${GREEN}✓ Backend dependencies already installed${NC}"
fi

# Step 3: Install frontend dependencies
echo ""
echo "${YELLOW}Step 3: Installing frontend dependencies...${NC}"
cd ../frontend
if [ ! -d "node_modules" ]; then
    npm install
    echo "${GREEN}✓ Frontend dependencies installed${NC}"
else
    echo "${GREEN}✓ Frontend dependencies already installed${NC}"
fi

cd "$PROJECT_ROOT"

# Step 4: Start Docker services (databases)
echo ""
echo "${YELLOW}Step 4: Starting Docker services...${NC}"
cd playground
docker-compose up -d mysql postgres

# Wait for databases to be healthy
echo "${YELLOW}Waiting for databases to be ready...${NC}"
timeout=60
while [ $timeout -gt 0 ]; do
    if docker-compose ps | grep -q "healthy"; then
        echo "${GREEN}✓ Databases are ready${NC}"
        break
    fi
    sleep 2
    timeout=$((timeout - 2))
done

if [ $timeout -le 0 ]; then
    echo "${RED}✗ Databases failed to start${NC}"
    docker-compose logs
    exit 1
fi

# Step 5: Start backend server
echo ""
echo "${YELLOW}Step 5: Starting backend server...${NC}"
cd backend
mkdir -p logs
NODE_ENV=development MYSQL_PORT=3307 PG_PORT=5433 npm start > logs/server.log 2>&1 &
BACKEND_PID=$!
echo $BACKEND_PID > .backend.pid
echo "${GREEN}✓ Backend server started (PID: $BACKEND_PID)${NC}"

# Wait for backend to be ready
sleep 3
if curl -s http://localhost:8080/health > /dev/null; then
    echo "${GREEN}✓ Backend is responding${NC}"
else
    echo "${RED}✗ Backend failed to start${NC}"
    cat logs/server.log
    exit 1
fi

# Step 6: Start frontend dev server
echo ""
echo "${YELLOW}Step 6: Starting frontend dev server...${NC}"
cd ../frontend
npm run dev > ../backend/logs/frontend.log 2>&1 &
FRONTEND_PID=$!
echo $FRONTEND_PID > ../backend/.frontend.pid
echo "${GREEN}✓ Frontend dev server started (PID: $FRONTEND_PID)${NC}"

# Final message
echo ""
echo "${GREEN}==============================================${NC}"
echo "${GREEN}✓ Playground is now running!${NC}"
echo ""
echo "  Frontend:     http://localhost:3000"
echo "  Backend:      http://localhost:8080"
echo "  MySQL:        localhost:3307"
echo "  PostgreSQL:   localhost:5433"
echo ""
echo "Press Ctrl+C to stop all services"
echo "Or run: ./stop.sh"
echo ""

# Handle shutdown
cleanup() {
    echo ""
    echo "${YELLOW}Stopping services...${NC}"

    if [ -f "playground/backend/.frontend.pid" ]; then
        kill $(cat playground/backend/.frontend.pid) 2>/dev/null || true
        rm playground/backend/.frontend.pid
    fi

    if [ -f "playground/backend/.backend.pid" ]; then
        kill $(cat playground/backend/.backend.pid) 2>/dev/null || true
        rm playground/backend/.backend.pid
    fi

    cd playground
    docker-compose down

    echo "${GREEN}✓ All services stopped${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Keep script running
wait
