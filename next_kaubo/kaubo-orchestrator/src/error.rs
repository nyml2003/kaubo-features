//! Error types for the orchestrator

use thiserror::Error;
use std::fmt;

/// Main orchestrator error type
#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("configuration error: {0}")]
    Config(String),
    
    #[error("loader error [{name}]: {source}")]
    LoaderError {
        name: String,
        #[source]
        source: LoaderError,
    },
    
    #[error("adaptive parser error [{name}]: {source}")]
    AdaptiveParserError {
        name: String,
        #[source]
        source: AdaptiveParserError,
    },
    
    #[error("pass error [{name}]: {source}")]
    PassError {
        name: String,
        #[source]
        source: PassError,
    },
    
    #[error("emitter error [{name}]: {source}")]
    EmitterError {
        name: String,
        #[source]
        source: EmitterError,
    },
    
    #[error("component not found: {kind} '{name}'")]
    ComponentNotFound {
        kind: String,
        name: String,
    },
    
    #[error("pipeline error: {message}")]
    PipelineError {
        message: String,
    },
    
    #[error("incomplete pipeline: cannot transition from '{from}' to '{to}'")]
    IncompleteChain {
        from: String,
        to: String,
    },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("serialization error: {0}")]
    Serialization(String),
}

/// Error type for loader components
#[derive(Error, Debug)]
pub enum LoaderError {
    #[error("source not found: {0}")]
    SourceNotFound(String),
    
    #[error("invalid source kind: expected {expected}, got {actual}")]
    InvalidSourceKind {
        expected: String,
        actual: String,
    },
    
    #[error("read failed: {0}")]
    ReadFailed(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Error type for adaptive parser components
#[derive(Error, Debug)]
pub enum AdaptiveParserError {
    #[error("invalid input format: expected {expected}, got {actual}")]
    InvalidInputFormat {
        expected: String,
        actual: String,
    },
    
    #[error("parse error: {0}")]
    ParseError(String),
    
    #[error("parsing failed: {0}")]
    ParsingFailed(String),
}

/// Error type for pass components
#[derive(Error, Debug)]
pub enum PassError {
    #[error("invalid input: {message}")]
    InvalidInput {
        message: String,
    },
    
    #[error("transform failed: {0}")]
    TransformFailed(String),
    
    #[error("validation error: {0}")]
    ValidationError(String),
    
    #[error("internal error: {0}")]
    Internal(String),
}

/// Error type for emitter components
#[derive(Error, Debug)]
pub enum EmitterError {
    #[error("invalid output format: {0}")]
    InvalidOutputFormat(String),
    
    #[error("write failed: {0}")]
    WriteFailed(String),
    
    #[error("target not found: {0}")]
    TargetNotFound(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<serde_json::Error> for OrchestratorError {
    fn from(err: serde_json::Error) -> Self {
        OrchestratorError::Serialization(err.to_string())
    }
}
