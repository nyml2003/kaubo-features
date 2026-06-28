//! `CompositeHandler` — broadcasts events to multiple downstream handlers.
//!
//! # Broadcast semantics
//!
//! `CompositeHandler` does **not** override `filter()` — it always returns
//! `true`.  Each child handler's own `filter()` + `handle()` pair makes the
//! per-event decision.  In release builds the `emit!` macro eliminates the
//! entire path; in debug builds an empty `Vec` traversal is negligible.

use kaubo_log::{EventHandler, ToolchainEvent};

/// Distributes events to zero or more child handlers.
pub struct CompositeHandler {
    pub handlers: Vec<Box<dyn EventHandler>>,
}

impl CompositeHandler {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Add a child handler.  Returns `self` for chaining.
    pub fn with(mut self, handler: Box<dyn EventHandler>) -> Self {
        self.handlers.push(handler);
        self
    }
}

impl Default for CompositeHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for CompositeHandler {
    /// Always returns `true` — delegates filtering to child handlers.
    fn filter(&self, _event: &ToolchainEvent) -> bool {
        true
    }

    fn handle(&self, event: &ToolchainEvent) {
        for h in &self.handlers {
            if h.filter(event) {
                h.handle(event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A handler that records the last event it saw for test assertions.
    struct RecordingHandler {
        pub seen: std::cell::RefCell<Vec<String>>,
    }

    impl EventHandler for RecordingHandler {
        fn filter(&self, _: &ToolchainEvent) -> bool {
            true
        }
        fn handle(&self, event: &ToolchainEvent) {
            self.seen.borrow_mut().push(format!("{event:?}"));
        }
    }

    #[test]
    fn composite_broadcasts_to_all_children() {
        let _child1 = RecordingHandler {
            seen: std::cell::RefCell::new(vec![]),
        };
        let _child2 = RecordingHandler {
            seen: std::cell::RefCell::new(vec![]),
        };

        // Can't put RecordingHandler in Vec<Box<dyn EventHandler>> because
        // RecordingHandler borrows its seen field — use a shared vec instead.
        let seen: std::rc::Rc<std::cell::RefCell<Vec<String>>> =
            std::rc::Rc::new(std::cell::RefCell::new(vec![]));

        struct SharedHandler {
            seen: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
            id: &'static str,
        }
        impl EventHandler for SharedHandler {
            fn filter(&self, _: &ToolchainEvent) -> bool {
                true
            }
            fn handle(&self, event: &ToolchainEvent) {
                self.seen
                    .borrow_mut()
                    .push(format!("{}: {event:?}", self.id));
            }
        }

        let composite = CompositeHandler::new()
            .with(Box::new(SharedHandler {
                seen: seen.clone(),
                id: "A",
            }))
            .with(Box::new(SharedHandler {
                seen: seen.clone(),
                id: "B",
            }));

        let event = ToolchainEvent::Pass(kaubo_log::PassEvent::Started {
            name: "TestPass",
        });
        composite.handle(&event);

        let entries = seen.borrow();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].starts_with("A:"));
        assert!(entries[1].starts_with("B:"));
    }

    #[test]
    fn empty_composite_does_nothing() {
        let composite = CompositeHandler::new();
        let event = ToolchainEvent::Pass(kaubo_log::PassEvent::Started {
            name: "TestPass",
        });
        // Should not panic
        composite.handle(&event);
    }
}
