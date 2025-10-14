# ================================
# Rust Project Quality Makefile
# ================================

# Use bash for better syntax
SHELL := /bin/bash

# Default target
.DEFAULT_GOAL := help

# --------------------------------
# 🧹 Code Formatting and Linting
# --------------------------------

fmt:
	@echo "==> Checking code formatting..."
	cargo fmt --all -- --check

fmt-fix:
	@echo "==> Formatting all code..."
	cargo fmt --all

lint:
	@echo "==> Running Clippy lint checks..."
	cargo clippy --all-targets --all-features -- -D warnings

# --------------------------------
# 🧪 Testing and Coverage
# --------------------------------

test:
	@echo "==> Running all tests..."
	cargo test --all --all-features -- --nocapture --quiet

coverage:
	@echo "==> Generating code coverage report (HTML)..."
	cargo tarpaulin --all --out Html

# LLVM-based coverage (if using grcov)
coverage-llvm:
	@echo "==> Generating LLVM-based coverage report..."
	RUSTFLAGS="-Cinstrument-coverage" \
	LLVM_PROFILE_FILE="coverage-%p-%m.profraw" \
	cargo test --all --all-features
	grcov . -s . --binary-path target/debug/ -t html \
		--branch --ignore-not-existing -o coverage/

# --------------------------------
# 🧱 Build Validation
# --------------------------------

check:
	@echo "==> Checking code builds cleanly..."
	cargo check --all-targets --all-features

# --------------------------------
# 🔐 Security & Dependency Checks
# --------------------------------

audit:
	@echo "==> Checking for vulnerable dependencies..."
	cargo audit

outdated:
	@echo "==> Checking for outdated dependencies..."
	cargo outdated

licenses:
	@echo "==> Checking license compliance..."
	cargo deny check licenses

# --------------------------------
# 🌐 WASM Build Targets
# --------------------------------

wasm-build:
	@echo "==> Building all WASM components..."
	@$(MAKE) -C wasm_components build-all

wasm-test:
	@echo "==> Running WASM component tests..."
	@$(MAKE) -C wasm_components test-all

wasm-clean:
	@echo "==> Cleaning WASM build artifacts..."
	@$(MAKE) -C wasm_components clean-all

# --------------------------------
# 🧩 Utility Targets
# --------------------------------

ci: fmt lint check test audit
	@echo "✅ All CI checks passed."

clean:
	@echo "==> Cleaning project..."
	cargo clean

help:
	@echo ""
	@echo "Available targets:"
	@echo "  fmt           - Check code formatting"
	@echo "  fmt-fix       - Format code automatically"
	@echo "  lint          - Run Clippy lints"
	@echo "  test          - Run all tests"
	@echo "  coverage      - Generate tarpaulin HTML coverage"
	@echo "  coverage-llvm - Generate LLVM (grcov) coverage report"
	@echo "  check         - Compile all targets"
	@echo "  audit         - Check for known vulnerabilities"
	@echo "  outdated      - Check for outdated dependencies"
	@echo "  licenses      - Check license compliance (cargo-deny)"
	@echo "  ci            - Run full CI check suite"
	@echo "  clean         - Remove target artifacts"
	@echo "  help          - Show this help message"
	@echo "  wasm-build    - Build all WASM components"
	@echo "  wasm-test     - Run all WASM component tests"
	@echo "  wasm-clean    - Clean WASM build artifacts"
