#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeInfo {
    /// 值类型（如int32、float64等）
    Value(ValueTypeInfo),
    /// 原生引用类型（如字符串、数组等语言内置类型）
    NativeReference(NativeReferenceTypeInfo),
}

impl TypeInfo {
    /// 获取类型在内存中的大小（值类型为自身大小，引用类型为指针大小）
    pub fn size(&self) -> usize {
        match self {
            TypeInfo::Value(vti) => vti.size(),
            // 引用类型在运行时以指针形式存储（64位系统为8字节）
            TypeInfo::NativeReference(_) => 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueTypeKind {
    Int64,
    Int32,
    Int16,
    Int8,
    UInt64,
    UInt32,
    UInt16,
    UInt8,
    Float64,
    Float32,
    Boolean,
    Unit, // 无返回值
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueTypeInfo {
    pub(crate) kind: ValueTypeKind,
}

impl ValueTypeInfo {
    /// 由类型自动推导大小，避免手动设置错误
    fn size(&self) -> usize {
        match self.kind {
            ValueTypeKind::Int64 | ValueTypeKind::UInt64 | ValueTypeKind::Float64 => 8,
            ValueTypeKind::Int32 | ValueTypeKind::UInt32 | ValueTypeKind::Float32 => 4,
            ValueTypeKind::Int16 | ValueTypeKind::UInt16 => 2,
            ValueTypeKind::Int8 | ValueTypeKind::UInt8 | ValueTypeKind::Boolean => 1,
            ValueTypeKind::Unit => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeReferenceTypeKind {
    String,
    // 数组需关联元素类型（泛型）
    Array(Box<TypeInfo>),
    // 字典需关联键和值类型（泛型）
    Dictionary(Box<TypeInfo>, Box<TypeInfo>),
    NativeFunction, // 原生函数（如系统调用）
    Type,           // 元类型（表示"类型本身"，如int32的类型是Type）
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeReferenceTypeInfo {
    kind: NativeReferenceTypeKind,
}
