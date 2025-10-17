# ================================
# RLE Grain Component Makefile
# ================================

SHELL := /bin/bash
COMPONENT_NAME := rle_grain
GRAIN_SOURCE := src/main.gr
WASM_OUTPUT := ../../$(COMPONENT_NAME).wasm
TARGET_DIR := target
TEST_FILE := test_rle.gr

.DEFAULT_GOAL := help

# --------------------------------
# 🏗️ Build Targets
# --------------------------------

build:
	@echo "🌾 Building Grain RLE WASM Component..."
	@if ! command -v grain &> /dev/null; then \
		echo "❌ Error: Grain compiler not found. Please install Grain from https://grain-lang.org/"; \
		echo "Installation: npm install -g @grain-lang/grain-cli"; \
		exit 1; \
	fi
	@echo "📁 Creating output directory..."
	@mkdir -p $(TARGET_DIR)
	@echo "🚀 Compiling Grain source to WebAssembly..."
	@if ! grain compile $(GRAIN_SOURCE) --release --no-wasm-tail-call -o $(TARGET_DIR)/$(COMPONENT_NAME).wasm; then \
		echo "❌ Failed to compile RLE component"; \
		exit 1; \
	fi
	@echo "✅ RLE component compiled successfully"
	@if [ -f "$(TARGET_DIR)/$(COMPONENT_NAME).wasm" ]; then \
		echo "✅ Compilation successful!"; \
		echo "📦 Output: $(TARGET_DIR)/$(COMPONENT_NAME).wasm"; \
		size=$$(wc -c < $(TARGET_DIR)/$(COMPONENT_NAME).wasm); \
		echo "📏 Size: $$size bytes"; \
		if command -v wasm-objdump &> /dev/null; then \
			echo ""; \
			echo "🔍 WASM Module Exports:"; \
			wasm-objdump -x $(TARGET_DIR)/$(COMPONENT_NAME).wasm | grep -A 20 "Export\[" || true; \
		fi; \
		cp $(TARGET_DIR)/$(COMPONENT_NAME).wasm $(WASM_OUTPUT); \
		echo ""; \
		echo "🎯 DAGwood integration ready!"; \
		echo "📦 WASM module copied to: wasm_components/$(COMPONENT_NAME).wasm"; \
		echo "   Configure processor with: module: wasm_components/$(COMPONENT_NAME).wasm"; \
	else \
		echo "❌ Compilation failed!"; \
		exit 1; \
	fi

# --------------------------------
# 🧪 Test Targets
# --------------------------------

test:
	@echo "🧪 Running tests for $(COMPONENT_NAME)..."
	@if ! command -v grain &> /dev/null; then \
		echo "❌ Error: Grain compiler not found. Cannot run tests."; \
		exit 1; \
	fi
	@if [ -f "$(TEST_FILE)" ]; then \
		echo "🌾 Running Grain test file: $(TEST_FILE)"; \
		grain $(TEST_FILE); \
	else \
		echo "⚠️  No test file found: $(TEST_FILE)"; \
		echo "💡 Create $(TEST_FILE) to add component tests"; \
	fi

test-verbose:
	@echo "🧪 Running verbose tests for $(COMPONENT_NAME)..."
	@if ! command -v grain &> /dev/null; then \
		echo "❌ Error: Grain compiler not found. Cannot run tests."; \
		exit 1; \
	fi
	@if [ -f "$(TEST_FILE)" ]; then \
		echo "🌾 Running Grain test file with verbose output: $(TEST_FILE)"; \
		grain $(TEST_FILE) --verbose; \
	else \
		echo "⚠️  No test file found: $(TEST_FILE)"; \
		echo "💡 Create $(TEST_FILE) to add component tests"; \
	fi

# --------------------------------
# 🧹 Cleanup Targets
# --------------------------------

clean:
	@echo "🧹 Cleaning $(COMPONENT_NAME) build artifacts..."
	@rm -rf $(TARGET_DIR)
	@rm -f $(WASM_OUTPUT)
	@echo "✅ Clean complete"

# --------------------------------
# 🔍 Development Targets
# --------------------------------

check:
	@echo "🔍 Checking $(COMPONENT_NAME) code..."
	@if ! command -v grain &> /dev/null; then \
		echo "❌ Error: Grain compiler not found. Cannot check code."; \
		exit 1; \
	fi
	@grain check $(GRAIN_SOURCE)

fmt:
	@echo "🎨 Formatting $(COMPONENT_NAME) code..."
	@if command -v grain &> /dev/null && grain --help | grep -q format; then \
		grain format $(GRAIN_SOURCE); \
	else \
		echo "⚠️  Grain formatter not available or not supported"; \
	fi

lint:
	@echo "🔍 Linting $(COMPONENT_NAME) code..."
	@if command -v grain &> /dev/null && grain --help | grep -q lint; then \
		grain lint $(GRAIN_SOURCE); \
	else \
		echo "⚠️  Grain linter not available - using check instead"; \
		$(MAKE) check; \
	fi

# --------------------------------
# 📊 Info Targets
# --------------------------------

info:
	@echo "📊 $(COMPONENT_NAME) Component Info:"
	@echo "  Language: Grain"
	@echo "  Source: $(GRAIN_SOURCE)"
	@echo "  Output: $(WASM_OUTPUT)"
	@echo "  Test File: $(TEST_FILE)"
	@if [ -f "$(WASM_OUTPUT)" ]; then \
		WASM_SIZE=$$(stat -c%s "$(WASM_OUTPUT)"); \
		echo "  Size: $${WASM_SIZE} bytes"; \
	else \
		echo "  Status: Not built"; \
	fi
	@if command -v grain &> /dev/null; then \
		echo "  Grain: Available ($$(grain --version 2>/dev/null || echo 'version unknown'))"; \
	else \
		echo "  Grain: Not installed"; \
	fi

help:
	@echo ""
	@echo "$(COMPONENT_NAME) Component Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  build         - Build the WASM component"
	@echo "  test          - Run component tests"
	@echo "  test-verbose  - Run tests with verbose output"
	@echo "  clean         - Remove build artifacts"
	@echo "  check         - Check code without building"
	@echo "  fmt           - Format code (if supported)"
	@echo "  lint          - Run linter checks"
	@echo "  info          - Show component information"
	@echo "  help          - Show this help message"
	@echo ""

.PHONY: build test test-verbose clean check fmt lint info help
