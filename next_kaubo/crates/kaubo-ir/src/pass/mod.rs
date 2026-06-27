//! Optimization passes — each independent, composable, testable.
//!
//! Passes transform CpsModule in-place. They do not depend on each other,
//! on cps_build, on flatten, or on the VM.

use crate::cps::CpsModule;
use kaubo_log::emit;

pub mod binary;
pub mod empty_block;
pub mod fold;
pub mod loop_inline;
pub mod move_fold;

pub trait Pass {
    fn name(&self) -> &'static str;
    fn run(&self, module: &mut CpsModule);
}

pub fn run_passes(
    module: &mut CpsModule,
    passes: &[&dyn Pass],
    events: Option<&dyn kaubo_log::EventHandler>,
) {
    for pass in passes {
        emit!(
            events,
            kaubo_log::ToolchainEvent::Pass(kaubo_log::PassEvent::Started {
                name: pass.name(),
            })
        );
        pass.run(module);
        emit!(
            events,
            kaubo_log::ToolchainEvent::Pass(kaubo_log::PassEvent::Finished {
                name: pass.name(),
            })
        );
    }
}
