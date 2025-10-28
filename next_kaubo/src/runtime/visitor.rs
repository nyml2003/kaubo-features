use crate::runtime::{
    type_info::{TypeInfo, ValueTypeInfo, ValueTypeKind},
    value::{Value, ValueUnion},
};

#[derive(Debug, PartialEq, Clone)]
pub enum ValueError {
    /// 左右类型不兼容（携带具体类型信息）
    IncompatibleTypes(TypeInfo, TypeInfo),
    /// 类型不支持当前操作（携带类型信息）
    UnsupportedType(TypeInfo),
}

/// 二元运算访问者：直接操作Value，通过type_info分发逻辑
pub trait BinaryValueVisitor {
    /// 执行二元运算
    /// 参数：左操作数、右操作数
    fn visit(&self, left: &Value, right: &Value) -> Result<Value, ValueError>;
}

/// 加法运算的具体实现（处理所有支持加法的Value类型）
pub struct AddVisitor;

impl BinaryValueVisitor for AddVisitor {
    fn visit(&self, left: &Value, right: &Value) -> Result<Value, ValueError> {
        // 先检查左右类型是否一致（基础类型安全校验）
        if left.type_info != right.type_info {
            return Err(ValueError::IncompatibleTypes(
                left.type_info.clone(),
                right.type_info.clone(),
            ));
        }

        // 根据类型信息和实际值执行加法
        match (&left.type_info, &left.value, &right.value) {
            // 处理int32 + int32
            (
                TypeInfo::Value(ValueTypeInfo {
                    kind: ValueTypeKind::Int32,
                }),
                ValueUnion::Int32(a),
                ValueUnion::Int32(b),
            ) => Ok(Value {
                type_info: left.type_info.clone(),
                value: ValueUnion::Int32(a + b),
            }),

            // 其他类型（如引用类型、布尔等）不支持加法
            (type_info, _, _) => Err(ValueError::UnsupportedType(type_info.clone())),
        }
    }
}
