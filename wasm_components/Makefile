# ================================
# WASM Components Master Makefile
# ================================

SHELL := /bin/bash

# Auto-discover all component directories that have Makefiles
COMPONENTS := $(shell find . -maxdepth 2 -name 'Makefile' -not -path './Makefile' -exec dirname {} \; | sed 's|./||' | sort)

.DEFAULT_GOAL := help

# --------------------------------
# 🏗️ Build Targets
# --------------------------------

build-all:
	@echo "🔨 Building all WASM components..."
	@echo "📋 Found components: $(COMPONENTS)"
	@if [ -z "$(COMPONENTS)" ]; then \
		echo "⚠️  No WASM components found with Makefiles"; \
		exit 0; \
	fi
	@for component in $(COMPONENTS); do \
		echo ""; \
		echo "🔨 Building component: $$component"; \
		echo "📂 Component directory: $$component"; \
		if $(MAKE) -C $$component build; then \
			echo "✅ Built: $$component"; \
		else \
			echo "❌ Build failed for $$component"; \
			exit 1; \
		fi; \
	done
	@echo ""
	@echo "🎉 All WASM components built successfully!"
	@echo "📊 Built artifacts:"
	@ls -la *.wasm 2>/dev/null || echo "  No .wasm files found"

build-%:
	@echo "🔨 Building specific component: $*"
	@if [ ! -d "$*" ]; then \
		echo "❌ Component directory not found: $*"; \
		exit 1; \
	fi
	@if [ ! -f "$*/Makefile" ]; then \
		echo "❌ No Makefile found in component: $*"; \
		exit 1; \
	fi
	@$(MAKE) -C $* build

# --------------------------------
# 🧪 Test Targets
# --------------------------------

test-all:
	@echo "🧪 Testing all WASM components..."
	@echo "📋 Found components: $(COMPONENTS)"
	@if [ -z "$(COMPONENTS)" ]; then \
		echo "⚠️  No WASM components found with Makefiles"; \
		exit 0; \
	fi
	@passed=0; failed=0; failed_components=""; \
	for component in $(COMPONENTS); do \
		echo ""; \
		echo "🧪 Testing component: $$component"; \
		echo "📂 Component directory: $$component"; \
		if $(MAKE) -C $$component test; then \
			echo "✅ Tests passed: $$component"; \
			passed=$$((passed + 1)); \
		else \
			echo "❌ Tests failed: $$component"; \
			failed=$$((failed + 1)); \
			failed_components="$$failed_components $$component"; \
		fi; \
	done; \
	echo ""; \
	echo "📊 Test Summary:"; \
	echo "  ✅ Passed: $$passed"; \
	echo "  ❌ Failed: $$failed"; \
	if [ $$failed -gt 0 ]; then \
		echo "  Failed components:$$failed_components"; \
		exit 1; \
	else \
		echo "🎉 All tests passed!"; \
	fi

test-%:
	@echo "🧪 Testing specific component: $*"
	@if [ ! -d "$*" ]; then \
		echo "❌ Component directory not found: $*"; \
		exit 1; \
	fi
	@if [ ! -f "$*/Makefile" ]; then \
		echo "❌ No Makefile found in component: $*"; \
		exit 1; \
	fi
	@$(MAKE) -C $* test

# --------------------------------
# 🧹 Cleanup Targets
# --------------------------------

clean-all:
	@echo "🧹 Cleaning all WASM components..."
	@for component in $(COMPONENTS); do \
		echo "🧹 Cleaning component: $$component"; \
		$(MAKE) -C $$component clean || true; \
	done
	@echo "🧹 Cleaning WASM artifacts..."
	@rm -f *.wasm
	@echo "✅ Clean complete"

clean-%:
	@echo "🧹 Cleaning specific component: $*"
	@if [ ! -d "$*" ]; then \
		echo "❌ Component directory not found: $*"; \
		exit 1; \
	fi
	@if [ ! -f "$*/Makefile" ]; then \
		echo "❌ No Makefile found in component: $*"; \
		exit 1; \
	fi
	@$(MAKE) -C $* clean

# --------------------------------
# 📊 Info Targets
# --------------------------------

list:
	@echo "📋 Available WASM components:"
	@if [ -z "$(COMPONENTS)" ]; then \
		echo "  No components found with Makefiles"; \
	else \
		for component in $(COMPONENTS); do \
			echo "  - $$component"; \
			if [ -f "$$component/Cargo.toml" ]; then \
				echo "    Language: Rust"; \
			elif [ -f "$$component/grain.toml" ]; then \
				echo "    Language: Grain"; \
			elif [ -f "$$component/CMakeLists.txt" ] || [ -f "$$component/meson.build" ] || [ -f "$$component/configure.ac" ]; then \
				echo "    Language: C/C++"; \
			elif [ -f "$$component/package.json" ]; then \
				echo "    Language: Node.js"; \
			elif [ -f "$$component/go.mod" ]; then \
				echo "    Language: Go"; \
			else \
				echo "    Language: Unknown"; \
			fi; \
			if [ -f "$$component.wasm" ]; then \
				size=$$(wc -c < "$$component.wasm" 2>/dev/null | tr -d '[:space:]' || echo "0"); \
				echo "    Status: Built ($$size bytes)"; \
			else \
				echo "    Status: Not built"; \
			fi; \
		done; \
	fi

info:
	@echo "📊 WASM Components Overview:"
	@echo "  Total components: $(words $(COMPONENTS))"
	@echo "  Components: $(COMPONENTS)"
	@built_count=0; \
	for component in $(COMPONENTS); do \
		if [ -f "$$component.wasm" ]; then \
			built_count=$$((built_count + 1)); \
		fi; \
	done; \
	echo "  Built artifacts: $$built_count"

help:
	@echo ""
	@echo "WASM Components Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  build-all     - Build all WASM components"
	@echo "  build-<name>  - Build specific component"
	@echo "  test-all      - Test all WASM components"
	@echo "  test-<name>   - Test specific component"
	@echo "  clean-all     - Clean all components"
	@echo "  clean-<name>  - Clean specific component"
	@echo "  list          - List all available components"
	@echo "  info          - Show components overview"
	@echo "  help          - Show this help message"
	@echo ""
	@echo "Examples:"
	@echo "  make build-wasm_appender  - Build just the wasm_appender component"
	@echo "  make test-rle_grain       - Test just the rle_grain component"
	@echo ""

.PHONY: build-all build-% test-all test-% clean-all clean-% list info help
