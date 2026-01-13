#!/bin/bash

# E2E Test Environment Teardown Script
# This script stops and cleans up database containers and volumes

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DOCKER_COMPOSE_FILE="$PROJECT_ROOT/docker-compose.yml"

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

cd "$PROJECT_ROOT"

# Parse command line arguments
CLEAN_VOLUMES=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --clean-volumes)
            CLEAN_VOLUMES=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --clean-volumes    Remove data volumes (WARNING: This will delete all test data)"
            echo "  -h, --help         Show this help message"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

log_info "Stopping database services..."
docker-compose down

if [ "$CLEAN_VOLUMES" = true ]; then
    log_warn "Removing data volumes..."
    docker-compose down -v
    log_info "Data volumes removed"
else
    log_info "Data volumes preserved (use --clean-volumes to remove them)"
fi

log_info "Teardown complete!"
