//! Span 跟踪实现（no_std 兼容）

/// Span ID（唯一标识符）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpanId(pub u64);

/// Span 表示一个代码执行上下文
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    /// Span ID
    pub id: SpanId,
    /// Span名称（通常是函数或操作名）
    pub name: &'static str,
}

impl Span {
    /// 创建新的 Span
    pub const fn new(id: SpanId, name: &'static str) -> Self {
        Span { id, name }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let span = Span::new(SpanId(1), "compile");
        assert_eq!(span.id, SpanId(1));
        assert_eq!(span.name, "compile");
    }
}
