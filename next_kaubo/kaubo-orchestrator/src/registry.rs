//! Component registries
//!
//! This module provides registries for managing different types of components.

use crate::component::{Component, ComponentKind};
use crate::loader::Loader;
use crate::converter::{Converter, DataFormat};
use crate::pass::Pass;
use crate::emitter::Emitter;
use std::collections::HashMap;

/// Generic registry for components
/// 
/// Note: This registry stores boxed trait objects. When using with dyn types
/// like `dyn Loader`, use `Registry<dyn Loader>` as the type alias.
pub struct Registry<T: Component + ?Sized> {
    components: HashMap<String, Box<T>>,
}

impl<T: Component + ?Sized> Default for Registry<T> {
    fn default() -> Self {
        Self {
            components: HashMap::new(),
        }
    }
}

impl<T: Component + ?Sized> std::fmt::Debug for Registry<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry")
            .field("count", &self.components.len())
            .finish()
    }
}

impl<T: Component + ?Sized> Registry<T> {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }
    
    /// Register a component
    pub fn register(&mut self, component: Box<T>) {
        let name = component.metadata().name.to_string();
        self.components.insert(name, component);
    }
    
    /// Get a component by name
    pub fn get(&self, name: &str) -> Option<&T> {
        self.components.get(name).map(|b| b.as_ref())
    }
    
    /// Get a mutable reference to a component
    pub fn get_mut(&mut self, name: &str) -> Option<&mut T> {
        self.components.get_mut(name).map(|b| b.as_mut())
    }
    
    /// Check if a component exists
    pub fn contains(&self, name: &str) -> bool {
        self.components.contains_key(name)
    }
    
    /// Remove a component
    pub fn remove(&mut self, name: &str) -> Option<Box<T>> {
        self.components.remove(name)
    }
    
    /// Get all component names
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.components.keys().map(|s| s.as_str())
    }
    
    /// Get all components
    pub fn all(&self) -> impl Iterator<Item = &T> {
        self.components.values().map(|b| b.as_ref())
    }
    
    /// Get the number of registered components
    pub fn len(&self) -> usize {
        self.components.len()
    }
    
    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
    
    /// Find components by capability
    pub fn find_by_input(&self, input: &DataFormat) -> impl Iterator<Item = &T> + '_ {
        let input = input.clone();
        self.all().filter(move |c| c.capabilities().can_accept(&input))
    }
    
    /// Find components by output
    pub fn find_by_output(&self, output: &DataFormat) -> impl Iterator<Item = &T> + '_ {
        let output = output.clone();
        self.all().filter(move |c| c.capabilities().can_produce(&output))
    }
}

/// Registry for Loader components
pub type LoaderRegistry = Registry<dyn Loader>;

/// Registry for Converter components
pub type ConverterRegistry = Registry<dyn Converter>;

/// Registry for Pass components
pub type PassRegistry = Registry<dyn Pass>;

/// Registry for Emitter components
pub type EmitterRegistry = Registry<dyn Emitter>;

/// Combined registry for all components
#[derive(Default)]
pub struct ComponentRegistry {
    pub loaders: LoaderRegistry,
    pub converters: ConverterRegistry,
    pub passes: PassRegistry,
    pub emitters: EmitterRegistry,
}

impl std::fmt::Debug for ComponentRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentRegistry")
            .field("loaders_count", &self.loaders.len())
            .field("converters_count", &self.converters.len())
            .field("passes_count", &self.passes.len())
            .field("emitters_count", &self.emitters.len())
            .finish()
    }
}

impl ComponentRegistry {
    /// Create a new empty combined registry
    pub fn new() -> Self {
        Self {
            loaders: LoaderRegistry::new(),
            converters: ConverterRegistry::new(),
            passes: PassRegistry::new(),
            emitters: EmitterRegistry::new(),
        }
    }
    
    /// Get total component count
    pub fn total_count(&self) -> usize {
        self.loaders.len() + self.converters.len() + self.passes.len() + self.emitters.len()
    }
    
    /// Print registry info
    pub fn print_info(&self) {
        println!("Registered Components:");
        println!("  Loaders: {}", self.loaders.len());
        for name in self.loaders.names() {
            println!("    - {}", name);
        }
        println!("  Converters: {}", self.converters.len());
        for name in self.converters.names() {
            println!("    - {}", name);
        }
        println!("  Passes: {}", self.passes.len());
        for name in self.passes.names() {
            println!("    - {}", name);
        }
        println!("  Emitters: {}", self.emitters.len());
        for name in self.emitters.names() {
            println!("    - {}", name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::{ComponentMetadata, Capabilities, ComponentKind};
    use crate::loader::{Source, RawData};
    use crate::error::LoaderError;

    struct TestLoader {
        name: &'static str,
    }

    impl Component for TestLoader {
        fn metadata(&self) -> ComponentMetadata {
            ComponentMetadata::new(
                self.name,
                "1.0.0",
                ComponentKind::Loader,
                None,
            )
        }
        
        fn capabilities(&self) -> Capabilities {
            Capabilities::new(vec![], vec![DataFormat::Text])
        }
    }

    impl Loader for TestLoader {
        fn load(&self, _source: &Source) -> Result<RawData, LoaderError> {
            Ok(RawData::Text("test".to_string()))
        }
    }

    #[test]
    fn test_registry() {
        let mut registry = LoaderRegistry::new();
        
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        
        registry.register(Box::new(TestLoader { name: "test1" }));
        registry.register(Box::new(TestLoader { name: "test2" }));
        
        assert_eq!(registry.len(), 2);
        assert!(registry.contains("test1"));
        assert!(registry.contains("test2"));
        
        let comp = registry.get("test1").unwrap();
        assert_eq!(comp.metadata().name, "test1");
    }

    #[test]
    fn test_component_registry() {
        let mut registry = ComponentRegistry::new();
        
        registry.loaders.register(Box::new(TestLoader { name: "file" }));
        registry.passes.register(Box::new(TestPass { name: "parser" }));
        
        assert_eq!(registry.total_count(), 2);
    }

    struct TestPass {
        name: &'static str,
    }

    impl Component for TestPass {
        fn metadata(&self) -> ComponentMetadata {
            ComponentMetadata::new(
                self.name,
                "1.0.0",
                ComponentKind::Pass,
                None,
            )
        }
        
        fn capabilities(&self) -> Capabilities {
            Capabilities::new(vec![DataFormat::Source], vec![DataFormat::Ast])
        }
    }

    impl Pass for TestPass {
        fn input_format(&self) -> crate::converter::DataFormat {
            crate::converter::DataFormat::Source
        }
        
        fn output_format(&self) -> crate::converter::DataFormat {
            crate::converter::DataFormat::Ast
        }
        
        fn run(&self, _input: crate::pass::Input, _ctx: &crate::pass::PassContext) -> Result<crate::pass::Output, crate::error::PassError> {
            Ok(crate::pass::Output::new(crate::converter::IR::Source("".to_string())))
        }
    }
}
