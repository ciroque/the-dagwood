#!/bin/bash
# Copyright (c) 2025 Steve Wagner (ciroque@live.com)
# SPDX-License-Identifier: MIT

# Build script for wasm appender component
# Builds the WASM module and copies it to the expected location

set -e  # Exit on any error

echo "🔨 Building wasm appender component..."

# Ensure we have the WASM target
echo "📦 Checking WASM target..."
if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
    echo "📥 Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# Build the WASM module (C-style)
echo "🚀 Building WASM module..."
cargo build --target wasm32-unknown-unknown --release

# Copy to expected location
echo "📋 Copying artifact to wasm_components/..."
cp target/wasm32-unknown-unknown/release/wasm_appender.wasm ../

# Show file size
WASM_SIZE=$(stat -c%s "../wasm_appender.wasm")
echo "✅ Build complete! wasm_appender.wasm size: ${WASM_SIZE} bytes"

# Optional: Show WASM module info if wasm-objdump is available
if command -v wasm-objdump &> /dev/null; then
    echo "📊 WASM module exports:"
    wasm-objdump -x ../hello.wasm | grep -A 20 "Export\[" | head -20
else
    echo "💡 Install wabt tools for detailed WASM analysis: apt install wabt"
fi

echo "🎉 Ready to use: wasm_components/hello.wasm"
