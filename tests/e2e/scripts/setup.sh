#!/bin/bash

# E2E Test Environment Setup Script
# This script starts the database services and waits for them to be ready

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DOCKER_COMPOSE_FILE="$PROJECT_ROOT/docker-compose.yml"
HEALTHCHECK_SCRIPT="$SCRIPT_DIR/healthcheck.sh"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if docker-compose file exists
if [ ! -f "$DOCKER_COMPOSE_FILE" ]; then
    log_error "docker-compose.yml not found at $DOCKER_COMPOSE_FILE"
    exit 1
fi

# Check if healthcheck script exists
if [ ! -f "$HEALTHCHECK_SCRIPT" ]; then
    log_error "healthcheck.sh not found at $HEALTHCHECK_SCRIPT"
    exit 1
fi

# Make healthcheck executable
chmod +x "$HEALTHCHECK_SCRIPT"

log_info "Starting database services..."
cd "$PROJECT_ROOT"

# Start docker-compose services
docker-compose up -d

# Wait for services to be healthy
log_info "Waiting for services to be ready..."

MAX_RETRIES=30
RETRY_COUNT=0
SLEEP_INTERVAL=2

while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    if bash "$HEALTHCHECK_SCRIPT"; then
        log_info "All services are healthy and ready!"
        exit 0
    fi

    RETRY_COUNT=$((RETRY_COUNT + 1))
    echo -n "."
    sleep $SLEEP_INTERVAL
done

echo ""
log_error "Services failed to become healthy within expected time"
exit 1
