//! Core data types for the DAG scheduler.
//!
//! These types form the foundation — all other modules depend on them.

use std::any::Any;
use std::fmt;
use std::sync::Arc;

// ── Kind ────────────────────────────────────────────────────────────

/// An opaque identifier for a stage of computation (e.g. `"Ast"`, `"Semantic"`).
///
/// The scheduler treats `Kind` as a black box — its meaning is defined
/// entirely by the [`Fetcher`](crate::Fetcher) that produces/consumes artifacts
/// of this kind.
///
/// # Extensibility
///
/// Built-in constants cover the standard compilation pipeline. Users can
/// register custom kinds via [`Kind::new`] without modifying scheduler code.
///
/// # Future (Phase 2+)
///
/// `Kind` may carry an optional [`KindHint`] tag (`Source`, `Transform`,
/// `Aggregate`) used by the scheduler for priority/resource grouping.
/// Tags do not participate in equality or hashing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Kind(String);

impl Kind {
    // ── Built-in constants ──

    /// Raw source text.
    pub const SOURCE: &'static str = "Source";
    /// Token stream (lexer output).
    pub const TOKEN_STREAM: &'static str = "TokenStream";
    /// Abstract syntax tree.
    pub const AST: &'static str = "Ast";
    /// Semantic analysis result (type inference, name resolution within module).
    pub const SEMANTIC: &'static str = "Semantic";
    /// Continuation-passing style IR.
    pub const CPS: &'static str = "Cps";
    /// Linked CPS (cross-module symbol resolution + type unification).
    pub const LINKED_CPS: &'static str = "LinkedCps";
    /// Module dependency graph.
    pub const MODULE_GRAPH: &'static str = "ModuleGraph";

    /// Create a new `Kind` from any string-like value.
    ///
    /// Use this for custom compilation stages not covered by the built-in
    /// constants (e.g., `Kind::new("Bytecode")`).
    pub fn new(name: impl Into<String>) -> Self {
        Kind(name.into())
    }

    /// Borrow the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Kind {
    fn from(s: &str) -> Self {
        Kind(s.to_owned())
    }
}

impl From<String> for Kind {
    fn from(s: String) -> Self {
        Kind(s)
    }
}

// ── ContentHash ──────────────────────────────────────────────────────

/// Content-addressable hash of an artifact's data.
///
/// Phase 1 uses a 64-bit non-cryptographic hash for simplicity.
/// The type is opaque — the hash scheme can be upgraded to SHA-256
/// in Phase 3 without changing the public API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentHash(u64);

impl ContentHash {
    /// Compute a hash from arbitrary bytes.
    ///
    /// Phase 1 uses `std::hash::DefaultHasher`. Phase 3 will switch to
    /// SHA-256 with dependency hash propagation (per design doc §11.1).
    pub fn from_bytes(data: &[u8]) -> Self {
        use std::hash::Hasher;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash_slice(data, &mut hasher);
        ContentHash(hasher.finish())
    }

    /// A placeholder hash value. Used when the hash scheme is not yet
    /// finalized. Phase 1 artifacts use this; Phase 3 removes it.
    #[doc(hidden)]
    pub fn placeholder() -> Self {
        ContentHash(0)
    }

    /// The raw hash value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

// ── ArtifactKey ──────────────────────────────────────────────────────

/// A unique coordinate in the dependency graph: `(module_id, kind)`.
///
/// `M` is the module identifier type. In production (`kaubo-driver`) this
/// is typically `String` or `Arc<str>`. In tests, `&'static str` or
/// `String` works trivially.
///
/// The scheduler treats `ArtifactKey` as an opaque identifier — it does not
/// inspect the module ID or kind beyond hashing and equality.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactKey<M> {
    /// The module this artifact belongs to.
    pub module_id: M,
    /// The stage/kind of computation this artifact represents.
    pub kind: Kind,
}

impl<M: fmt::Display> fmt::Display for ArtifactKey<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.module_id, self.kind)
    }
}

impl<M> ArtifactKey<M> {
    /// Create a new artifact key.
    pub fn new(module_id: M, kind: Kind) -> Self {
        ArtifactKey { module_id, kind }
    }
}

// ── Artifact ─────────────────────────────────────────────────────────

/// A type-erased, immutable, shareable compilation artifact.
///
/// `M` is the module identifier type — must match the scheduler's `M`.
///
/// # Streaming support (Phase 2)
///
/// When `is_final` is `false`, the artifact lives in the InFlight Map and
/// downstream consumers pull data via a streaming handle. When `is_final`
/// becomes `true`, the artifact moves to the Ready Cache.
///
/// Phase 1 always sets `is_final = true`.
///
/// # Memory
///
/// Inner data is stored behind `Arc` — cloning an `Artifact` is cheap
/// (atomic reference count increment).
#[derive(Clone)]
pub struct Artifact<M> {
    /// The key this artifact was produced for.
    pub key: ArtifactKey<M>,
    /// Content hash of this artifact's data.
    pub hash: ContentHash,
    /// Whether this artifact is complete (Phase 1: always `true`).
    pub is_final: bool,
    /// Type-erased data payload.
    pub(crate) data: Arc<dyn Any + Send + Sync>,
}

impl<M> Artifact<M> {
    /// Create a new, final artifact.
    ///
    /// `T` must be `Send + Sync + 'static` so the artifact can be shared
    /// across threads.
    pub fn new<T: Send + Sync + 'static>(module_id: M, kind: Kind, data: T) -> Self {
        let key = ArtifactKey { module_id, kind };
        let hash = ContentHash::placeholder(); // Phase 3: real content-addressing
        Artifact {
            key,
            hash,
            is_final: true,
            data: Arc::new(data),
        }
    }

    /// Create an artifact with an explicit key and hash.
    pub fn with_key<T: Send + Sync + 'static>(
        key: ArtifactKey<M>,
        hash: ContentHash,
        data: T,
    ) -> Self {
        Artifact {
            key,
            hash,
            is_final: true,
            data: Arc::new(data),
        }
    }

    /// Downcast to a shared reference of the concrete type.
    ///
    /// Returns `None` if `T` doesn't match — callers should handle the
    /// mismatch gracefully (e.g. return a `DagError::Internal`).
    pub fn try_downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.data.downcast_ref::<T>()
    }

    /// Downcast to a shared reference. Panics on type mismatch.
    ///
    /// Prefer [`try_downcast_ref`] for production code.
    pub fn downcast_ref<T: 'static>(&self) -> &T {
        self.data
            .downcast_ref::<T>()
            .expect("downcast_ref: type mismatch")
    }

    /// Clone the inner data. Returns `None` on type mismatch.
    pub fn try_downcast_clone<T: Clone + 'static>(&self) -> Option<T> {
        self.try_downcast_ref::<T>().cloned()
    }

    /// Clone the inner data. Panics on type mismatch.
    ///
    /// Prefer [`try_downcast_clone`] for production code.
    pub fn downcast_clone<T: Clone + 'static>(&self) -> T {
        self.downcast_ref::<T>().clone()
    }
}

impl<M: fmt::Debug> fmt::Debug for Artifact<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Artifact")
            .field("key", &self.key)
            .field("hash", &self.hash)
            .field("is_final", &self.is_final)
            .field("data", &"<opaque>")
            .finish()
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_constants_are_distinct() {
        assert_ne!(Kind::new(Kind::SOURCE), Kind::new(Kind::AST));
        assert_ne!(Kind::new(Kind::SEMANTIC), Kind::new(Kind::CPS));
    }

    #[test]
    fn kind_custom_is_allowed() {
        let k = Kind::new("Bytecode");
        assert_eq!(k.as_str(), "Bytecode");
    }

    #[test]
    fn kind_equality_is_by_value() {
        let a = Kind::new("X");
        let b = Kind::new("X");
        assert_eq!(a, b);
    }

    #[test]
    fn artifact_key_display() {
        let key = ArtifactKey::new("mod.kb", Kind::new(Kind::AST));
        assert_eq!(key.to_string(), "mod.kb/Ast");
    }

    #[test]
    fn artifact_key_equality() {
        let a = ArtifactKey::new("m", Kind::new("X"));
        let b = ArtifactKey::new("m", Kind::new("X"));
        assert_eq!(a, b);
    }

    #[test]
    fn artifact_downcast_roundtrip() {
        let a = Artifact::new("mod".to_string(), Kind::new(Kind::AST), 42i64);
        assert_eq!(*a.downcast_ref::<i64>(), 42);
    }

    #[test]
    #[should_panic(expected = "type mismatch")]
    fn artifact_downcast_wrong_type_panics() {
        let a = Artifact::new("mod".to_string(), Kind::new(Kind::AST), 42i64);
        let _ = a.downcast_ref::<String>(); // should panic
    }

    #[test]
    fn artifact_clone_is_shallow() {
        let a = Artifact::new("mod".to_string(), Kind::new(Kind::AST), vec![1, 2, 3]);
        let b = a.clone();
        assert_eq!(a.key, b.key);
        // Both point to the same Arc
        assert!(Arc::ptr_eq(&a.data, &b.data));
    }

    #[test]
    fn content_hash_different_data_different_hash() {
        let h1 = ContentHash::from_bytes(b"hello");
        let h2 = ContentHash::from_bytes(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn content_hash_same_data_same_hash() {
        let h1 = ContentHash::from_bytes(b"same");
        let h2 = ContentHash::from_bytes(b"same");
        assert_eq!(h1, h2);
    }
}
