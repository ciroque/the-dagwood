#!/bin/bash
# Copyright (c) 2025 Steve Wagner (ciroque@live.com)
# SPDX-License-Identifier: MIT

# Build script for all WASM components in The DAGwood project
# Usage: ./scripts/build-wasm.sh [component_name]
# If no component specified, builds all components

set -e  # Exit on any error

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
WASM_COMPONENTS_DIR="$PROJECT_ROOT/wasm_components"

echo "🏗️  DAGwood WASM Component Builder"
echo "📁 Project root: $PROJECT_ROOT"

# Function to build a single component
build_component() {
    local component_name="$1"
    local component_dir="$WASM_COMPONENTS_DIR/$component_name"
    
    if [[ ! -d "$component_dir" ]]; then
        echo "❌ Component directory not found: $component_dir"
        return 1
    fi
    
    echo ""
    echo "🔨 Building component: $component_name"
    echo "📂 Component directory: $component_dir"
    
    # Check if component has a build script
    if [[ -f "$component_dir/build.sh" ]]; then
        echo "🚀 Running component build script..."
        cd "$component_dir"
        ./build.sh
    else
        echo "⚠️  No build.sh found, using default build process..."
        cd "$component_dir"
        
        # Ensure WASM target is available
        if ! rustup target list --installed | grep -q "wasm32-wasip1"; then
            echo "📥 Installing wasm32-wasip1 target..."
            rustup target add wasm32-wasip1
        fi
        
        # Build the component
        cargo build --target wasm32-wasip1 --release
        
        # Copy to wasm_components directory with component name
        local crate_name=$(grep '^name = ' Cargo.toml | sed 's/name = "\(.*\)"/\1/')
        cp "target/wasm32-wasip1/release/${crate_name}.wasm" "../${component_name}.wasm"
        
        echo "✅ Built: wasm_components/${component_name}.wasm"
    fi
}

# Function to list available components
list_components() {
    echo "📋 Available WASM components:"
    for dir in "$WASM_COMPONENTS_DIR"/*; do
        if [[ -d "$dir" && -f "$dir/Cargo.toml" ]]; then
            local component_name=$(basename "$dir")
            echo "  - $component_name"
        fi
    done
}

# Main logic
if [[ $# -eq 0 ]]; then
    # Build all components
    echo "🔄 Building all WASM components..."
    list_components
    
    for dir in "$WASM_COMPONENTS_DIR"/*; do
        if [[ -d "$dir" && -f "$dir/Cargo.toml" ]]; then
            local component_name=$(basename "$dir")
            build_component "$component_name"
        fi
    done
    
    echo ""
    echo "🎉 All WASM components built successfully!"
    echo "📊 Built artifacts:"
    ls -la "$WASM_COMPONENTS_DIR"/*.wasm 2>/dev/null || echo "  No .wasm files found"
    
elif [[ "$1" == "--list" ]]; then
    list_components
    
else
    # Build specific component
    build_component "$1"
fi

echo ""
echo "✨ Build complete!"
