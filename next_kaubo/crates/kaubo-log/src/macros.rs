//! The `emit!` macro — compile-time zero-overhead event emission.
//!
//! # Two-layer architecture
//!
//! * **Compile layer** (`#[cfg(feature = "kaubo-debug-log")]`): the macro body
//!   either expands to a real event emission or compiles away to nothing.
//!   Precedent: LLVM `LLVM_DEBUG(X)`, V8 `DCHECK(condition)`.
//! * **Runtime layer** (`EventHandler` trait): when the feature is on, the
//!   `filter()` pre-check avoids formatting work for events below the
//!   configured severity level.
//!
//! # Guarantee
//!
//! With `kaubo-debug-log` **off** (release builds), `emit!` produces zero
//! instructions and zero bytes in the binary.  The event expression is
//! captured as raw tokens and never evaluated.
//!
//! # Usage
//!
//! ```ignore
//! emit!(events, ToolchainEvent::Vm(VmEvent::LoopIteration {
//!     func_idx: 0, block_id: 1, count: 42,
//! }));
//! emit!(events, ToolchainEvent::Pass(PassEvent::Started { name: "ConstantFold" }));
//! ```

/// Emit a structured toolchain event (debug builds / feature enabled).
///
/// Builds the event, calls `filter()`, and if it passes, calls `handle()`.
#[cfg(feature = "kaubo-debug-log")]
#[macro_export]
macro_rules! emit {
    ($events:expr, $event:expr) => {
        if let Some(h) = $events {
            let evt = $event;
            if h.filter(&evt) {
                h.handle(&evt);
            }
        }
    };
}

/// No-op in release builds (feature disabled).
///
/// Expands to `let _ = &$events;` which the compiler optimizes away to zero
/// instructions.  The event expression is captured as raw tokens and never
/// evaluated (zero-cost guarantee).
#[cfg(not(feature = "kaubo-debug-log"))]
#[macro_export]
macro_rules! emit {
    ($events:expr, $($rest:tt)*) => {
        // Compiles away to nothing.  `let _ = &$events` suppresses
        // "unused variable" warnings; the compiler eliminates the dead load.
        // The `$($rest:tt)*` arm swallows the event expression without
        // evaluating it — equivalent to LLVM's `((void)0)`.
        let _ = &$events;
    };
}
