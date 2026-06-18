use crate::plan::ArtifactId;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CacheKey {
    pub task_profile: String,
    pub source_version: u64,
    pub node_stage: String,
    pub output: ArtifactId,
}

impl CacheKey {
    pub fn new(
        task_profile: impl Into<String>,
        source_version: u64,
        node_stage: impl Into<String>,
        output: ArtifactId,
    ) -> Self {
        Self {
            task_profile: task_profile.into(),
            source_version,
            node_stage: node_stage.into(),
            output,
        }
    }
}

#[derive(Debug, Default)]
pub struct ArtifactCache {
    artifacts: BTreeMap<CacheKey, String>,
}

impl ArtifactCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &CacheKey) -> Option<&String> {
        self.artifacts.get(key)
    }

    pub fn insert(&mut self, key: CacheKey, artifact: String) {
        self.artifacts.insert(key, artifact);
    }

    pub fn len(&self) -> usize {
        self.artifacts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_is_partitioned_by_source_version() {
        let mut cache = ArtifactCache::new();
        let a = CacheKey::new("check", 1, "parse", ArtifactId::new("ast"));
        let b = CacheKey::new("check", 2, "parse", ArtifactId::new("ast"));
        cache.insert(a.clone(), "old".to_string());
        cache.insert(b.clone(), "new".to_string());

        assert_eq!(cache.get(&a), Some(&"old".to_string()));
        assert_eq!(cache.get(&b), Some(&"new".to_string()));
        assert_eq!(cache.len(), 2);
    }
}
