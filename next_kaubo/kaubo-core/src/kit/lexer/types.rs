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

/// 源代码范围（span）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Coordinate,
    pub end: Coordinate,
}

impl Span {
    /// 创建新的 span
    pub fn new(start: Coordinate, end: Coordinate) -> Self {
        Self { start, end }
    }

    /// 从单个坐标创建 span（用于单 token）
    pub fn at(coord: Coordinate) -> Self {
        Self {
            start: coord,
            end: coord,
        }
    }

    /// 合并两个 span
    pub fn merge(&self, other: &Span) -> Self {
        Self {
            start: self.start,
            end: other.end,
        }
    }
}

impl Default for Span {
    fn default() -> Self {
        Self {
            start: Coordinate::default(),
            end: Coordinate::default(),
        }
    }
}
