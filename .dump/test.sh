#!/bin/bash
set -e

echo "================================"
echo "AloePlatform Test Runner"
echo "================================"
echo

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

run_test() {
    local name=$1
    local cmd=$2
    
    echo -e "${YELLOW}Running: $name${NC}"
    if eval $cmd; then
        echo -e "${GREEN}✓ $name passed${NC}"
        echo
        return 0
    else
        echo -e "${RED}✗ $name failed${NC}"
        echo
        return 1
    fi
}

# Track failures
FAILURES=0

# Native tests
if run_test "Native tests" "cargo test --lib --bins"; then
    :
else
    FAILURES=$((FAILURES + 1))
fi

# WASI tests
if rustup target list | grep -q "wasm32-wasip2 (installed)"; then
    if run_test "WASI tests" "cargo test --target wasm32-wasip2 --lib"; then
        :
    else
        FAILURES=$((FAILURES + 1))
    fi
else
    echo -e "${YELLOW}⚠ Skipping WASI tests (target not installed)${NC}"
    echo "  Install with: rustup target add wasm32-wasip2"
    echo
fi

# Browser tests
if command -v wasm-pack &> /dev/null; then
    if rustup target list | grep -q "wasm32-unknown-unknown (installed)"; then
        if run_test "Browser tests" "wasm-pack test --headless --firefox"; then
            :
        else
            FAILURES=$((FAILURES + 1))
        fi
    else
        echo -e "${YELLOW}⚠ Skipping browser tests (target not installed)${NC}"
        echo "  Install with: rustup target add wasm32-unknown-unknown"
        echo
    fi
else
    echo -e "${YELLOW}⚠ Skipping browser tests (wasm-pack not installed)${NC}"
    echo "  Install with: cargo install wasm-pack"
    echo
fi

echo "================================"
if [ $FAILURES -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ $FAILURES test suite(s) failed${NC}"
    exit 1
fi