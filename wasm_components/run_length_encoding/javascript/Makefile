# ================================
# RLE JavaScript WIT Component Makefile
# ================================

SHELL := /bin/bash
COMPONENT_NAME := rle_js
WASM_OUTPUT := ../../$(COMPONENT_NAME).wasm
BUILD_DIR := build
NODE_MODULES := node_modules

.DEFAULT_GOAL := help

# --------------------------------
# 🏗️ Build Targets
# --------------------------------

build: $(NODE_MODULES)
	@echo "📦 Building JavaScript WIT Component (RLE)..."
	@mkdir -p $(BUILD_DIR)
	@echo "🚀 Componentizing JavaScript with jco..."
	@npm run build
	@if [ -f "$(BUILD_DIR)/$(COMPONENT_NAME).wasm" ]; then \
		echo "✅ WIT Component created successfully!"; \
		echo "📦 Output: $(BUILD_DIR)/$(COMPONENT_NAME).wasm"; \
		size=$$(wc -c < "$(BUILD_DIR)/$(COMPONENT_NAME).wasm" 2>/dev/null || echo "0"); \
		echo "📏 Size: $$size bytes"; \
		echo ""; \
		echo "🎯 DAGwood WIT Component ready!"; \
		echo "📦 Copying WASM component to: wasm_components/$(COMPONENT_NAME).wasm"; \
		cp $(BUILD_DIR)/$(COMPONENT_NAME).wasm $(WASM_OUTPUT); \
		echo "   Configure processor with: module: wasm_components/$(COMPONENT_NAME).wasm"; \
		echo "   Executor: WitNodeExecutor"; \
	else \
		echo "❌ Componentization failed - no WASM output found"; \
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
	@echo "JavaScript WIT Component RLE Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  build    - Componentize JavaScript to WIT Component and copy to wasm_components/"
	@echo "  test     - Run test suite"
	@echo "  clean    - Remove build artifacts and dependencies"
	@echo "  info     - Show component information"
	@echo "  help     - Show this help message"
	@echo ""
	@echo "Dependencies:"
	@echo "  - Node.js 22.12.0+ (for jco componentize)"
	@echo "  - npm (for package management)"
	@echo ""
	@echo "First time setup:"
	@echo "  1. make build    # Installs deps and creates WIT Component"
	@echo "  2. make test     # Verifies RLE logic works"
	@echo ""
	@echo "This creates a proper WIT Component that:"
	@echo "  - Implements dagwood:component/processing-node interface"
	@echo "  - Works with WitNodeExecutor (Component Model)"
	@echo "  - Is language-agnostic and composable"
	@echo ""

.PHONY: build test clean info help
