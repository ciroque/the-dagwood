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
        
        # Ensure WASM target is available (no WASI for complete sandboxing)
        if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
            echo "📥 Installing wasm32-unknown-unknown target..."
            rustup target add wasm32-unknown-unknown
        fi
        
        # Build the component (no WASI imports for security)
        cargo build --target wasm32-unknown-unknown --release
        
        # Copy to wasm_components directory with component name
        local crate_name=$(grep '^name = ' Cargo.toml | sed 's/name = "\(.*\)"/\1/')
        cp "target/wasm32-unknown-unknown/release/${crate_name}.wasm" "../${component_name}.wasm"
        
    fi

    echo "✅ Built: wasm_components/${component_name}.wasm"
}

# Function to list available components
list_components() {
    echo "📋 Available WASM components (with build.sh):"
    find "$WASM_COMPONENTS_DIR" -maxdepth 2 -name 'build.sh' -exec dirname {} \; | while read -r dir; do
        component_name=$(basename "$dir")
        echo "  - $component_name"
    done
}

# Main logic
if [[ $# -eq 0 ]]; then
    echo "🔄 Building all WASM components..."
    list_components
    
    # Find all directories containing build.sh files and build them
    find "$WASM_COMPONENTS_DIR" -maxdepth 2 -name 'build.sh' -exec dirname {} \; | while read -r dir; do
        component_name=$(basename "$dir")
        build_component "$component_name"
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
