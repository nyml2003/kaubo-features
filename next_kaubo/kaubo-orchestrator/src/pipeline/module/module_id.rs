//! Module identifier for multi-module compilation

use std::path::PathBuf;

/// Error type for module ID parsing
#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error("Empty module path")]
    EmptyModulePath,
    
    #[error("Invalid module path: {0}")]
    InvalidModulePath(String),
    
    #[error("Empty component in path: {0}")]
    EmptyComponent(String),
}

/// Module identifier
///
/// Represents a module path like "math.utils" as components ["math", "utils"]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleId {
    /// Module path components
    /// "math.utils" -> ["math", "utils"]
    pub components: Vec<String>,
}

impl ModuleId {
    /// Parse an import path into a ModuleId
    ///
    /// # Examples
    /// ```
    /// use kaubo_orchestrator::pipeline::module::ModuleId;
    ///
    /// let id = ModuleId::parse("math").unwrap();
    /// assert_eq!(id.components, vec!["math"]);
    ///
    /// let id = ModuleId::parse("math.utils").unwrap();
    /// assert_eq!(id.components, vec!["math", "utils"]);
    /// ```
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        if s.is_empty() {
            return Err(ParseError::EmptyModulePath);
        }
        
        // Check for invalid characters
        if !s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
            return Err(ParseError::InvalidModulePath(s.to_string()));
        }
        
        let components: Vec<String> = s.split('.').map(|s| s.to_string()).collect();
        
        // Check for empty components (e.g., "math..utils")
        if components.iter().any(|c| c.is_empty()) {
            return Err(ParseError::EmptyComponent(s.to_string()));
        }
        
        Ok(Self { components })
    }
    
    /// Convert to VFS logical path
    ///
    /// ["math", "utils"] -> "/mod/math.utils"
    pub fn to_vfs_path(&self) -> PathBuf {
        PathBuf::from(format!("/mod/{}", self.components.join(".")))
    }
    
    /// Convert to file system relative path
    ///
    /// ["math", "utils"] -> "math/utils.kaubo"
    pub fn to_file_path(&self) -> PathBuf {
        let mut path = PathBuf::new();
        
        // All components except last become directories
        if self.components.len() > 1 {
            for comp in &self.components[..self.components.len() - 1] {
                path.push(comp);
            }
        }
        
        // Last component becomes filename
        if let Some(last) = self.components.last() {
            path.push(format!("{}.kaubo", last));
        }
        
        path
    }
    
    /// Get the module name (last component)
    pub fn name(&self) -> &str {
        self.components.last()
            .map(|s| s.as_str())
            .unwrap_or("")
    }
    
    /// Get the parent module path
    ///
    /// ["math", "utils"] -> Some(["math"])
    pub fn parent(&self) -> Option<ModuleId> {
        if self.components.len() <= 1 {
            return None;
        }
        
        Some(ModuleId {
            components: self.components[..self.components.len() - 1].to_vec(),
        })
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.components.join("."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple() {
        let id = ModuleId::parse("math").unwrap();
        assert_eq!(id.components, vec!["math"]);
    }
    
    #[test]
    fn test_parse_nested() {
        let id = ModuleId::parse("math.utils").unwrap();
        assert_eq!(id.components, vec!["math", "utils"]);
    }
    
    #[test]
    fn test_parse_deep() {
        let id = ModuleId::parse("a.b.c.d").unwrap();
        assert_eq!(id.components, vec!["a", "b", "c", "d"]);
    }
    
    #[test]
    fn test_parse_empty() {
        assert!(matches!(
            ModuleId::parse(""),
            Err(ParseError::EmptyModulePath)
        ));
    }
    
    #[test]
    fn test_parse_empty_component() {
        assert!(matches!(
            ModuleId::parse("math..utils"),
            Err(ParseError::EmptyComponent(_))
        ));
    }
    
    #[test]
    fn test_parse_invalid_char() {
        assert!(matches!(
            ModuleId::parse("math/utils"),
            Err(ParseError::InvalidModulePath(_))
        ));
    }
    
    #[test]
    fn test_to_vfs_path() {
        let id = ModuleId::parse("math.utils").unwrap();
        assert_eq!(id.to_vfs_path(), PathBuf::from("/mod/math.utils"));
    }
    
    #[test]
    fn test_to_file_path() {
        let id = ModuleId::parse("math.utils").unwrap();
        assert_eq!(id.to_file_path(), PathBuf::from("math/utils.kaubo"));
    }
    
    #[test]
    fn test_to_file_path_simple() {
        let id = ModuleId::parse("math").unwrap();
        assert_eq!(id.to_file_path(), PathBuf::from("math.kaubo"));
    }
    
    #[test]
    fn test_name() {
        let id = ModuleId::parse("math.utils").unwrap();
        assert_eq!(id.name(), "utils");
    }
    
    #[test]
    fn test_parent() {
        let id = ModuleId::parse("math.utils.helper").unwrap();
        let parent = id.parent().unwrap();
        assert_eq!(parent.components, vec!["math", "utils"]);
    }
    
    #[test]
    fn test_parent_none() {
        let id = ModuleId::parse("math").unwrap();
        assert!(id.parent().is_none());
    }
    
    #[test]
    fn test_display() {
        let id = ModuleId::parse("math.utils").unwrap();
        assert_eq!(id.to_string(), "math.utils");
    }
}
