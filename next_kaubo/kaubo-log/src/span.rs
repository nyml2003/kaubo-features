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

/// 用于跟踪 span 嵌套关系的栈
#[derive(Debug)]
pub struct SpanStack {
    spans: alloc::vec::Vec<Span>,
}

impl SpanStack {
    /// 创建新的空栈
    pub fn new() -> Self {
        SpanStack { spans: alloc::vec::Vec::new() }
    }

    /// 压入一个新的 span
    pub fn push(&mut self, span: Span) {
        self.spans.push(span);
    }

    /// 弹出栈顶的 span
    pub fn pop(&mut self) -> Option<Span> {
        self.spans.pop()
    }

    /// 查看栈顶的 span（不弹出）
    pub fn current(&self) -> Option<&Span> {
        self.spans.last()
    }

    /// 获取当前 span ID（如果有）
    pub fn current_id(&self) -> Option<SpanId> {
        self.current().map(|s| s.id)
    }

    /// 获取栈深度
    pub fn depth(&self) -> usize {
        self.spans.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// 清空栈
    pub fn clear(&mut self) {
        self.spans.clear();
    }
}

impl Default for SpanStack {
    fn default() -> Self {
        Self::new()
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

    #[test]
    fn test_span_stack() {
        let mut stack = SpanStack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.depth(), 0);

        stack.push(Span::new(SpanId(1), "outer"));
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.current_id(), Some(SpanId(1)));

        stack.push(Span::new(SpanId(2), "inner"));
        assert_eq!(stack.depth(), 2);
        assert_eq!(stack.current_id(), Some(SpanId(2)));

        let popped = stack.pop();
        assert_eq!(popped.map(|s| s.id), Some(SpanId(2)));
        assert_eq!(stack.depth(), 1);

        stack.clear();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_span_stack_empty_pop() {
        let mut stack = SpanStack::new();
        assert_eq!(stack.pop(), None);
        assert_eq!(stack.current(), None);
        assert_eq!(stack.current_id(), None);
    }
}
