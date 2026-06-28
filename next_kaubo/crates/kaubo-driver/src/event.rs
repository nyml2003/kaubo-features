//! Event layer — sinks, routing, and lifecycle management.
//!
//! Extends Phase 1's `EventHandler` trait with lifecycle methods (`open` / `close` /
//! `flush`) and fan-out routing.  All existing `EventHandler` implementations
//! automatically become `EventSink` via a blanket impl — zero migration cost.

use kaubo_log::{EventHandler, ToolchainEvent};

/// An event destination with lifecycle management.
///
/// Every existing `EventHandler` is automatically an `EventSink` via the
/// blanket impl below — no code changes needed for Phase 1 handlers.
pub trait EventSink {
    /// Unique name for this sink (used in configuration and logging).
    fn name(&self) -> &str;

    /// Handle an event.
    fn handle(&self, event: &ToolchainEvent);

    /// Optional: initialise the sink (open files, establish connections, etc.).
    fn open(&mut self) {}

    /// Optional: tear down the sink (close files, flush buffers, etc.).
    fn close(&mut self) {}

    /// Optional: flush any buffered output.
    fn flush(&mut self) {}
}

/// Blanket impl: all Phase 1 `EventHandler` types are automatically `EventSink`.
impl<T: EventHandler> EventSink for T {
    fn name(&self) -> &str {
        "handler"
    }

    fn handle(&self, event: &ToolchainEvent) {
        if self.filter(event) {
            EventHandler::handle(self, event);
        }
    }
}

/// Fans events out to multiple sinks.  Sinks can be added / removed at runtime.
pub struct EventRouter {
    sinks: Vec<Box<dyn EventSink>>,
}

impl EventRouter {
    pub fn new() -> Self {
        Self { sinks: vec![] }
    }

    pub fn add(&mut self, sink: Box<dyn EventSink>) {
        self.sinks.push(sink);
    }

    pub fn remove(&mut self, name: &str) {
        self.sinks.retain(|s| s.name() != name);
    }

    /// Open all sinks.
    pub fn open_all(&mut self) {
        for sink in &mut self.sinks {
            sink.open();
        }
    }

    /// Close all sinks (flush + close).
    pub fn close_all(&mut self) {
        for sink in &mut self.sinks {
            sink.flush();
            sink.close();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.sinks.is_empty()
    }
}

impl EventHandler for EventRouter {
    fn filter(&self, _event: &ToolchainEvent) -> bool {
        true
    }

    fn handle(&self, event: &ToolchainEvent) {
        for sink in &self.sinks {
            sink.handle(event);
        }
    }
}

/// A "null" sink that discards all events.  Useful as a default when no
/// output is desired.
pub struct NullSink;

impl EventSink for NullSink {
    fn name(&self) -> &str {
        "null"
    }
    fn handle(&self, _event: &ToolchainEvent) {}
}
