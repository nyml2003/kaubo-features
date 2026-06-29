//! ExecuteBuilder — CpsModule → RunOutcome.
//!
//! Terminal builder that loads a CpsModule into the VM and executes it.

use crate::RunOutcome;
use kaubo_dag::{Artifact, ArtifactKey, Builder, DagError, FetchContext, Kind};
use kaubo_ir::cps::CpsModule;
use std::future::Future;
use std::pin::Pin;

/// Executes a compiled `CpsModule` in the VM and collects the result.
pub struct ExecuteBuilder {
    pub module_id: String,
    pub max_loop_iterations: u64,
}

impl ExecuteBuilder {
    pub fn new(module_id: impl Into<String>) -> Self {
        ExecuteBuilder {
            module_id: module_id.into(),
            max_loop_iterations: u64::MAX,
        }
    }

    pub fn with_max_loop_iterations(mut self, limit: u64) -> Self {
        self.max_loop_iterations = limit;
        self
    }
}

impl Builder<String, RunOutcome> for ExecuteBuilder {
    fn name(&self) -> &str {
        "Execute"
    }

    fn dependencies(&self) -> Vec<ArtifactKey<String>> {
        vec![ArtifactKey::new(self.module_id.clone(), Kind::new(Kind::CPS))]
    }

    fn build<'a>(
        &'a self,
        inputs: Vec<Artifact<String>>,
        _ctx: &'a mut FetchContext<String>,
    ) -> Pin<Box<dyn Future<Output = Result<RunOutcome, DagError<String>>> + Send + 'a>> {
        let max_loops = self.max_loop_iterations;
        let cps_artifact = inputs.into_iter().next().unwrap();
        Box::pin(async move {
            let cps = cps_artifact.downcast_ref::<CpsModule>();

            if cps.functions.is_empty() {
                return Ok(RunOutcome {
                    result: 0,
                    output: vec![],
                });
            }

            let mut vm = kaubo_vm::VM::new();
            vm.max_loop_iterations = max_loops;
            vm.load(cps).map_err(|e| DagError::BuilderError(format!("load: {e}")))?;

            let func_idx = cps.functions.len() - 1;
            let reg_count = cps.functions[func_idx].reg_count;

            let result = vm
                .execute(func_idx, reg_count, None)
                .map_err(|e| DagError::BuilderError(format!("runtime: {e:?}")))?;

            Ok(RunOutcome {
                result,
                output: vm.output,
            })
        })
    }
}
