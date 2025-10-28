#[derive(Debug, PartialEq)]
pub enum ValueError {
    /// 类型不兼容（如int32与float64相加）
    IncompatibleTypes,
    /// 不支持的操作（如字符串与整数相加）
    UnsupportedOperation,
}
