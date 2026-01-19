#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}╔═══════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║   Unified SQL LSP Playground - Start Script          ║${NC}"
echo -e "${GREEN}╚═══════════════════════════════════════════════════════╝${NC}"
echo ""

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKTREE_ROOT="$(dirname "$SCRIPT_DIR")"

# Check if we're in a worktree
if [ -f "$WORKTREE_ROOT/Cargo.toml" ]; then
  cd "$WORKTREE_ROOT"
else
  # Try current directory
  if [ -f "Cargo.toml" ]; then
    WORKTREE_ROOT="$(pwd)"
  else
    echo -e "${RED}✗ Error: Cannot find project root${NC}"
    exit 1
  fi
fi

# Function to cleanup on exit
cleanup() {
  echo ""
  echo -e "${YELLOW}─────────────────────────────────────────────────${NC}"
  echo -e "${YELLOW}Shutting down...${NC}"

  # Kill LSP server if running
  if [ -n "$LSP_PID" ] && kill -0 $LSP_PID 2>/dev/null; then
    echo -e "${GREEN}  → Stopping LSP server (PID: $LSP_PID)${NC}"
    kill $LSP_PID 2>/dev/null || true
    wait $LSP_PID 2>/dev/null || true
  fi

  echo -e "${GREEN}✓ Cleanup complete${NC}"
  echo -e "${YELLOW}─────────────────────────────────────────────────${NC}"
}

# Trap EXIT and INT signals
trap cleanup EXIT INT

# Check if required commands are available
if ! command -v cargo &> /dev/null; then
  echo -e "${RED}✗ Error: cargo not found. Please install Rust.${NC}"
  exit 1
fi

if ! command -v pnpm &> /dev/null; then
  echo -e "${RED}✗ Error: pnpm not found. Please install pnpm.${NC}"
  exit 1
fi

# Start LSP server in background
echo -e "${GREEN}[1/2] Starting LSP server on TCP port 4137...${NC}"
cargo run --bin unified-sql-lsp -- --tcp 4137 --catalog playground > /tmp/lsp-server.log 2>&1 &
LSP_PID=$!

# Wait for LSP server to start
sleep 3

# Check if LSP server is running
if ! kill -0 $LSP_PID 2>/dev/null; then
  echo -e "${RED}✗ Failed to start LSP server${NC}"
  echo -e "${RED}  Check log file: /tmp/lsp-server.log${NC}"
  cat /tmp/lsp-server.log
  exit 1
fi

echo -e "${GREEN}  ✓ LSP server started (PID: $LSP_PID)${NC}"
echo -e "     → Listening on: ${YELLOW}ws://localhost:4137${NC}"
echo ""

# Start playground dev server
echo -e "${GREEN}[2/2] Starting playground dev server...${NC}"
cd playground

# Check if node_modules exists
if [ ! -d "node_modules" ]; then
  echo -e "${YELLOW}  → Installing dependencies...${NC}"
  pnpm install --silent
fi

echo -e "${GREEN}  → Opening browser at: http://localhost:5173${NC}"
echo ""
echo -e "${YELLOW}─────────────────────────────────────────────────${NC}"
echo -e "${GREEN}✓ Playground is ready!${NC}"
echo -e "${YELLOW}─────────────────────────────────────────────────${NC}"
echo ""
echo -e "Press ${YELLOW}Ctrl+C${NC} to stop both servers"
echo ""

# Run playground dev server
pnpm run dev
