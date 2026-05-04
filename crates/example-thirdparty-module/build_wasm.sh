#!/bin/bash
# Script to build the example module for Wasm target

# Ensure the wasm32-wasip1 target is installed
rustup target add wasm32-wasip1

# Build for release
cargo build --target wasm32-wasip1 --release

# Output path
WASM_PATH="target/wasm32-wasip1/release/example_thirdparty_module.wasm"

if [ -f "$WASM_PATH" ]; then
    echo "✓ Build successful: $WASM_PATH"
    echo "Copy this file to your DirtyRack modules folder to use it."
else
    echo "✗ Build failed."
    exit 1
fi
