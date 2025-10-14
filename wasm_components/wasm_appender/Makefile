# ================================
# WASM Appender Component Makefile
# ================================

SHELL := /bin/bash
COMPONENT_NAME := wasm_appender
WASM_TARGET := wasm32-unknown-unknown
WASM_OUTPUT := ../$(COMPONENT_NAME).wasm

.DEFAULT_GOAL := help

# --------------------------------
# 🏗️ Build Targets
# --------------------------------

build:
	@echo "🔨 Building $(COMPONENT_NAME) component..."
	@echo "📦 Checking WASM target..."
	@if ! rustup target list --installed | grep -q "$(WASM_TARGET)"; then \
		echo "📥 Installing $(WASM_TARGET) target..."; \
		rustup target add $(WASM_TARGET); \
	fi
	@echo "🚀 Building WASM module..."
	@cargo build --target $(WASM_TARGET) --release
	@echo "📋 Copying artifact to wasm_components/..."
	@cp target/$(WASM_TARGET)/release/$(COMPONENT_NAME).wasm $(WASM_OUTPUT)
	@WASM_SIZE=$$(wc -c < "$(WASM_OUTPUT)"); \
	echo "✅ Build complete! $(COMPONENT_NAME).wasm size: $${WASM_SIZE} bytes"
	@if command -v wasm-objdump &> /dev/null; then \
		echo "📊 WASM module exports:"; \
		wasm-objdump -x $(WASM_OUTPUT) | grep -A 20 "Export\[" | head -20 || true; \
	else \
		echo "💡 Install wabt tools for detailed WASM analysis: apt install wabt"; \
	fi
	@echo "🎉 Ready to use: wasm_components/$(COMPONENT_NAME).wasm"

# --------------------------------
# 🧪 Test Targets
# --------------------------------

test:
	@echo "🧪 Running tests for $(COMPONENT_NAME)..."
	@echo "🔨 Building WASM module first (required for integration tests)..."
	@$(MAKE) build
	@echo "🚀 Running wasmtime-based integration tests..."
	@cargo test --test wasm_tests -- --nocapture

test-verbose:
	@echo "🧪 Running verbose tests for $(COMPONENT_NAME)..."
	@echo "🔨 Building WASM module first (required for integration tests)..."
	@$(MAKE) build
	@echo "🚀 Running wasmtime-based integration tests with verbose output..."
	@cargo test --test wasm_tests -- --nocapture --test-threads=1

# --------------------------------
# 🧹 Cleanup Targets
# --------------------------------

clean:
	@echo "🧹 Cleaning $(COMPONENT_NAME) build artifacts..."
	@cargo clean
	@rm -f $(WASM_OUTPUT)
	@echo "✅ Clean complete"

# --------------------------------
# 🔍 Development Targets
# --------------------------------

check:
	@echo "🔍 Checking $(COMPONENT_NAME) code..."
	@cargo check --all-targets --all-features

fmt:
	@echo "🎨 Formatting $(COMPONENT_NAME) code..."
	@cargo fmt --all

lint:
	@echo "🔍 Linting $(COMPONENT_NAME) code..."
	@cargo clippy --all-targets --all-features -- -D warnings

# --------------------------------
# 📊 Info Targets
# --------------------------------

info:
	@echo "📊 $(COMPONENT_NAME) Component Info:"
	@echo "  Language: Rust"
	@echo "  Target: $(WASM_TARGET)"
	@echo "  Output: $(WASM_OUTPUT)"
	@if [ -f "$(WASM_OUTPUT)" ]; then \
		WASM_SIZE=$$(stat -c%s "$(WASM_OUTPUT)"); \
		echo "  Size: $${WASM_SIZE} bytes"; \
	else \
		echo "  Status: Not built"; \
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
	@echo "  fmt           - Format code"
	@echo "  lint          - Run linter checks"
	@echo "  info          - Show component information"
	@echo "  help          - Show this help message"
	@echo ""

.PHONY: build test test-verbose clean check fmt lint info help
