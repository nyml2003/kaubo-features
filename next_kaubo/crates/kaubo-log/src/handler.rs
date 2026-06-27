//! EventHandler trait — the abstract interface for consuming toolchain events.
//!
//! Stages emit events through this trait without knowing how many handlers exist
//! or where output goes.  The orchestration layer (driver) assembles concrete
//! handlers and injects them.

use crate::event::ToolchainEvent;

/// Receives structured toolchain events.
///
/// # Design
///
/// * `filter()` — cheap pre-check.  Returns `false` to skip formatting.
///   Precedent: .NET `IsEnabled()`, JVM `shouldCommit()`, Rust `log_enabled!()`.
/// * `handle()` — processes the event.  Only called when `filter()` returns `true`.
pub trait EventHandler {
    /// Cheap pre-check.  Return `false` to avoid formatting the event.
    fn filter(&self, event: &ToolchainEvent) -> bool;

    /// Handle the event (format, write, store, etc.).
    /// Only called when `filter()` returned `true`.
    fn handle(&self, event: &ToolchainEvent);
}

/// A no-op handler that discards all events.
///
/// Used as the default when logging is disabled.  The `emit!` macro's
/// feature gate eliminates calls entirely in release builds, so this
/// struct is only relevant in debug builds without the feature flag.
pub struct NoopHandler;

impl EventHandler for NoopHandler {
    fn filter(&self, _: &ToolchainEvent) -> bool {
        false
    }
    fn handle(&self, _: &ToolchainEvent) {}
}
