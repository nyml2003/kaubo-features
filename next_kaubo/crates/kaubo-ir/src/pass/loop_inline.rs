use crate::cps::CpsModule;
use super::Pass;

pub struct LoopInline;

impl Pass for LoopInline {
    fn name(&self) -> &'static str { "loop-inline" }
    fn run(&self, _module: &mut CpsModule) {
        // TODO: fix back-edge redirect
    }
}
