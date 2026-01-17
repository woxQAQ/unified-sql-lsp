#!/bin/bash
# Generate profiling report from latest benchmark run

set -e

LATEST_REPORT=$(ls -td target/profiling-reports/* 2>/dev/null | head -1)

if [ -z "$LATEST_REPORT" ]; then
    echo "No profiling reports found. Run ./scripts/profiling/run_all.sh first."
    exit 1
fi

echo "Latest profiling report: $LATEST_REPORT"
echo ""
echo "Summary:"
cat "$LATEST_REPORT/SUMMARY.md"
