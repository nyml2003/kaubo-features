use std::fmt;
/// 事件类型（输入字符）
pub type Event = char;

// 定义组合 trait，包含所有需要的约束
pub trait TokenKindTrait:
    fmt::Debug + Clone + PartialEq + Ord + PartialOrd + Into<u8> + Send + Sync
{
}
