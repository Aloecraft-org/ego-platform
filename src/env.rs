//! Process environment variables, with honest absence.
//!
//! Native and WASI processes have an environment; a browser page does not.
//! `std::env::var` collapses "this platform has no environment" into the
//! same error as "the variable is unset", and code deciding whether to fall
//! back to another configuration source needs to tell them apart — an unset
//! variable on native means "the operator chose nothing", while the browser
//! means "look somewhere else entirely". So the answer here is three-valued
//! and the caller matches on it:
//!
//! ```
//! use ego_platform::env::{EnvVar, var};
//!
//! match var("PORT") {
//!     EnvVar::Set(v) => println!("configured: {v}"),
//!     EnvVar::Unset => println!("default it"),
//!     EnvVar::Unsupported => println!("no environment here; use the config the host handed in"),
//! }
//! ```

/// The three answers an environment lookup can honestly give.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvVar {
    /// The variable is set to this value.
    Set(String),
    /// The platform has an environment and this variable is not in it.
    Unset,
    /// The platform has no process environment at all (the browser).
    Unsupported,
}

impl EnvVar {
    /// The value, if set. `Unset` and `Unsupported` both answer `None` —
    /// use a `match` where the difference matters, which is the module's
    /// point.
    pub fn value(self) -> Option<String> {
        match self {
            EnvVar::Set(v) => Some(v),
            _ => None,
        }
    }
}

/// Does this platform have a process environment at all?
pub fn supported() -> bool {
    !cfg!(all(target_arch = "wasm32", target_os = "unknown"))
}

/// Look up `name` in the process environment.
///
/// A value that is present but not valid Unicode is reported lossily
/// rather than hidden: configuration that exists should never read as
/// absent.
pub fn var(name: &str) -> EnvVar {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        let _ = name;
        EnvVar::Unsupported
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        match std::env::var(name) {
            Ok(v) => EnvVar::Set(v),
            Err(std::env::VarError::NotPresent) => EnvVar::Unset,
            Err(std::env::VarError::NotUnicode(os)) => {
                EnvVar::Set(os.to_string_lossy().into_owned())
            }
        }
    }
}

#[cfg(all(test, not(all(target_arch = "wasm32", target_os = "unknown"))))]
mod tests {
    use super::*;

    #[test]
    fn set_unset_and_supported() {
        assert!(supported());
        // Uses a name no runner sets; set_var is unsafe in edition 2024 and
        // this test suite runs threaded, so assert on ambient state instead.
        assert_eq!(var("EGO_PLATFORM_TEST_SURELY_UNSET_VAR"), EnvVar::Unset);
        match var("PATH") {
            EnvVar::Set(_) | EnvVar::Unset => {}
            EnvVar::Unsupported => panic!("native has an environment"),
        }
        assert_eq!(EnvVar::Unset.value(), None);
        assert_eq!(EnvVar::Set("x".into()).value(), Some("x".into()));
    }
}
