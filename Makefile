# ===========================================
# ALOEPLATFORM - Cross-platform Rust Library
# ===========================================

.PHONY: help check test build run clean all

# Default target
help:
	@echo "Available targets:"
	@echo "  check        - Check all platforms (native, wasm, web)"
	@echo "  test         - Run tests for all platforms"
	@echo "  build        - Build for all platforms"
	@echo "  run          - Run examples for all platforms"
	@echo "  clean        - Clean build artifacts"
	@echo "  all          - Check, test, and build everything"
	@echo ""
	@echo "Platform-specific targets:"
	@echo "  check_native, test_native, build_native, run_native"
	@echo "  check_wasm, test_wasm, build_wasm, run_wasm"
	@echo "  check_web, test_web, build_web"

# ===========================================
# CHECK TARGETS (Fast compilation check)
# ===========================================

check_native:
	@echo "==> Checking native target..."
	@RUSTFLAGS="-Awarnings" cargo check

check_wasm:
	@echo "==> Checking WASM (WASI P2) target..."
	@RUSTFLAGS="-Awarnings" cargo check --target wasm32-wasip2

check_web:
	@echo "==> Checking Web (Browser) target..."
	@RUSTFLAGS="-Awarnings" cargo check --target wasm32-unknown-unknown

check: check_native check_wasm check_web
	@echo "✓ All platforms checked successfully"

# ===========================================
# TEST TARGETS
# ===========================================

test_native:
	@echo "==> Running native tests..."
	@cargo test --lib --bins

test_wasm:
	@echo "==> Running WASM (WASI P2) tests..."
	@cargo test --target wasm32-wasip2 --lib

test_web:
	@echo "==> Running Web (Browser) tests..."
	@echo "Note: Browser tests require wasm-pack"
	@if command -v wasm-pack >/dev/null 2>&1; then \
		wasm-pack test --headless --firefox --chrome; \
	else \
		echo "Warning: wasm-pack not found. Install with: cargo install wasm-pack"; \
		echo "Skipping browser tests..."; \
	fi

test: test_native test_wasm test_web
	@echo "✓ All platform tests completed"

# ===========================================
# BUILD TARGETS
# ===========================================

build_native:
	@echo "==> Building native target..."
	@cargo build --release

build_wasm:
	@echo "==> Building WASM (WASI P2) target..."
	@cargo build --release --target wasm32-wasip2

build_web:
	@echo "==> Building Web (Browser) target..."
	@cargo build --release --target wasm32-unknown-unknown

build: build_native build_wasm build_web
	@echo "✓ All platforms built successfully"

# ===========================================
# RUN TARGETS (Examples)
# ===========================================

run_native:
	@echo "==> Running native example..."
	@cargo run --release

run_wasm:
	@echo "==> Running WASM (WASI P2) example..."
	@cargo run --release --target wasm32-wasip2

run: run_native run_wasm
	@echo "✓ Examples executed successfully"

# ===========================================
# UTILITY TARGETS
# ===========================================

clean:
	@echo "==> Cleaning build artifacts..."
	@cargo clean
	@echo "✓ Clean completed"

fmt:
	@echo "==> Formatting code..."
	@cargo fmt
	@echo "✓ Format completed"

clippy:
	@echo "==> Running clippy..."
	@cargo clippy --all-targets --all-features -- -D warnings
	@echo "✓ Clippy completed"

doc:
	@echo "==> Building documentation..."
	@cargo doc --no-deps --open
	@echo "✓ Documentation built"

# ===========================================
# COMBINED TARGETS
# ===========================================

all: check test build
	@echo "✓✓✓ All tasks completed successfully ✓✓✓"

ci: fmt clippy check test
	@echo "✓ CI checks passed"