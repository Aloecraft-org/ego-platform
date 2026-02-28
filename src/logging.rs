//! Platform-specific logging initialization.
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// Track last output time - logs will update this
pub static LAST_OUTPUT_TIME: AtomicU64 = AtomicU64::new(0);

// Global hook for custom output behavior (e.g. shell prompt rewriting)
// We use a static RwLock to allow safe concurrent access and modification.
type LogHook = Box<dyn Fn(&log::Record) + Sync + Send>;
static OUTPUT_HOOK: RwLock<Option<LogHook>> = RwLock::new(None);

pub fn register_output_hook<F>(hook: F)
where
    F: Fn(&log::Record) + Sync + Send + 'static,
{
    if let Ok(mut lock) = OUTPUT_HOOK.write() {
        *lock = Some(Box::new(hook));
    }
}

pub fn notify_output() {
    // use std::time::{SystemTime, UNIX_EPOCH};
    let now = crate::SystemTime::now()
        .duration_since(crate::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    LAST_OUTPUT_TIME.store(now, Ordering::Relaxed);
}

// Track if we're in command mode (WASI P2 mostly, but available generally)
static COMMAND_MODE: AtomicBool = AtomicBool::new(false);

static BACKEND_LOGGER: RwLock<Option<Box<dyn log::Log + Send + Sync>>> = RwLock::new(None);

pub fn set_command_mode(enabled: bool) {
    COMMAND_MODE.store(enabled, Ordering::Relaxed);
}

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        // Delegate enabled check to backend if present
        if let Ok(guard) = BACKEND_LOGGER.read() {
            if let Some(logger) = guard.as_ref() {
                return logger.enabled(metadata);
            }
        }
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        // 1. Update activity timestamp
        notify_output();

        // 2. Check Command Mode
        // We bypass command mode blocking if the target is explicit "term" output
        let is_term_output = record.target() == "term";
        if !is_term_output && COMMAND_MODE.load(Ordering::Relaxed) {
            return;
        }

        // 3. Dispatch to Hook (if present)
        // Hooks take precedence over backend output for shell integration
        if let Ok(guard) = OUTPUT_HOOK.read() {
            if let Some(hook) = guard.as_ref() {
                hook(record);
                return;
            }
        }

        // 4. Default Fallback (Delegate to Backend)
        if let Ok(guard) = BACKEND_LOGGER.read() {
            if let Some(logger) = guard.as_ref() {
                logger.log(record);
                return;
            }
        }

        // 5. Ultimate Fallback (if backend missing)
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            use wasm_bindgen::JsValue;
            let msg = format!("[{}] {}", record.level(), record.args());
            web_sys::console::log_1(&JsValue::from_str(&msg));
        }
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            println!("[{}] {}", record.level(), record.args());
        }
    }

    fn flush(&self) {
        if let Ok(guard) = BACKEND_LOGGER.read() {
            if let Some(logger) = guard.as_ref() {
                logger.flush();
            }
        }
    }
}

static LOGGER: SimpleLogger = SimpleLogger;

/// Initialize logging for the current platform.
pub fn init() {
    // 1. Configure Platform Backend
    let mut max_level = log::LevelFilter::Info;

    // A. Web (console_log)
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        // Use console_log crate to map generic Log calls to console.debug/info/warn
        // This makes browser filtering work natively.
        // We initialize it manually (not via init()) to wrap it.
        // Note: The struct is named `WebConsoleLogger` in newer versions or exposed differently.
        // If manual instantiation is tricky, we can implement a trivial wrapper that calls web_sys::console.
        // But let's try the suggestion from the compiler first if available, otherwise fallback.

        struct WebLogger;
        impl log::Log for WebLogger {
            fn enabled(&self, _metadata: &log::Metadata) -> bool {
                true
            }
            fn log(&self, record: &log::Record) {
                // Map Rust log levels to console methods
                use wasm_bindgen::JsValue;
                let msg = format!("{}", record.args());
                let js_msg = JsValue::from_str(&msg);

                match record.level() {
                    log::Level::Error => web_sys::console::error_1(&js_msg),
                    log::Level::Warn => web_sys::console::warn_1(&js_msg),
                    log::Level::Info => web_sys::console::info_1(&js_msg),
                    log::Level::Debug => web_sys::console::debug_1(&js_msg),
                    log::Level::Trace => web_sys::console::trace_1(&js_msg),
                }
            }
            fn flush(&self) {}
        }

        let logger = WebLogger;
        max_level = log::LevelFilter::Debug; // Let browser filter
        if let Ok(mut guard) = BACKEND_LOGGER.write() {
            *guard = Some(Box::new(logger));
        }
        console_error_panic_hook::set_once();
    }

    // B. Native & WASI (env_logger)
    // env_logger works on WASI too, reading RUST_LOG from the host environment.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        // Default to INFO if RUST_LOG is not set
        let env = env_logger::Env::default().default_filter_or("info");
        let logger = env_logger::Builder::from_env(env).build();

        max_level = logger.filter();
        if let Ok(mut guard) = BACKEND_LOGGER.write() {
            *guard = Some(Box::new(logger));
        }
    }

    // 2. Install SimpleLogger as the global subscriber
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(max_level);
    }
}
