//! Optimization passes — each independent, composable, testable.
//!
//! Passes transform CpsModule in-place. They do not depend on each other,
//! on cps_build, on flatten, or on the VM.

use crate::cps::CpsModule;

pub mod binary;
pub mod empty_block;
pub mod fold;
pub mod loop_inline;

pub trait Pass {
    fn name(&self) -> &'static str;
    fn run(&self, module: &mut CpsModule);
}

pub fn run_passes(module: &mut CpsModule, passes: &[&dyn Pass]) {
    for pass in passes {
        if cfg!(debug_assertions) {
            eprintln!("[PASS] {}", pass.name());
        }
        pass.run(module);
    }
}
