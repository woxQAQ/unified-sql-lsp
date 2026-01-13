#!/bin/bash

# E2E Test Environment Healthcheck Script
# This script tests database connectivity and returns 0/1 status code

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Color output (disabled for healthcheck as it should be machine-readable)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

VERBOSE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -v, --verbose    Show detailed healthcheck output"
            echo "  -h, --help       Show this help message"
            echo ""
            echo "Exit codes:"
            echo "  0  All services are healthy"
            echo "  1  One or more services are unhealthy"
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

log_verbose() {
    if [ "$VERBOSE" = true ]; then
        echo -e "${GREEN}[HEALTHCHECK]${NC} $1"
    fi
}

# Function to check MySQL health
check_mysql() {
    local container_name="unified-sql-lsp-mysql"
    local host="127.0.0.1"
    local port="3307"
    local user="root"
    local password="root_password"

    # Check if container is running
    if ! docker ps | grep -q "$container_name"; then
        log_verbose "MySQL container is not running"
        return 1
    fi

    # Check if we can connect to MySQL
    if docker exec "$container_name" mysqladmin ping -h localhost -u "$user" -p"$password" --silent; then
        log_verbose "MySQL is healthy"
        return 0
    else
        log_verbose "MySQL is not responding"
        return 1
    fi
}

# Function to check PostgreSQL health (reserved for future use)
check_postgresql() {
    # Reserved for future PostgreSQL health checks
    return 0
}

# Main healthcheck logic
ALL_HEALTHY=true

# Check MySQL
if ! check_mysql; then
    ALL_HEALTHY=false
fi

# Check PostgreSQL (when enabled)
# if ! check_postgresql; then
#     ALL_HEALTHY=false
# fi

# Return exit code
if [ "$ALL_HEALTHY" = true ]; then
    if [ "$VERBOSE" = true ]; then
        echo "All services are healthy"
    fi
    exit 0
else
    if [ "$VERBOSE" = true ]; then
        echo "One or more services are unhealthy" >&2
    fi
    exit 1
fi
