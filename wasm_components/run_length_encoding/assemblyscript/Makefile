# ================================
# RLE AssemblyScript Component Makefile
# ================================

SHELL := /bin/bash
COMPONENT_NAME := rle_assemblyscript
WASM_OUTPUT := ../../$(COMPONENT_NAME).wasm
BUILD_DIR := build
NODE_MODULES := node_modules

.DEFAULT_GOAL := help

# --------------------------------
# 🏗️ Build Targets
# --------------------------------

build: $(NODE_MODULES)
	@echo "📦 Building AssemblyScript RLE WASM Component..."
	@mkdir -p $(BUILD_DIR)
	@echo "🚀 Compiling AssemblyScript to WebAssembly..."
	@npm run build
	@if [ -f "$(BUILD_DIR)/$(COMPONENT_NAME).wasm" ]; then \
		echo "✅ Compilation successful!"; \
		echo "📦 Output: $(BUILD_DIR)/$(COMPONENT_NAME).wasm"; \
		size=$$(wc -c < "$(BUILD_DIR)/$(COMPONENT_NAME).wasm" 2>/dev/null || echo "0"); \
		echo "📏 Size: $$size bytes"; \
		echo ""; \
		echo "🎯 DAGwood integration ready!"; \
		echo "📦 Copying WASM module to: wasm_components/$(COMPONENT_NAME).wasm"; \
		cp $(BUILD_DIR)/$(COMPONENT_NAME).wasm $(WASM_OUTPUT); \
		echo "   Configure processor with: module: wasm_components/$(COMPONENT_NAME).wasm"; \
	else \
		echo "❌ Compilation failed - no WASM output found"; \
		exit 1; \
	fi

$(NODE_MODULES): package.json
	@echo "📦 Installing npm dependencies..."
	@npm install
	@touch $(NODE_MODULES)

# --------------------------------
# 🧪 Test Targets
# --------------------------------

test: build
	@echo "🧪 Running AssemblyScript RLE tests..."
	@if [ ! -f "$(BUILD_DIR)/$(COMPONENT_NAME).wasm" ]; then \
		echo "❌ WASM module not built. Run 'make build' first."; \
		exit 1; \
	fi
	@npm test
	@echo "✅ All tests passed!"

# --------------------------------
# 🧹 Cleanup Targets
# --------------------------------

clean:
	@echo "🧹 Cleaning AssemblyScript build artifacts..."
	@rm -rf $(BUILD_DIR)
	@rm -rf $(NODE_MODULES)
	@rm -f package-lock.json
	@rm -f $(WASM_OUTPUT)
	@echo "✅ Clean complete"

# --------------------------------
# 📊 Info Targets
# --------------------------------

info:
	@echo "📊 AssemblyScript RLE Component Info:"
	@echo "  Component: $(COMPONENT_NAME)"
	@echo "  Output: $(WASM_OUTPUT)"
	@if [ -f "$(BUILD_DIR)/$(COMPONENT_NAME).wasm" ]; then \
		size=$$(wc -c < "$(BUILD_DIR)/$(COMPONENT_NAME).wasm" 2>/dev/null || echo "0"); \
		echo "  Status: Built ($$size bytes)"; \
	else \
		echo "  Status: Not built"; \
	fi
	@if [ -d "$(NODE_MODULES)" ]; then \
		echo "  Dependencies: Installed"; \
	else \
		echo "  Dependencies: Not installed"; \
	fi

help:
	@echo ""
	@echo "AssemblyScript RLE Component Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  build    - Compile AssemblyScript to WASM and copy to wasm_components/"
	@echo "  test     - Run test suite"
	@echo "  clean    - Remove build artifacts and dependencies"
	@echo "  info     - Show component information"
	@echo "  help     - Show this help message"
	@echo ""
	@echo "Dependencies:"
	@echo "  - Node.js (for AssemblyScript compiler)"
	@echo "  - npm (for package management)"
	@echo ""
	@echo "First time setup:"
	@echo "  1. make build    # Installs deps and builds"
	@echo "  2. make test     # Verifies RLE logic works"
	@echo ""

.PHONY: build test clean info help
