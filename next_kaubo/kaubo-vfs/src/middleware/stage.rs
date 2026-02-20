//! Middleware execution stage

/// Execution stage for middleware
///
/// Stages are ordered by priority. Lower numbers execute first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stage {
    /// Outer layer: logging, tracing, monitoring
    Outer = 100,
    /// Pre-processing: permission checks, validation
    PreProcess = 200,
    /// Path mapping: resolve logical paths to physical paths
    Mapping = 300,
    /// Caching: content and metadata caching
    Caching = 400,
    /// Post-processing: cache write-back, indexing
    PostProcess = 500,
}

impl Stage {
    /// Get stage priority (lower = earlier)
    pub fn priority(&self) -> u32 {
        *self as u32
    }
}

impl Default for Stage {
    fn default() -> Self {
        Stage::Outer
    }
}
