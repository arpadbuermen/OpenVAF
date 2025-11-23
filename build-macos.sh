#!/bin/bash
#
# Build script for OpenVAF on macOS
#
# This script sets up the environment for building OpenVAF with LLVM 18 on macOS.
# It handles the necessary environment variables and dependencies.
#

set -e

echo "==> Building OpenVAF on macOS with LLVM 18"

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
echo "==> Clang version:"
"$LLVM_SYS_181_PREFIX/bin/clang" --version

# Build
echo "==> Building..."
cargo build --release "$@"

echo "==> Build complete!"
echo "==> Binary location: ./target/release/openvaf-r"
