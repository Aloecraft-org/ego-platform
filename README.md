# ego-platform

[![CI](https://github.com/aloecraft-org/ego-platform/actions/workflows/ci.yml/badge.svg)](https://github.com/aloecraft-org/ego-platform/actions/workflows/ci.yml)

A cross-platform Rust library providing unified APIs for native, WASI, and browser environments.

## Features

- **Platform Detection**: Detect runtime environment at compile-time
- **Logging**: Platform-appropriate logging initialization, with an optional output hook
- **Async Spawn**: Task spawning with correct trait bounds per platform, plus a `TaskHandle` mirroring tokio's `JoinHandle`
- **Time Utilities**: Sleep, intervals (with missed-tick behavior), timeout, `Instant`/`SystemTime`
- **IO**: Async stdin/stdout that stays responsive on WASI P2's single-threaded runtime
- **Filesystem**: `read`/`write`/`metadata`/`read_dir` backed by `std::fs` natively and `localStorage` in the browser
- **Synchronization**: Broadcast channels (native/WASI only)

## Supported Platforms

| Platform | Target Triple | Runtime |
|----------|--------------|---------|
| Native | `x86_64-unknown-linux-gnu` (etc.) | Tokio |
| WASI | `wasm32-wasip2` | Tokio (limited) + wasmtime |
| Browser | `wasm32-unknown-unknown` | wasm-bindgen-futures |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
ego-platform = "0.1"
```

## Usage

### Basic Example

```rust
use ego_platform::{init, detect, Platform, spawn, sleep};
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
use ego_platform::Interval;
use std::time::Duration;

let mut interval = Interval::new(Duration::from_millis(100));

loop {
    interval.tick().await;
    println!("Tick!");
}
```

### Broadcast Channels (Native/WASI only)

```rust
use ego_platform::broadcast;

let (tx, mut rx) = broadcast::channel::<String>(10);

tx.send("Hello".to_string()).ok();
let msg = rx.recv().await.unwrap();
```

## Building

### Prerequisites

- Rust 1.88+ (the crate uses the 2024 edition)
- Targets: `rustup target add wasm32-wasip2 wasm32-unknown-unknown`
- WASI test runtime: [wasmtime](https://wasmtime.dev/) (used as the cargo runner, see `.cargo/config.toml`)
- Browser tests: `wasm-bindgen-cli` **pinned to the version in `Cargo.lock`** (currently 0.2.127) plus a browser and matching webdriver (e.g. Firefox + geckodriver):

  ```bash
  cargo install wasm-bindgen-cli --version 0.2.127
  ```

The devcontainer in `.devcontainer/` has all of this preinstalled.

### Quick Build

```bash
make check   # cargo check on all three targets
make test    # run tests on all three targets
make build   # build all three targets
```

### Platform-Specific Commands

```bash
# Native
cargo build
cargo test

# WASI (runs under wasmtime via the configured cargo runner)
cargo build --target wasm32-wasip2
cargo test --target wasm32-wasip2

# Browser (runs under wasm-bindgen-test-runner + headless browser)
cargo build --target wasm32-unknown-unknown
cargo test --target wasm32-unknown-unknown
```

Each `make` verb also has per-platform variants: `make test_native`,
`make test_wasi`, `make test_browser` (same pattern for `build` and `check`).
Append `quiet` to any invocation to suppress rustc warnings (`make test quiet`).

## Continuous Integration

GitHub Actions ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) runs
the same steps as the devcontainer workflow on every push and pull request:

- `make fmt_check` and `make clippy` (all three targets, warnings denied)
- Native build + tests on Linux, macOS, and Windows
- WASI build + tests under wasmtime
- Browser build + tests under `wasm-bindgen-test-runner` with a headless browser

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
├── logging.rs      # Platform-specific logging (+ output hook)
├── spawn.rs        # Task spawning and TaskHandle
├── time.rs         # Sleep, Interval, timeout, Instant/SystemTime
├── io.rs           # Async stdin/stdout
├── sync.rs         # Broadcast channels
└── fs/             # Filesystem (std::fs native, localStorage in browser)
```

## API Differences by Platform

### Spawn

**Native/WASI**: Requires `Send` bound
```rust
pub fn spawn<F, T>(future: F) -> TaskHandle<T>
where F: Future<Output = T> + Send + 'static
```

**Browser**: No `Send` bound required
```rust
pub fn spawn<F, T>(future: F) -> TaskHandle<T>
where F: Future<Output = T> + 'static
```

### Broadcast Channels

**Native/WASI**: Full `tokio::sync::broadcast` implementation

**Browser**: Not available

### Stdin/Stdout

- **Native**: Background thread bridges blocking reads into the async runtime
- **WASI P2**: `wasi:io/poll`-based polling keeps the single-threaded runtime responsive
- **Browser**: Stubs (stdin is always EOF; stdout reports success)

## Troubleshooting

### Browser Tests Not Running

`cargo test --target wasm32-unknown-unknown` uses `wasm-bindgen-test-runner`
(see `.cargo/config.toml`). The runner's version must match the `wasm-bindgen`
version in `Cargo.lock` exactly, and a browser + webdriver (e.g. Firefox +
geckodriver) must be on `PATH`:

```bash
cargo install wasm-bindgen-cli --version 0.2.127
```

### WASI Tests Failing to Execute

The `.wasm` binaries are run through wasmtime (configured as the cargo
runner). Make sure `wasmtime` is installed and on `PATH`.

### Logging Not Working

Make sure you call `init()` before logging:
```rust
ego_platform::init();
log::info!("Now logging works!");
```

## Contributing

Contributions are welcome! Please ensure:

1. All tests pass: `make test`
2. Code is formatted: `make fmt`
3. No clippy warnings on any target: `make clippy`
4. Tests added for new features

`make ci` runs the same sequence as GitHub Actions.

## License

Apache-2.0
