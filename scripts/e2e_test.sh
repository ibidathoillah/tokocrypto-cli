#!/usr/bin/env bash
# =============================================================================
# Tokocrypto CLI — End-to-End Test Suite
# =============================================================================
# Tests all CLI commands against the live Tokocrypto API.
# Public endpoints require no credentials.
# Private endpoints require API credentials.
#
# Usage:
#   ./scripts/e2e_test.sh              # Run all tests
#   ./scripts/e2e_test.sh --public     # Run public tests only
#   ./scripts/e2e_test.sh --private    # Run private tests only
#   ./scripts/e2e_test.sh --ws         # Run bounded WebSocket smoke tests
#   ./scripts/e2e_test.sh --private-no-precheck  # Run private tests even if auth precheck fails
# =============================================================================

set -euo pipefail

BINARY="${TOKOCRYPTO_BIN:-./target/debug/tokocrypto}"
PAIR="${TOKOCRYPTO_TEST_PAIR:-TKO_IDR}"
PAIR_LOWER=$(echo "$PAIR" | tr '[:upper:]' '[:lower:]')
PAIR_FLEX="${TOKOCRYPTO_TEST_PAIR_FLEX:-tko/idr}"
TEST_COIN="${TOKOCRYPTO_TEST_COIN:-USDT}"

# Counters
PASS=0
FAIL=0
SKIP=0
TOTAL=0

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# =============================================================================
# Helpers
# =============================================================================

log_header() {
    echo ""
    echo -e "${CYAN}${BOLD}══════════════════════════════════════════════${NC}"
    echo -e "${CYAN}${BOLD}  $1${NC}"
    echo -e "${CYAN}${BOLD}══════════════════════════════════════════════${NC}"
}

run_test() {
    local description="$1"
    shift
    TOTAL=$((TOTAL + 1))

    echo -n "  [$TOTAL] $description ... "

    local output
    local exit_code=0
    output=$("$@" 2>&1) || exit_code=$?

    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}PASS${NC}"
        PASS=$((PASS + 1))
    else
        echo -e "${RED}FAIL${NC} (exit=$exit_code)"
        echo "       CMD: $*"
        echo "       OUT: $(echo "$output" | head -3)"
        FAIL=$((FAIL + 1))
    fi
}

run_test_json() {
    local description="$1"
    shift
    TOTAL=$((TOTAL + 1))

    echo -n "  [$TOTAL] $description ... "

    local output
    local exit_code=0
    output=$("$@" 2>&1) || exit_code=$?

    if [ $exit_code -eq 0 ]; then
        # Verify it's valid JSON
        if echo "$output" | python3 -c "import sys, json; json.load(sys.stdin)" 2>/dev/null; then
            echo -e "${GREEN}PASS${NC} (valid JSON)"
            PASS=$((PASS + 1))
        else
            echo -e "${RED}FAIL${NC} (invalid JSON)"
            echo "       OUT: $(echo "$output" | head -3)"
            FAIL=$((FAIL + 1))
        fi
    else
        echo -e "${RED}FAIL${NC} (exit=$exit_code)"
        echo "       CMD: $*"
        echo "       OUT: $(echo "$output" | head -3)"
        FAIL=$((FAIL + 1))
    fi
}

skip_test() {
    local description="$1"
    TOTAL=$((TOTAL + 1))
    SKIP=$((SKIP + 1))
    echo -e "  [$TOTAL] $description ... ${YELLOW}SKIP${NC}"
}

# =============================================================================
# Parse args
# =============================================================================

RUN_PUBLIC=true
RUN_PRIVATE=true
RUN_WS=false
SKIP_PRIVATE_PRECHECK=false

if [[ "${1:-}" == "--public" ]]; then
    RUN_PRIVATE=false
elif [[ "${1:-}" == "--private" ]]; then
    RUN_PUBLIC=false
elif [[ "${1:-}" == "--ws" ]]; then
    RUN_PUBLIC=false
    RUN_PRIVATE=false
    RUN_WS=true
elif [[ "${1:-}" == "--private-no-precheck" ]]; then
    RUN_PUBLIC=false
    SKIP_PRIVATE_PRECHECK=true
fi

# =============================================================================
# Build
# =============================================================================

echo -e "${BOLD}Building tokocrypto-cli ...${NC}"
cargo build 2>&1 | tail -1

if [ ! -f "$BINARY" ]; then
    echo -e "${RED}Binary not found at $BINARY${NC}"
    exit 1
fi

echo -e "${GREEN}Binary: $BINARY${NC}"
echo -e "Test pair: ${CYAN}$PAIR${NC}"
echo ""

# =============================================================================
# PUBLIC MARKET TESTS
# =============================================================================

if $RUN_PUBLIC; then

log_header "PUBLIC — Market Data"

run_test "market ping (table)" \
    $BINARY ping

run_test "market ping (json)" \
    $BINARY -o json ping

run_test "market server-time (table)" \
    $BINARY server-time

run_test_json "market server-time (json)" \
    $BINARY -o json server-time

run_test "market symbols (json)" \
    $BINARY -o json symbols

run_test "market execution-rules --symbol $PAIR (table)" \
    $BINARY execution-rules --pair "$PAIR"

run_test_json "market execution-rules --pair $PAIR_FLEX (json)" \
    $BINARY -o json execution-rules --pair "$PAIR_FLEX"

run_test "market depth $PAIR (table)" \
    $BINARY orderbook "$PAIR" --count 5

run_test_json "market depth $PAIR_FLEX (json)" \
    $BINARY -o json orderbook "$PAIR_FLEX" --count 5

run_test_json "market trades $PAIR (limit=5)" \
    $BINARY -o json trades "$PAIR" --count 5

run_test_json "market agg-trades $PAIR_FLEX (limit=5)" \
    $BINARY -o json agg-trades "$PAIR_FLEX" --count 5

run_test_json "market klines $PAIR (limit=5)" \
    $BINARY -o json klines "$PAIR" --count 5

log_header "PUBLIC — CLI Features"

run_test "--help" \
    $BINARY --help

run_test "--version" \
    $BINARY --version

run_test "market --help" \
    $BINARY ping --help

run_test "account --help" \
    $BINARY account-info --help

run_test "trade --help" \
    $BINARY order --help

run_test "funding --help" \
    $BINARY deposit --help

run_test "ws --help" \
    $BINARY ws --help

run_test "auth --help" \
    $BINARY auth --help

fi  # RUN_PUBLIC

# =============================================================================
# PRIVATE ACCOUNT TESTS
# =============================================================================

if $RUN_PRIVATE; then

log_header "PRIVATE — Account & Trade (requires credentials)"

HAS_CREDS=false
AUTH_TEST_OUTPUT=""
AUTH_TEST_EXIT=0

if $SKIP_PRIVATE_PRECHECK; then
    HAS_CREDS=true
    echo -e "  ${YELLOW}Skipping credential precheck (--private-no-precheck)${NC}"
else
    AUTH_TEST_OUTPUT=$($BINARY auth test 2>&1) || AUTH_TEST_EXIT=$?
    if [ $AUTH_TEST_EXIT -eq 0 ]; then
        HAS_CREDS=true
    fi
fi

if $HAS_CREDS && ! $SKIP_PRIVATE_PRECHECK; then
    HAS_CREDS=true
    echo -e "  ${GREEN}Credentials verified ✓${NC}"
else
    if ! $SKIP_PRIVATE_PRECHECK; then
        echo -e "  ${YELLOW}Credential precheck failed — skipping private tests${NC}"
        echo -e "  Reason: $(echo "$AUTH_TEST_OUTPUT" | head -1)"
        echo -e "  Configure with: ${CYAN}tokocrypto auth set --api-key KEY --api-secret SECRET${NC}"
    fi
fi

if $HAS_CREDS; then
    run_test "auth test" \
        $BINARY auth test

    run_test "auth show" \
        $BINARY auth show

    run_test "account info (table)" \
        $BINARY account-info

    run_test_json "account info (json)" \
        $BINARY -o json account-info

    run_test "account balance (table)" \
        $BINARY balance

    run_test_json "account balance (json)" \
        $BINARY -o json balance

    run_test_json "account assets $TEST_COIN" \
        $BINARY -o json assets "$TEST_COIN"

    run_test "trade open-orders $PAIR_FLEX" \
        $BINARY -o json order open-orders "$PAIR_FLEX"

    run_test "trade all-orders $PAIR" \
        $BINARY -o json order all-orders "$PAIR"

    run_test "funding deposit-address $TEST_COIN (BSC)" \
        $BINARY -o json deposit addresses "$TEST_COIN" --network BSC

    run_test "funding withdraw-history" \
        $BINARY -o json withdrawal status --asset "$TEST_COIN"

    run_test "funding deposit-history" \
        $BINARY -o json deposit status --asset "$TEST_COIN"

else
    skip_test "auth test"
    skip_test "auth show"
    skip_test "account info"
    skip_test "account balance"
    skip_test "account assets"
    skip_test "trade open-orders"
    skip_test "trade all-orders"
    skip_test "funding deposit-address"
    skip_test "funding withdraw-history"
    skip_test "funding deposit-history"
fi

fi  # RUN_PRIVATE

# =============================================================================
# WEBSOCKET TESTS
# =============================================================================

if $RUN_WS; then

log_header "WEBSOCKET — Market & User Streams"

run_test "ws depth $PAIR_FLEX" \
    $BINARY -o json ws depth "$PAIR_FLEX" --limit 1 --seconds 15

AUTH_TEST_OUTPUT=""
AUTH_TEST_EXIT=0
AUTH_TEST_OUTPUT=$($BINARY auth test 2>&1) || AUTH_TEST_EXIT=$?
if [ "${AUTH_TEST_EXIT:-0}" -eq 0 ]; then
    run_test "ws balances subscribe" \
        $BINARY -o json ws balances --limit 1 --seconds 10

    run_test "ws orders subscribe" \
        $BINARY -o json ws orders --limit 1 --seconds 10
else
    echo -e "  ${YELLOW}Credential precheck failed — skipping private WebSocket tests${NC}"
    echo -e "  Reason: $(echo "$AUTH_TEST_OUTPUT" | head -1)"
    skip_test "ws balances subscribe"
    skip_test "ws orders subscribe"
fi

fi  # RUN_WS

# =============================================================================
# Summary
# =============================================================================

echo ""
echo -e "${BOLD}══════════════════════════════════════════════${NC}"
echo -e "${BOLD}  E2E Test Results${NC}"
echo -e "${BOLD}══════════════════════════════════════════════${NC}"
echo -e "  Total:   ${BOLD}$TOTAL${NC}"
echo -e "  Passed:  ${GREEN}${BOLD}$PASS${NC}"
echo -e "  Failed:  ${RED}${BOLD}$FAIL${NC}"
echo -e "  Skipped: ${YELLOW}${BOLD}$SKIP${NC}"
echo -e "${BOLD}══════════════════════════════════════════════${NC}"

if [ $FAIL -gt 0 ]; then
    echo -e "${RED}${BOLD}SOME TESTS FAILED${NC}"
    exit 1
else
    echo -e "${GREEN}${BOLD}ALL TESTS PASSED ✓${NC}"
    exit 0
fi
