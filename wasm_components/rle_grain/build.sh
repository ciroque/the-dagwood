#!/bin/bash

# Build script for Grain RLE WASM Component
# Compiles Grain source to WebAssembly targeting the DAGwood WIT interface

set -e

echo "Building Grain RLE WASM Component..."

# Check if Grain compiler is available
if ! command -v grain &> /dev/null; then
    echo "Error: Grain compiler not found. Please install Grain from https://grain-lang.org/"
    echo "Installation: npm install -g @grain-lang/cli"
    exit 1
fi

# Create output directory
mkdir -p target

# Compile Grain to WASM
echo "Compiling Grain source to WebAssembly..."

echo "Step 1: Compiling RLE module..."
if ! grain compile src/rle.gr --release -o target/rle.gr.wasm; then
    echo "❌ Failed to compile RLE module"
    exit 1
fi
echo "✅ RLE module compiled successfully"

echo "Step 2: Compiling minimal module (no stdlib imports)..."
if ! grain compile src/minimal.gr --release -o target/rle_grain.wasm; then
    echo "❌ Failed to compile minimal module"
    exit 1
fi
echo "✅ Minimal module compiled successfully"

# Check if compilation was successful
if [ -f "target/rle_grain.wasm" ]; then
    echo "✅ Compilation successful!"
    echo "📦 Output: target/rle_grain.wasm"
    
    # Display file size
    size=$(wc -c < target/rle_grain.wasm)
    echo "📏 Size: $size bytes"
    
    # Verify WASM module exports (if wasm-objdump is available)
    if command -v wasm-objdump &> /dev/null; then
        echo ""
        echo "🔍 WASM Module Exports:"
        wasm-objdump -x target/rle_grain.wasm | grep -A 20 "Export\[" || true
    fi
    
    echo ""
    echo "🎯 Ready for DAGwood integration!"
    echo "   Copy target/rle_grain.wasm to your DAGwood wasm_modules directory"
    echo "   Configure processor with: module: wasm_modules/rle_grain.wasm"
    
else
    echo "❌ Compilation failed!"
    exit 1
fi
