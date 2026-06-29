//! LinkedCpsFetcher — collects per-module Cps and ExportTables, then links.
//!
//! PerModuleCpsFetcher compiles each module concurrently via the DAG and
//! seeds ExportTable/{path}. This fetcher requests Cps and ExportTable
//! for each module, then calls LinkStage::link().

use crate::export_table::ExportTable;
use crate::module_graph::ModuleGraph;
use kaubo_dag::{Artifact, ArtifactKey, DagError, FetchContext, Fetcher, Kind};
use kaubo_ir::cps::CpsModule;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

pub struct LinkedCpsFetcher;

impl Default for LinkedCpsFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkedCpsFetcher {
    pub fn new() -> Self { LinkedCpsFetcher }
}

impl Fetcher<String> for LinkedCpsFetcher {
    fn key(&self) -> ArtifactKey<String> {
        ArtifactKey::new("__linked__".to_string(), Kind::new(Kind::LINKED_CPS))
    }
    fn dependencies(&self) -> Vec<ArtifactKey<String>> {
        vec![ArtifactKey::new("__graph__".to_string(), Kind::new(Kind::MODULE_GRAPH))]
    }
    fn fetch<'a>(
        &'a self,
        inputs: Vec<Artifact<String>>,
        ctx: &'a mut FetchContext<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<String>, DagError<String>>> + Send + 'a>> {
        let graph_artifact = inputs.into_iter().next().unwrap();
        Box::pin(async move {
            let Some(graph) = graph_artifact.try_downcast_ref::<ModuleGraph>() else {
                return Err(DagError::Internal("LinkedCps: expected ModuleGraph".into()));
            };
            let order = &graph.order;
            if order.is_empty() {
                return Ok(Artifact::new("__linked__".to_string(), Kind::new(Kind::LINKED_CPS),
                    CpsModule { functions: vec![], constants: vec![], structs: vec![], enums: vec![], vtables: vec![], symbol_map: HashMap::new(), func_owners: vec![] }));
            }

            let mut built: HashMap<String, ExportTable> = HashMap::new();
            for path in order {
                // Request Cps first — triggers PerModuleCpsFetcher which
                // compiles the module and seeds ExportTable/{path}
                let cps_key = ArtifactKey::new(path.clone(), Kind::new(Kind::CPS));
                let _ = ctx.request_dependency(cps_key).await?;
                // Now ExportTable is ready in cache
                let et_key = ArtifactKey::new(path.clone(), Kind::new("ExportTable"));
                let et = ctx.request_dependency(et_key).await?.downcast_clone::<ExportTable>();
                built.insert(path.clone(), et);
            }

            let linked = crate::link_stage::LinkStage::link(&built, order).map_err(|e| {
                DagError::fetcher_error(ArtifactKey::new("__linked__".to_string(), Kind::new(Kind::LINKED_CPS)), format!("link: {e}"))
            })?;
            Ok(Artifact::new("__linked__".to_string(), Kind::new(Kind::LINKED_CPS), linked))
        })
    }
}
