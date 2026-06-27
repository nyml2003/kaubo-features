//! `kaubo-log-handlers` — concrete `EventHandler` implementations.
//!
//! # Crate contents
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`console`] | `ConsoleHandler` — writes formatted events to stderr / browser console |
//! | [`composite`] | `CompositeHandler` — broadcasts to multiple child handlers |
//! | [`env`] | `parse_severity`, `init_from_env`, `make_handler` — `KAUBO_LOG` parsing |
//!
//! # Architecture
//!
//! `kaubo-log` provides the abstract layer (trait + event types + `emit!` macro).
//! This crate provides the **concrete implementations**.
//!
//! `kaubo-driver` depends only on `kaubo-log` (the abstraction), NOT on this crate.
//! CLI / WASM / tests construct handlers from this crate and inject them into the driver.

pub mod console;
pub mod composite;
pub mod env;

pub use console::ConsoleHandler;
pub use composite::CompositeHandler;
pub use env::{init_from_env, make_handler, parse_severity};
