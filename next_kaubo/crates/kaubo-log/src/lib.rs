//! `kaubo-log` — structured event types, `EventHandler` trait, and `emit!` macro.
//!
//! This crate provides the **abstract logging layer** for the Kaubo toolchain.
//! It contains zero platform code and zero external dependencies.
//!
//! # Crate responsibilities
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`event`] | `ToolchainEvent`, `VmEvent`, `CpsEvent`, `PassEvent`, `Severity` |
//! | [`handler`] | `EventHandler` trait, `NoopHandler` |
//! | [`macros`] | `emit!` macro — compile-time zero-overhead event emission |
//!
//! # What's NOT in this crate
//!
//! * `ConsoleHandler`, `CompositeHandler` — these are in `kaubo-log-handlers`
//! * `parse_env()`, `init_from_env()` — these are in `kaubo-log-handlers`
//! * Any platform-specific code (`std::env`, `web_sys`, file I/O)

pub mod event;
pub mod handler;
pub mod macros;

pub use event::*;
pub use handler::*;
