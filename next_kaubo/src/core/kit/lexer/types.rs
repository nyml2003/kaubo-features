//! 通用位置类型

/// 源代码坐标
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coordinate {
    pub line: usize,
    pub column: usize,
}

impl Default for Coordinate {
    fn default() -> Self {
        Self { line: 1, column: 1 }
    }
}
