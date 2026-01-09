#!/bin/bash

echo "🛑 Stopping Unified SQL LSP Playground..."

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT/playground"

# Stop frontend and backend processes
if [ -f "backend/.frontend.pid" ]; then
    kill $(cat backend/.frontend.pid) 2>/dev/null || true
    rm backend/.frontend.pid
    echo "✓ Frontend stopped"
fi

if [ -f "backend/.backend.pid" ]; then
    kill $(cat backend/.backend.pid) 2>/dev/null || true
    rm backend/.backend.pid
    echo "✓ Backend stopped"
fi

# Stop Docker services
docker-compose down

echo "✓ All services stopped"
