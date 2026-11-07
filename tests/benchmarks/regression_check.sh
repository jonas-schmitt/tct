#!/usr/bin/env bash
# Regression Detection Script for TCT Real-World Schema Benchmarks
#
# This script compares current benchmark results with baseline to detect regressions.
# Generated benchmark results live under target/ by default so opt-in benchmark
# runs do not modify tracked files.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BASELINE_FILE="$SCRIPT_DIR/baseline.json"
RESULTS_DIR="${TCT_BENCHMARK_OUTPUT_DIR:-$REPO_ROOT/target/tct-benchmarks}"
LATEST_FILE="${TCT_BENCHMARK_LATEST_FILE:-$RESULTS_DIR/latest_extended_results.json}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🔍 TCT Real-World Schema Regression Check"
echo "========================================"

# Check if baseline exists
if [ ! -f "$BASELINE_FILE" ]; then
    echo -e "${YELLOW}⚠️  No baseline file found at $BASELINE_FILE${NC}"
    echo "To create baseline:"
    echo "  cp $LATEST_FILE $BASELINE_FILE"
    echo "  git add $BASELINE_FILE"
    echo "  git commit -m 'Add real-world schema benchmark baseline'"
    exit 0
fi

# Check if latest results exist
if [ ! -f "$LATEST_FILE" ]; then
    echo -e "${RED}❌ No latest results found at $LATEST_FILE${NC}"
    echo "Run extended benchmarks first:"
    echo "  cargo test --features extended-benchmarks --test schema_compilation_test test_extended_real_world_schemas -- --ignored --exact --nocapture"
    echo
    echo "The test writes latest results to:"
    echo "  $RESULTS_DIR/latest_extended_results.json"
    exit 1
fi

# Extract metrics using jq (or python if jq not available)
extract_metric() {
    local file="$1"
    local metric="$2"

    if command -v jq >/dev/null 2>&1; then
        jq -r ".$metric" "$file"
    else
        # Fallback to python if jq not available
        python3 -c "import json; data=json.load(open('$file')); print(data['$metric'])"
    fi
}

# Extract baseline metrics
baseline_success_rate=$(extract_metric "$BASELINE_FILE" "success_rate")
baseline_avg_time=$(extract_metric "$BASELINE_FILE" "avg_compilation_time_ms")
baseline_total=$(extract_metric "$BASELINE_FILE" "total_schemas")
baseline_successful=$(extract_metric "$BASELINE_FILE" "successful_schemas")

# Extract current metrics
current_success_rate=$(extract_metric "$LATEST_FILE" "success_rate")
current_avg_time=$(extract_metric "$LATEST_FILE" "avg_compilation_time_ms")
current_total=$(extract_metric "$LATEST_FILE" "total_schemas")
current_successful=$(extract_metric "$LATEST_FILE" "successful_schemas")

echo "📊 Baseline Metrics:"
echo "   Success Rate: $(python3 -c "print(f'{float('$baseline_success_rate') * 100:.1f}%')")"
echo "   Avg Time: ${baseline_avg_time}ms"
echo "   Schemas: $baseline_successful/$baseline_total"
echo
echo "📊 Current Metrics:"
echo "   Success Rate: $(python3 -c "print(f'{float('$current_success_rate') * 100:.1f}%')")"
echo "   Avg Time: ${current_avg_time}ms"
echo "   Schemas: $current_successful/$current_total"
echo

# Calculate changes
success_rate_change=$(python3 -c "print(float('$current_success_rate') - float('$baseline_success_rate'))")
schema_count_changed=false
if [ "$current_total" != "$baseline_total" ]; then
    schema_count_changed=true
fi

if [ "$schema_count_changed" = false ]; then
    time_change_percent=$(python3 -c "print((float('$current_avg_time') - float('$baseline_avg_time')) / float('$baseline_avg_time') * 100)")
else
    time_change_percent="NaN"
fi

echo "📈 Changes:"
echo "   Success Rate: $(python3 -c "
change = float('$success_rate_change') * 100
if change >= 0:
    print(f'+{change:.1f}%')
else:
    print(f'{change:.1f}%')
")"
if [ "$schema_count_changed" = false ]; then
    echo "   Avg Time: $(python3 -c "
change = float('$time_change_percent')
if change >= 0:
    print(f'+{change:.1f}%')
else:
    print(f'{change:.1f}%')
")"
else
    echo "   Avg Time: not comparable; schema count changed"
fi

# Check for regressions
regression_detected=false

# Success rate regression check (>5% drop)
success_drop_threshold=-0.05
if python3 -c "exit(0 if float('$success_rate_change') >= $success_drop_threshold else 1)"; then
    echo -e "${GREEN}✅ Success rate: No regression${NC}"
else
    echo -e "${RED}❌ Success rate regression detected: $(python3 -c "print(f'{float('$success_rate_change') * 100:.1f}%')") drop (threshold: 5%)${NC}"
    regression_detected=true
fi

# Check for new schema failures
if [ "$schema_count_changed" = true ]; then
    echo -e "${YELLOW}ℹ️  Schema count changed: $baseline_total → $current_total${NC}"
    echo -e "${YELLOW}⚠️  Compilation time regression check skipped because average time is not comparable across different schema sets.${NC}"
else
    # Compilation time regression check (>50% increase)
    time_increase_threshold=50.0
    if python3 -c "exit(0 if float('$time_change_percent') <= $time_increase_threshold else 1)"; then
        echo -e "${GREEN}✅ Compilation time: No regression${NC}"
    else
        echo -e "${RED}❌ Compilation time regression detected: $(python3 -c "print(f'{float('$time_change_percent'):.1f}%')") increase (threshold: 50%)${NC}"
        regression_detected=true
    fi
fi

echo
if [ "$regression_detected" = true ]; then
    echo -e "${RED}❌ REGRESSION DETECTED${NC}"
    echo
    echo "🔍 To investigate:"
    echo "   1. Check recent commits for changes affecting schema compilation"
    echo "   2. Run individual category tests to isolate issues:"
    echo "      cargo test --test schema_compilation_test test_configuration_schemas"
    echo "      cargo test --test schema_compilation_test test_api_schemas"
    echo "   3. Review detailed results in $LATEST_FILE"
    echo "   4. If changes are intentional, update baseline:"
    echo "      cp $LATEST_FILE $BASELINE_FILE"
    echo
    exit 1
else
    echo -e "${GREEN}✅ NO REGRESSIONS DETECTED${NC}"
    echo
    if python3 -c "exit(0 if float('$success_rate_change') <= 0 else 1)"; then
        echo -e "${GREEN}🎉 Success rate improved by $(python3 -c "print(f'{float('$success_rate_change') * 100:.1f}%')")!${NC}"
    fi
    exit 0
fi
