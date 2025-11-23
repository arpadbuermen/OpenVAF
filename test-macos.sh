#!/bin/bash
#
# Test script for OpenVAF on macOS
#
# This script runs all tests including integration tests with LLVM 18 on macOS.
#

set -e

echo "==> Testing OpenVAF on macOS with LLVM 18"

# Check if LLVM 18 is installed
if ! brew list llvm@18 &>/dev/null; then
    echo "Error: llvm@18 is not installed"
    echo "Please install it with: brew install llvm@18"
    exit 1
fi

# Set up environment variables
export LLVM_SYS_181_PREFIX=$(brew --prefix llvm@18)
export PATH="$(brew --prefix llvm@18)/bin:$PATH"
export RUST_BACKTRACE=1

echo "==> LLVM 18 path: $LLVM_SYS_181_PREFIX"

# Run tests
echo "==> Running unit tests and integration tests..."
cargo test --release "$@"

echo "==> All tests passed!"
