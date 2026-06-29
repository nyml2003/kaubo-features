//! AstFetcher — Source text → Module (AST).
//!
//! Wraps `kaubo_syntax::parser::Parser` into a DAG Fetcher.
//! This is the entry point of the compilation pipeline.

use kaubo_dag::{Artifact, ArtifactKey, DagError, FetchContext, Fetcher, Kind};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Produces a parsed `Module` from source text.
///
/// The source text is stored inside the fetcher (not passed via an Artifact)
/// because in single-file mode there is one canonical source. Multi-file
/// mode (Phase 2) will use a `SourceFetcher` that reads from VFS.
pub struct AstFetcher {
    pub module_id: String,
    pub source: Arc<str>,
}

impl AstFetcher {
    pub fn new(module_id: impl Into<String>, source: impl Into<Arc<str>>) -> Self {
        AstFetcher {
            module_id: module_id.into(),
            source: source.into(),
        }
    }
}

impl Fetcher<String> for AstFetcher {
    fn key(&self) -> ArtifactKey<String> {
        ArtifactKey::new(self.module_id.clone(), Kind::new(Kind::AST))
    }

    fn dependencies(&self) -> Vec<ArtifactKey<String>> {
        vec![]
    }

    fn fetch<'a>(
        &'a self,
        _inputs: Vec<Artifact<String>>,
        _ctx: &'a mut FetchContext<String>,
    ) -> Pin<Box<dyn Future<Output = Result<Artifact<String>, DagError<String>>> + Send + 'a>> {
        let source = Arc::clone(&self.source);
        let module_id = self.module_id.clone();
        Box::pin(async move {
            let module = kaubo_syntax::parser::Parser::new(&source)
                .parse()
                .map_err(|e| {
                    DagError::fetcher_error(
                        ArtifactKey::new(module_id.clone(), Kind::new(Kind::AST)),
                        format!("parse: {e}"),
                    )
                })?;
            Ok(Artifact::new(module_id, Kind::new(Kind::AST), module))
        })
    }
}
