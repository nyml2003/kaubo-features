//! PerModuleCpsFetcher — concurrent per-module compilation via dynamic deps.
//!
//! Each module's compilation is an independent DAG node. Import resolution
//! works through the DAG: A depends on B's `ExportTable`, which B produces
//! after its own compilation. The DAG scheduler orchestrates concurrency.

use crate::export_table::{ExportEntry, ExportTable, ImportTable, RawImport, ResolvedImport};
use crate::module_graph::ModuleGraph;
use crate::module_loader::ModuleLoader;
use crate::protocol::Pipeline;
use kaubo_dag::{Artifact, ArtifactKey, DagError, FetchContext, Fetcher, Kind};
use kaubo_infer::types::{ImportKind, ImportSpec};
use kaubo_ir::flatten::flatten_module;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub struct PerModuleCpsFetcher {
    pub module_path: String,
    pub pipeline: Option<Pipeline>,
    pub loader: Arc<dyn ModuleLoader>,
}

impl PerModuleCpsFetcher {
    pub fn new(module_path: impl Into<String>, pipeline: Option<Pipeline>, loader: Arc<dyn ModuleLoader>) -> Self {
        PerModuleCpsFetcher { module_path: module_path.into(), pipeline, loader }
    }
}

impl Fetcher<String> for PerModuleCpsFetcher {
    fn key(&self) -> ArtifactKey<String> {
        ArtifactKey::new(self.module_path.clone(), Kind::new(Kind::CPS))
    }

    fn dependencies(&self) -> Vec<ArtifactKey<String>> {
        vec![ArtifactKey::new(self.module_path.clone(), Kind::new(Kind::SOURCE))]
    }

    fn fetch<'a>(
        &'a self,
        inputs: Vec<Artifact<String>>,
        ctx: &'a mut FetchContext<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<String>, DagError<String>>> + Send + 'a>> {
        let module_path = self.module_path.clone();
        let pipeline = self.pipeline.clone();
        let loader = Arc::clone(&self.loader);
        let source_artifact = inputs.into_iter().next().unwrap();

        Box::pin(async move {
            let source = source_artifact.downcast_clone::<String>();
            let path = module_path.clone();
            let et_key = ArtifactKey::new(path.clone(), Kind::new("ExportTable"));

            // Reserve ExportTable so downstream modules wait for it
            ctx.mark_in_flight(et_key.clone());

            // 1. Request ModuleGraph to discover imports
            let graph_key = ArtifactKey::new("__graph__".to_string(), Kind::new(Kind::MODULE_GRAPH));
            let graph_artifact = ctx.request_dependency(graph_key).await?;
            let Some(graph) = graph_artifact.try_downcast_ref::<ModuleGraph>() else {
                return Err(DagError::Internal("PerModuleCps: expected ModuleGraph".into()));
            };
            let raw_imports = graph.imports.get(&path).map_or(&[][..], |v| v);

            // 2. Resolve imports — wait for each dependency's ExportTable
            let import_table = if raw_imports.is_empty() {
                ImportTable::empty()
            } else {
                resolve_imports(loader.as_ref(), &path, raw_imports, ctx).await?
            };

            // 3. Parse
            let module = {
                let mut parser = kaubo_syntax::parser::Parser::new(&source);
                for ri in &import_table.entries {
                    if matches!(ri.entry, ExportEntry::Struct { .. }) {
                        parser.register_struct_name(&ri.local_name);
                    }
                }
                parser.parse().map_err(|e| {
                    DagError::fetcher_error(ArtifactKey::new(path.clone(), Kind::new(Kind::CPS)), format!("parse: {e}"))
                })?
            };

            // 4. Convert to ImportSpecs
            let import_specs: Option<Vec<ImportSpec>> = if import_table.is_empty() { None } else {
                Some(import_table.entries.iter().map(|ri| {
                    export_entry_to_import_spec(&ri.entry, &ri.local_name, &ri.source_path)
                }).collect())
            };

            // 5. Infer with imports
            let (type_env, struct_fields, exports) =
                kaubo_infer::infer_module_with_imports(&module, import_specs.as_deref())
                    .map_err(|e| DagError::fetcher_error(ArtifactKey::new(path.clone(), Kind::new(Kind::CPS)), format!("infer: {}", e.msg)))?;

            // 6. CPS build with imports
            let import_map: Option<HashMap<String, usize>> = if import_table.is_empty() { None } else {
                Some(import_table.by_name.clone())
            };
            let import_structs = build_import_structs(&import_table);
            let (cps, export_funcs, export_consts) = kaubo_ir::cps_build::build_module_with_imports(
                &module, None, import_map.as_ref(), &exports, import_structs.as_ref(),
            ).map_err(|e| DagError::fetcher_error(ArtifactKey::new(path.clone(), Kind::new(Kind::CPS)), format!("build: {e}")))?;

            // 7. Flatten + passes
            let mut cps = cps;
            flatten_module(&mut cps);
            if let Some(ref passes) = pipeline {
                if !passes.is_empty() { passes.run(&mut cps, None); }
            }

            // 8. Build FULL ExportTable from TypeEnv (not simplified!)
            let export_table = build_full_export_table(
                &path, &cps, type_env, struct_fields, &exports, &export_funcs, &export_consts, import_table,
            );

            // 9. Wake downstream waiters by storing the complete export table
            ctx.seed_artifact_and_wake(Artifact::new(path.clone(), Kind::new("ExportTable"), export_table));

            Ok(Artifact::new(path, Kind::new(Kind::CPS), cps))
        })
    }
}

async fn resolve_imports(
    loader: &dyn ModuleLoader,
    path: &str,
    raw_imports: &[RawImport],
    ctx: &mut FetchContext<String>,
) -> Result<ImportTable, DagError<String>> {
    let mut entries = Vec::new();
    let mut by_name = HashMap::new();
    for raw_imp in raw_imports {
        let (dep_path, _) = loader.resolve(path, &raw_imp.source_path)
            .map_err(|e| DagError::BuilderError(format!("resolve failed: {e}")))?;
        let et_key = ArtifactKey::new(dep_path.clone(), Kind::new("ExportTable"));
        let et_artifact = ctx.request_dependency(et_key).await?;
        let Some(export_table) = et_artifact.try_downcast_ref::<ExportTable>() else {
            return Err(DagError::Internal(format!("PerModuleCps: expected ExportTable for {dep_path}")));
        };
        for name in &raw_imp.names {
            let entry = export_table.entries.iter().find(|e| e.export_name() == *name).cloned()
                .ok_or_else(|| DagError::BuilderError(format!("export '{name}' not found in {dep_path}")))?;
            if by_name.contains_key(name) {
                let existing: &ResolvedImport = &entries[*by_name.get(name).unwrap()];
                return Err(DagError::BuilderError(format!("symbol conflict: '{name}' from {} and {dep_path}", existing.source_path)));
            }
            by_name.insert(name.clone(), entries.len());
            entries.push(ResolvedImport { local_name: name.clone(), source_path: dep_path.clone(), entry });
        }
    }
    Ok(ImportTable { entries, by_name })
}

fn export_entry_to_import_spec(entry: &ExportEntry, local_name: &str, _source_path: &str) -> ImportSpec {
    match entry {
        ExportEntry::Const { ty, .. } => ImportSpec { local_name: local_name.to_string(), source_path: _source_path.to_string(), kind: ImportKind::Const { ty: ty.clone() } },
        ExportEntry::Function { ty, .. } => ImportSpec { local_name: local_name.to_string(), source_path: _source_path.to_string(), kind: ImportKind::Function { ty: ty.clone() } },
        ExportEntry::Struct { fields, struct_id, .. } => ImportSpec { local_name: local_name.to_string(), source_path: _source_path.to_string(), kind: ImportKind::Struct { struct_id: *struct_id, fields: fields.clone() } },
        ExportEntry::Interface { methods, .. } => ImportSpec { local_name: local_name.to_string(), source_path: _source_path.to_string(), kind: ImportKind::Interface { methods: methods.clone() } },
    }
}

fn build_import_structs(import_table: &ImportTable) -> Option<HashMap<String, (String, usize, Vec<(String, String)>)>> {
    if import_table.is_empty() { return None; }
    let mut s = HashMap::new();
    for ri in &import_table.entries {
        if let ExportEntry::Struct { name, fields, struct_id } = &ri.entry {
            let field_strs: Vec<(String, String)> = fields.iter()
                .map(|(n, t)| (n.clone(), format!("{t}"))).collect();
            s.insert(name.clone(), (ri.source_path.clone(), *struct_id, field_strs));
        }
    }
    if s.is_empty() { None } else { Some(s) }
}

#[allow(clippy::too_many_arguments)]
fn build_full_export_table(
    path: &str, cps: &kaubo_ir::cps::CpsModule, type_env: kaubo_infer::TypeEnv,
    _struct_fields: HashMap<usize, Vec<(String, kaubo_infer::Type)>>,
    exports: &HashSet<String>, export_funcs: &HashMap<String, usize>,
    export_consts: &HashMap<String, usize>, import_table: ImportTable,
) -> ExportTable {
    use kaubo_infer::Type;
    let mut entries = Vec::new();
    for name in exports {
        let ty = type_env.get(name).map(|s| {
            if s.bound.is_empty() { (*s.body).clone() } else { Type::Null }
        }).unwrap_or(Type::Null);
        let entry = match &ty {
            Type::Arrow(_, _) => {
                let func_idx = export_funcs.get(name).copied().unwrap_or_else(|| {
                    cps.functions.iter().position(|f| f.name == *name).unwrap_or(0)
                });
                ExportEntry::Function { name: name.clone(), ty, func_idx }
            }
            Type::Record(struct_id, fields) => ExportEntry::Struct { name: name.clone(), fields: fields.clone(), struct_id: *struct_id },
            _ => {
                let const_idx = export_consts.get(name).copied().unwrap_or(0);
                ExportEntry::Const { name: name.clone(), ty, const_idx }
            }
        };
        entries.push(entry);
    }
    for (key, scheme) in &type_env {
        if key.contains('.') && scheme.bound.is_empty() {
            if let Type::Arrow(_, _) = *scheme.body {
                if let Some(func_idx) = cps.functions.iter().position(|f| f.name == *key) {
                    entries.push(ExportEntry::Function { name: key.clone(), ty: (*scheme.body).clone(), func_idx });
                }
            }
        }
    }
    ExportTable { source_path: path.to_string(), entries, import_table, cps_module: Arc::new(cps.clone()) }
}
