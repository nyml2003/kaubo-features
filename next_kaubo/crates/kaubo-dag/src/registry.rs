//! FetcherRegistry — maps [`Kind`](crate::Kind) to fetcher factory functions.
//!
//! # Concurrency strategy
//!
//! Phase 1 uses a simple `Mutex<HashMap>` for the registry. The registry
//! is populated at startup and is read-only during execution — the `Mutex`
//! is only contended during initialization.
//!
//! Phase 2+ may upgrade to `RwLock` or `OnceCell` for zero-contention reads
//! when runtime registration (e.g., plugin systems) is needed.

use crate::fetcher::Fetcher;
use crate::types::{ArtifactKey, Kind};
use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;

/// A factory function that creates a new [`Fetcher`] instance for a given
/// output key.
pub type FetcherFactory<M> = Box<dyn Fn(ArtifactKey<M>) -> Box<dyn Fetcher<M>> + Send + Sync>;

/// Registry mapping [`Kind`] to fetcher factory functions.
///
/// # Usage
///
/// ```ignore
/// let registry = FetcherRegistry::new();
/// registry.register(Kind::new("Source"), Box::new(|key| {
///     Box::new(MySourceFetcher { key })
/// }));
///
/// // Later, the scheduler calls:
/// let fetcher = registry.create(key); // looks up factory by key.kind
/// ```
pub struct FetcherRegistry<M>
where
    M: Eq + std::hash::Hash + Clone + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    factories: Mutex<HashMap<Kind, FetcherFactory<M>>>,
}

impl<M> FetcherRegistry<M>
where
    M: Eq + std::hash::Hash + Clone + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    /// Create a new, empty registry.
    pub fn new() -> Self {
        FetcherRegistry {
            factories: Mutex::new(HashMap::new()),
        }
    }

    /// Register a fetcher factory for a given [`Kind`].
    ///
    /// If a factory already exists for this kind, it is replaced.
    pub fn register(&self, kind: Kind, factory: FetcherFactory<M>) {
        let mut factories = self.factories.lock().unwrap();
        factories.insert(kind, factory);
    }

    /// Look up the factory for a given key's kind and create a new fetcher
    /// instance.
    ///
    /// Returns `None` if no factory is registered for the key's kind.
    pub fn create(&self, key: &ArtifactKey<M>) -> Option<Box<dyn Fetcher<M>>> {
        let factories = self.factories.lock().unwrap();
        factories.get(&key.kind).map(|factory| factory(key.clone()))
    }

    /// Check whether a factory is registered for the given kind.
    pub fn contains(&self, kind: &Kind) -> bool {
        let factories = self.factories.lock().unwrap();
        factories.contains_key(kind)
    }

    /// Number of registered kinds.
    pub fn len(&self) -> usize {
        self.factories.lock().unwrap().len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.factories.lock().unwrap().is_empty()
    }
}

impl<M> Default for FetcherRegistry<M>
where
    M: Eq + std::hash::Hash + Clone + fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DagError;
    use crate::fetcher::FetchContext;
    use crate::types::Artifact;
    use std::future::Future;

    type M = String;

    /// A trivial fetcher that always returns the same artifact.
    struct ConstantFetcher {
        key: ArtifactKey<M>,
        value: i64,
    }

    impl Fetcher<M> for ConstantFetcher {
        fn key(&self) -> ArtifactKey<M> {
            self.key.clone()
        }

        fn dependencies(&self) -> Vec<ArtifactKey<M>> {
            vec![]
        }

        fn fetch<'a>(
            &'a self,
            _inputs: Vec<Artifact<M>>,
            _ctx: &'a mut FetchContext<M>,
        ) -> std::pin::Pin<
            Box<dyn Future<Output = Result<Artifact<M>, DagError<M>>> + Send + 'a>,
        > {
            let artifact = Artifact::new(
                self.key.module_id.clone(),
                self.key.kind.clone(),
                self.value,
            );
            Box::pin(async move { Ok(artifact) })
        }
    }

    #[test]
    fn register_and_create() {
        let registry = FetcherRegistry::<M>::new();
        let kind = Kind::new("Test");

        registry.register(
            kind.clone(),
            Box::new(move |key| Box::new(ConstantFetcher { key, value: 42 })),
        );

        assert!(registry.contains(&kind));

        let key = ArtifactKey::new("mod".to_string(), kind);
        let fetcher = registry.create(&key).unwrap();
        assert_eq!(fetcher.key(), key);
    }

    #[test]
    fn create_returns_none_for_unknown_kind() {
        let registry = FetcherRegistry::<M>::new();
        let key = ArtifactKey::new("mod".to_string(), Kind::new("Unknown"));
        assert!(registry.create(&key).is_none());
    }

    #[test]
    fn len_and_is_empty() {
        let registry = FetcherRegistry::<M>::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        registry.register(Kind::new("A"), Box::new(|_| unimplemented!()));
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
    }
}
