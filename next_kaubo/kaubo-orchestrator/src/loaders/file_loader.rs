//! File Loader - 从文件系统加载源代码

use crate::component::{Capabilities, Component, ComponentKind, ComponentMetadata};
use crate::converter::DataFormat;
use crate::error::LoaderError;
use crate::loader::{Loader, RawData, Source, SourceKind};

/// 文件加载器组件
pub struct FileLoader;

impl FileLoader {
    /// 创建新的文件加载器
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for FileLoader {
    fn metadata(&self) -> ComponentMetadata {
        ComponentMetadata::new(
            "file_loader",
            "0.1.0",
            ComponentKind::Loader,
            Some("从文件系统加载源代码"),
        )
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::new(
            vec![DataFormat::Custom("file".to_string())],
            vec![DataFormat::Source]
        )
    }
}

impl Loader for FileLoader {
    fn load(&self, source: &Source) -> Result<RawData, LoaderError> {
        match &source.kind {
            SourceKind::File => {
                if let Some(path) = &source.path {
                    std::fs::read_to_string(path)
                        .map(RawData::Text)
                        .map_err(|e| LoaderError::ReadFailed(format!("无法读取文件 '{}': {}", path.display(), e)))
                } else {
                    Err(LoaderError::InvalidSourceKind {
                        expected: "file with path".to_string(),
                        actual: "file without path".to_string(),
                    })
                }
            }
            _ => Err(LoaderError::InvalidSourceKind {
                expected: "file".to_string(),
                actual: format!("{:?}", source.kind),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_loader_metadata() {
        let loader = FileLoader::new();
        let metadata = loader.metadata();

        assert_eq!(metadata.name, "file_loader");
        assert_eq!(metadata.kind, ComponentKind::Loader);
    }

    #[test]
    fn test_file_loader_capabilities() {
        let loader = FileLoader::new();
        let caps = loader.capabilities();

        assert!(caps.can_accept(&DataFormat::Custom("file".to_string())));
        assert!(caps.can_produce(&DataFormat::Source));
    }
}
