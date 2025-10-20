# ================================
# RLE Component Makefile
# ================================

SHELL := /bin/bash
COMPONENT_NAME := run_length_encoding
WASM_TARGET := wasm32-wasip2
WASM_OUTPUT := ../../rle_rust.wasm

.DEFAULT_GOAL := help

# --------------------------------
# 🏗️ Build Targets
# --------------------------------

build:
	@echo "🔨 Building $(COMPONENT_NAME) component..."
	@echo "📦 Checking wasm32-wasip2 target..."
	@if ! rustup target list | grep -q "wasm32-wasip2 (installed)"; then \
		echo "📥 Installing wasm32-wasip2 target..."; \
		rustup target add wasm32-wasip2; \
	fi
	@echo "🧹 Ensuring clean WIT dependencies..."
	@if command -v wit-deps &> /dev/null; then \
		wit-deps; \
	else \
		echo "⚠️  wit-deps not found, install with: cargo install wit-deps-cli"; \
		exit 1; \
	fi
	@echo "🚀 Building WASM component..."
	@cargo build --release --target $(WASM_TARGET)
	@echo "📋 Copying artifact to $(WASM_OUTPUT)..."
	@cp target/$(WASM_TARGET)/release/$(COMPONENT_NAME).wasm $(WASM_OUTPUT)
	@WASM_SIZE=$$(wc -c < "$(WASM_OUTPUT)"); \
	echo "✅ Build complete! $(COMPONENT_NAME).wasm size: $${WASM_SIZE} bytes"
	@if command -v wasm-tools &> /dev/null; then \
		echo "📊 WASM component info:"; \
		wasm-tools component wit $(WASM_OUTPUT) 2>/dev/null | head -20 || echo "  (Component Model format)"; \
	else \
		echo "💡 Install wasm-tools for component analysis: cargo install wasm-tools"; \
	fi
	@echo "🎉 Ready to use: $(WASM_OUTPUT)"

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
	@echo "  Target: $(WASM_TARGET) (Component Model)"
	@echo "  Output: $(WASM_OUTPUT)"
	@if [ -f "$(WASM_OUTPUT)" ]; then \
		WASM_SIZE=$$(wc -c < "$(WASM_OUTPUT)"); \
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
