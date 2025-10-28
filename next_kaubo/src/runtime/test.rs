use crate::runtime::{
    type_info::{TypeInfo, ValueTypeInfo, ValueTypeKind},
    value::{Value, ValueUnion},
};

fn build_int32(value: i32) -> Value {
    let int32_type = TypeInfo::Value(ValueTypeInfo {
        kind: ValueTypeKind::Int32,
    });
    let int32_value = Value {
        type_info: int32_type,
        value: ValueUnion::Int32(value),
    };
    int32_value
}

#[cfg(test)]
mod tests {
    use crate::runtime::visitor::AddVisitor;

    use super::*;

    #[test]
    fn test_value_type_size() {
        let a = build_int32(123);
        let b = build_int32(456);
        println!("int32 var: {:?}", a.accept(AddVisitor, &b));
    }
}
