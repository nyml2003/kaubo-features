//! Component trait definitions
//!
//! This module defines the base `Component` trait and related types
//! that all orchestrator components (Loaders, AdaptiveParsers, Passes, Emitters) implement.

/// The kind of component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentKind {
    /// Loader: reads input from external sources
    Loader,
    /// AdaptiveParser: parses raw data into initial IR
    AdaptiveParser,
    /// Pass: transforms intermediate representation (IR)
    Pass,
    /// Emitter: writes output to targets
    Emitter,
}

impl fmt::Display for ComponentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComponentKind::Loader => write!(f, "loader"),
            ComponentKind::AdaptiveParser => write!(f, "adaptive_parser"),
            ComponentKind::Pass => write!(f, "pass"),
            ComponentKind::Emitter => write!(f, "emitter"),
        }
    }
}

use std::fmt;
use crate::adaptive_parser::DataFormat;

/// Metadata about a component
#[derive(Debug, Clone)]
pub struct ComponentMetadata {
    /// The component name (unique identifier)
    pub name: &'static str,
    /// The component version
    pub version: &'static str,
    /// The kind of component
    pub kind: ComponentKind,
    /// Optional description
    pub description: Option<&'static str>,
}

impl ComponentMetadata {
    /// Create new metadata
    pub fn new(
        name: &'static str,
        version: &'static str,
        kind: ComponentKind,
        description: Option<&'static str>,
    ) -> Self {
        Self {
            name,
            version,
            kind,
            description,
        }
    }
}

/// Capabilities declared by a component
#[derive(Debug, Clone)]
pub struct Capabilities {
    /// Input formats this component can accept
    pub inputs: Vec<DataFormat>,
    /// Output formats this component can produce
    pub outputs: Vec<DataFormat>,
}

impl Capabilities {
    /// Create new capabilities
    pub fn new(inputs: Vec<DataFormat>, outputs: Vec<DataFormat>) -> Self {
        Self { inputs, outputs }
    }
    
    /// Check if this component can accept the given input format
    pub fn can_accept(&self, format: &DataFormat) -> bool {
        self.inputs.is_empty() || self.inputs.contains(format)
    }
    
    /// Check if this component can produce the given output format
    pub fn can_produce(&self, format: &DataFormat) -> bool {
        self.outputs.contains(format)
    }
}

/// The base trait for all orchestrator components
///
/// All components (Loaders, AdaptiveParsers, Passes, Emitters) implement this trait.
pub trait Component: Send + Sync {
    /// Get the component metadata
    fn metadata(&self) -> ComponentMetadata;
    
    /// Get the component capabilities
    fn capabilities(&self) -> Capabilities;
    
    /// Validate configuration for this component (optional)
    ///
    /// Default implementation always returns Ok(())
    fn validate_config(&self, _config: &serde_json::Value) -> Result<(), String> {
        Ok(())
    }
}

/// Helper trait for downcasting components
pub trait ComponentExt: Component {
    /// Check if this component is of the given kind
    fn is_kind(&self, kind: ComponentKind) -> bool {
        self.metadata().kind == kind
    }
    
    /// Get the component name
    fn name(&self) -> &'static str {
        self.metadata().name
    }
    
    /// Check if this component can transition from the given format
    fn can_transition_from(&self, format: &DataFormat) -> bool {
        self.capabilities().can_accept(format)
    }
    
    /// Check if this component can transition to the given format
    fn can_transition_to(&self, format: &DataFormat) -> bool {
        self.capabilities().can_produce(format)
    }
}

impl<T: Component + ?Sized> ComponentExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestComponent;

    impl Component for TestComponent {
        fn metadata(&self) -> ComponentMetadata {
            ComponentMetadata::new(
                "test",
                "1.0.0",
                ComponentKind::Pass,
                Some("Test component"),
            )
        }

        fn capabilities(&self) -> Capabilities {
            Capabilities::new(vec![DataFormat::Source], vec![DataFormat::Ast])
        }
    }

    #[test]
    fn test_component_metadata() {
        let comp = TestComponent;
        let meta = comp.metadata();
        
        assert_eq!(meta.name, "test");
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.kind, ComponentKind::Pass);
        assert_eq!(meta.description, Some("Test component"));
    }

    #[test]
    fn test_capabilities() {
        let comp = TestComponent;
        let caps = comp.capabilities();
        
        assert!(caps.can_accept(&DataFormat::Source));
        assert!(!caps.can_accept(&DataFormat::Ast));
        
        assert!(caps.can_produce(&DataFormat::Ast));
        assert!(!caps.can_produce(&DataFormat::Source));
    }
}
