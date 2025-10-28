use crate::runtime::{
    type_info::TypeInfo,
    visitor::{BinaryValueVisitor, ValueError},
};

/// 运行时的值容器（类型+数据）
#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    pub(crate) type_info: TypeInfo,
    pub(crate) value: ValueUnion,
}

/// 存储具体数据（值类型直接存，引用类型存指针）
#[derive(Debug, Clone, PartialEq)]
pub enum ValueUnion {
    // 值类型数据
    Int64(i64),
    Int32(i32),
    Int16(i16),
    Int8(i8),
    Uint64(u64),
    Uint32(u32),
    Uint16(u16),
    Uint8(u8),
    Float64(f64),
    Float32(f32),
    Boolean(bool),
    Unit,

    // 引用类型数据（存储堆内存地址，usize表示指针）
    Reference(usize),

    // 元类型（值为类型本身，如TypeInfo::int32）
    Type(Box<TypeInfo>),
}

impl Value {
    /// 接受二元运算访问者，将自身和另一个值交给访问者处理
    pub fn accept<V: BinaryValueVisitor>(
        &self,
        visitor: V,
        other: &Self,
    ) -> Result<Self, ValueError> {
        visitor.visit(self, other)
    }
}
