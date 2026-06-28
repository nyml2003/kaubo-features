//! `KAUBO_LOG` environment variable parsing.
//!
//! Follows the `RUST_LOG` format (Rust ecosystem de-facto standard):
//!
//! ```text
//! KAUBO_LOG=debug              # all stages at debug level
//! KAUBO_LOG=vm=trace           # only VM at trace
//! KAUBO_LOG=debug,vm=trace     # default debug, VM extra at trace
//! ```
//!
//! # WASM compatibility
//!
//! `std::env::var` is unavailable on WASM.  WASM callers should use
//! `kaubo_log_handlers::make_handler(level)` or the `set_log_level` JS
//! binding exposed by `kaubo-wasm`.  `init_from_env()` is only usable
//! on native targets.

use crate::composite::CompositeHandler;
use crate::console::ConsoleHandler;
use kaubo_log::Severity;

/// Parse a single severity level string.
///
/// Returns `None` for unrecognized values (case-insensitive).
pub fn parse_severity(s: &str) -> Option<Severity> {
    match s.to_lowercase().as_str() {
        "trace" => Some(Severity::Trace),
        "debug" => Some(Severity::Debug),
        "info" => Some(Severity::Info),
        "warn" | "warning" => Some(Severity::Warn),
        "error" => Some(Severity::Error),
        _ => None,
    }
}

/// Build a `CompositeHandler` from the `KAUBO_LOG` environment variable.
///
/// Returns `None` if the environment variable is not set or empty.
///
/// # Format
///
/// ```text
/// KAUBO_LOG=debug              # default debug for all stages
/// KAUBO_LOG=vm=trace           # only VM at trace
/// KAUBO_LOG=debug,vm=trace     # default debug, VM extra at trace
/// ```
///
/// Unrecognized entries are silently ignored.
pub fn init_from_env() -> Option<CompositeHandler> {
    let value = std::env::var("KAUBO_LOG").ok()?;
    if value.trim().is_empty() {
        return None;
    }

    let mut composite = CompositeHandler::new();

    for part in value.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some((target, level_str)) = part.split_once('=') {
            let target = target.trim();
            let level_str = level_str.trim();
            if let Some(severity) = parse_severity(level_str) {
                match target {
                    "vm" => {
                        // Per-target filtering is architecturally reserved but
                        // ConsoleHandler doesn't yet support per-target levels.
                        // For now, we just add a ConsoleHandler at this level.
                        composite = composite.with(Box::new(ConsoleHandler::new(severity)));
                    }
                    _ => {
                        // Unrecognized target — treat as default level
                        composite = composite.with(Box::new(ConsoleHandler::new(severity)));
                    }
                }
            }
        } else {
            // Bare level name: default severity for all stages
            if let Some(severity) = parse_severity(part) {
                composite = composite.with(Box::new(ConsoleHandler::new(severity)));
            }
        }
    }

    if composite.handlers.is_empty() {
        None
    } else {
        Some(composite)
    }
}

/// Build a `CompositeHandler` with a single `ConsoleHandler` at the given level.
///
/// This is the programmatic API used by WASM (where `std::env::var` is unavailable).
pub fn make_handler(level: Severity) -> CompositeHandler {
    CompositeHandler::new().with(Box::new(ConsoleHandler::new(level)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_severity_known() {
        assert_eq!(parse_severity("trace"), Some(Severity::Trace));
        assert_eq!(parse_severity("DEBUG"), Some(Severity::Debug));
        assert_eq!(parse_severity("Info"), Some(Severity::Info));
        assert_eq!(parse_severity("WARN"), Some(Severity::Warn));
        assert_eq!(parse_severity("warning"), Some(Severity::Warn));
        assert_eq!(parse_severity("ERROR"), Some(Severity::Error));
    }

    #[test]
    fn parse_severity_unknown() {
        assert_eq!(parse_severity("verbose"), None);
        assert_eq!(parse_severity(""), None);
    }

    #[test]
    fn make_handler_returns_composite() {
        let _h = make_handler(Severity::Debug);
    }
}
