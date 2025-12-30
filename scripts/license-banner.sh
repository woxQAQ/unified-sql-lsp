#!/usr/bin/env bash
# license-banner.sh - Add or check license headers in source files
#
# Usage:
#   ./scripts/license-banner.sh check      # Check for missing headers
#   ./scripts/license-banner.sh add       # Add headers to files missing them
#   ./scripts/license-banner.sh --help    # Show this help message

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# License banner template
BANNER="// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details
"

# File extensions to check
RUST_EXTS=("rs")
# Add more extensions as needed: GO_EXTS=("go"), etc.

# Directories to exclude
EXCLUDE_DIRS=("target" "node_modules" ".git")

# Function to check if a directory should be excluded
should_exclude_dir() {
    local dir="$1"
    for exclude in "${EXCLUDE_DIRS[@]}"; do
        if [[ "$dir" == *"$exclude"* ]]; then
            return 0
        fi
    done
    return 1
}

# Function to check if a file has the license banner
has_banner() {
    local file="$1"
    local ext="${file##*.}"

    case "$ext" in
        rs)
            if head -1 "$file" | grep -q "Copyright (c) 2025 woxQAQ"; then
                return 0  # Has banner
            else
                return 1  # No banner
            fi
            ;;
        # Add more cases for other file types
        *)
            return 1  # Unknown file type, skip
            ;;
    esac
}

# Function to add banner to a file
add_banner() {
    local file="$1"
    local ext="${file##*.}"

    case "$ext" in
        rs)
            # Create temp file with banner + original content
            local tmpfile=$(mktemp)
            echo "$BANNER" > "$tmpfile"
            cat "$file" >> "$tmpfile"
            mv "$tmpfile" "$file"
            echo -e "${GREEN}✓${NC} Added banner to $file"
            ;;
        # Add more cases for other file types
        *)
            echo -e "${YELLOW}⊘${NC} Skipped $file (unsupported type)"
            ;;
    esac
}

# Function to process files
process_files() {
    local action="$1"  # "check" or "add"
    local missing=0

    # Find all source files
    while IFS= read -r -d '' file; do
        # Check if any parent directory is in exclude list
        local should_skip=false
        local dir_path=$(dirname "$file")
        while [[ "$dir_path" != "." ]]; do
            if should_exclude_dir "$dir_path"; then
                should_skip=true
                break
            fi
            dir_path=$(dirname "$dir_path")
        done

        if [[ "$should_skip" == "true" ]]; then
            continue
        fi

        if ! has_banner "$file"; then
            ((missing++))
            if [[ "$action" == "add" ]]; then
                add_banner "$file"
            else
                echo -e "${RED}✗${NC} Missing banner: $file"
            fi
        fi
    done < <(find . -type f \( -name "*.rs" \) -print0)

    return $missing
}

# Main script
main() {
    local action="${1:-check}"

    case "$action" in
        check)
            echo "Checking for license banners..."
            echo ""
            if process_files "check"; then
                echo ""
                echo -e "${GREEN}✓ All files have license banners!${NC}"
                exit 0
            else
                local count=$?
                echo ""
                echo -e "${RED}✗ $count file(s) missing license banner${NC}"
                echo "Run './scripts/license-banner.sh add' to add them"
                exit 1
            fi
            ;;
        add)
            echo "Adding license banners to files..."
            echo ""
            process_files "add"
            echo ""
            echo -e "${GREEN}✓ Done!${NC}"
            echo "Run './scripts/license-banner.sh check' to verify"
            ;;
        --help|-h)
            cat <<EOF
license-banner.sh - Add or check license headers in source files

Usage:
  ./scripts/license-banner.sh check      Check for missing headers
  ./scripts/license-banner.sh add       Add headers to files missing them
  ./scripts/license-banner.sh --help    Show this help message

Supported file types:
  - Rust (.rs)

Excluded directories:
  - target/
  - node_modules/
  - .git/
EOF
            ;;
        *)
            echo -e "${RED}Error: Unknown action '$action'${NC}"
            echo "Run './scripts/license-banner.sh --help' for usage"
            exit 1
            ;;
    esac
}

main "$@"
