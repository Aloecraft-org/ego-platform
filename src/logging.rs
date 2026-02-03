// src/platform/logging.rs

pub fn init() {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        console_log::init_with_level(log::Level::Info)
            .expect("Failed to init logger");
        console_error_panic_hook::set_once();
    }
    
    #[cfg(all(target_arch = "wasm32", target_env = "p2"))]
    {
        // WASI: Simple logger (can't use env_logger without pulling in std::env issues)
        struct SimpleLogger;
        
        impl log::Log for SimpleLogger {
            fn enabled(&self, _metadata: &log::Metadata) -> bool {
                true
            }
            
            fn log(&self, record: &log::Record) {
                println!("[{}] {}", record.level(), record.args());
            }
            
            fn flush(&self) {}
        }
        
        static LOGGER: SimpleLogger = SimpleLogger;
        log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(log::LevelFilter::Info))
            .expect("Failed to init logger");
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }
}