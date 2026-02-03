# ===========================================
# CHECK TARGETS (Fast compilation check)
# ===========================================

check_native:
	RUSTFLAGS="-Awarnings" cargo check

check_wasm:
	RUSTFLAGS="-Awarnings" cargo check --target wasm32-wasip2

check_web:
	RUSTFLAGS="-Awarnings" cargo check --target wasm32-unknown-unknown

check: check_native check_wasm check_web