# AloePlatform

A cross-platform Rust library providing unified APIs for native, WASI, and browser environments.

## Features

- **Platform Detection**: Detect runtime environment at compile-time
- **Logging**: Platform-appropriate logging initialization
- **Async Spawn**: Task spawning with correct trait bounds per platform
- **Time Utilities**: Sleep, intervals, and system time
- **Synchronization**: Broadcast channels (native/WASI only)

## Supported Platforms

| Platform | Target Triple | Runtime |
|----------|--------------|---------|
| Native | `x86_64-unknown-linux-gnu` (etc.) | Tokio |
| WASI | `wasm32-wasip2` | Tokio (limited) |
| Browser | `wasm32-unknown-unknown` | wasm-bindgen-futures |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
aloeplatform = "0.1.0"
```

## Usage

### Basic Example

```rust
use aloeplatform::{init, detect, Platform, spawn, sleep};
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Initialize platform (sets up logging)
    init();
    
    // Detect current platform
    match detect() {
        Platform::Native => println!("Running natively"),
        Platform::Wasi => println!("Running on WASI"),
        Platform::Browser => println!("Running in browser"),
    }
    
    // Spawn background task
    spawn(async {
        println!("Background task");
    });
    
    // Sleep
    sleep(Duration::from_secs(1)).await;
}
```

### Intervals

```rust
use aloeplatform::Interval;
use std::time::Duration;

let mut interval = Interval::new(Duration::from_millis(100));

loop {
    interval.tick().await;
    println!("Tick!");
}
```

### Broadcast Channels (Native/WASI only)

```rust
use aloeplatform::broadcast;

let (tx, mut rx) = broadcast::channel::<String>(10);

tx.send("Hello".to_string()).ok();
let msg = rx.recv().await.unwrap();
```

## Building

### Prerequisites

- Rust 1.70+ (2021 edition or later)
- For WASI: `rustup target add wasm32-wasip2`
- For Browser: `rustup target add wasm32-unknown-unknown`
- For WASI runtime: Install [wasmtime](https://wasmtime.dev/)

### Quick Build

```bash
# Check all platforms
make check

# Run tests on all platforms
make test

# Build all platforms
make build

# Run examples
make run
```

### Platform-Specific Commands

```bash
# Native
cargo build
cargo test
cargo run

# WASI
cargo build --target wasm32-wasip2
cargo test --target wasm32-wasip2
cargo run --target wasm32-wasip2

# Browser
cargo build --target wasm32-unknown-unknown
wasm-pack test --headless --firefox
```

## Testing

The library includes comprehensive tests for all platforms:

```bash
# Run all tests
make test

# Native tests only
make test_native

# WASI tests only
make test_wasm

# Browser tests only (requires wasm-pack)
make test_web
```

### Test Coverage

- ✅ Platform detection
- ✅ Logging initialization
- ✅ Task spawning
- ✅ Sleep functionality
- ✅ Interval timers
- ✅ System time
- ✅ Broadcast channels

## Architecture

### Platform Detection

Platform detection happens at compile-time using cfg attributes:

```rust
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
// Browser code

#[cfg(all(target_arch = "wasm32", target_env = "p2"))]
// WASI code

#[cfg(not(target_arch = "wasm32"))]
// Native code
```

### Module Organization

```
src/
├── lib.rs          # Public API and platform detection
├── logging.rs      # Platform-specific logging
├── spawn.rs        # Task spawning
├── time.rs         # Sleep, intervals, SystemTime
└── sync.rs         # Broadcast channels
```

## API Differences by Platform

### Spawn

**Native/WASI**: Requires `Send` bound
```rust
pub fn spawn<F>(future: F)
where F: Future<Output = ()> + Send + 'static
```

**Browser**: No `Send` bound required
```rust
pub fn spawn<F>(future: F)
where F: Future<Output = ()> + 'static
```

### Broadcast Channels

**Native/WASI**: Full tokio::sync::broadcast implementation

**Browser**: Stub implementation (returns immediately, no actual broadcasting)

## Performance Notes

- **Native**: Full tokio runtime with work-stealing scheduler
- **WASI**: Tokio runtime with limited I/O (no network, limited filesystem)
- **Browser**: Single-threaded event loop via wasm-bindgen

## Troubleshooting

### WASI Build Issues

If you see errors about unstable features:

```toml
# Add to .cargo/config.toml
[target.wasm32-wasip2.rustflags]
rustflags = ["--cfg", "wasi_ext"]
```

### Browser Tests Not Running

Install wasm-pack:
```bash
cargo install wasm-pack
```

### Logging Not Working

Make sure you call `init()` before logging:
```rust
aloeplatform::init();
log::info!("Now logging works!");
```

## Contributing

Contributions are welcome! Please ensure:

1. All tests pass: `make test`
2. Code is formatted: `make fmt`
3. No clippy warnings: `make clippy`
4. Tests added for new features

## License

Apache-2.0

## Changelog

### 0.1.0 (2024-02-03)
- Initial release
- Platform detection for Native, WASI, and Browser
- Cross-platform logging, spawning, time, and sync primitives
- Comprehensive test suite