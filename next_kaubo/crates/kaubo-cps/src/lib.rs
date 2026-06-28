//! CPS (Continuation-Passing Style) contract types.
//!
#![allow(clippy::type_complexity)]
//! This crate owns the data contract shared by lowering, optimization, and VM
//! execution. It does not lower, optimize, encode, or execute programs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpsModule {
    pub functions: Vec<CpsFunction>,
    pub constants: Vec<Constant>,
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub vtables: Vec<VtableDef>,
    /// (module_path, export_name) → global_func_idx
    /// Used for dynamic import symbol lookup at runtime.
    #[serde(default)]
    pub symbol_map: HashMap<(String, String), usize>,
    /// func_owners[i] = 原始模块路径（函数 i 归属哪个模块）。
    #[serde(default)]
    pub func_owners: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VtableDef {
    pub interface_name: String,
    pub struct_name: String,
    pub methods: Vec<(String, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpsFunction {
    pub name: String,
    pub blocks: Vec<CpsBlock>,
    pub entry: usize,
    pub reg_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpsBlock {
    pub id: usize,
    pub params: Vec<usize>,
    pub instrs: Vec<CpsInstr>,
    pub term: CpsTerminator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CpsInstr {
    BinOp(usize, CpsBinOp, usize, usize),
    UnOp(usize, CpsUnOp, usize),
    LoadConst(usize, usize),
    Move(usize, usize),
    NewStruct(usize, usize, Vec<usize>),
    GetField(usize, usize, u16),
    SetField(usize, usize, u16, usize),
    NewVariant(usize, usize, u16, Vec<usize>),
    GetVariantTag(usize, usize),
    GetVariantField(usize, usize, u16),
    SetVariantField(usize, usize, u16, usize),
    NewList(usize, Vec<usize>),
    ListLen(usize, usize),
    IndexGet(usize, usize, usize),
    IndexSet(usize, usize, usize, usize),
    NewTuple(usize, Vec<usize>),
    TupleIndex(usize, usize, u16),
    NewInt64Array(usize, Vec<usize>),
    NewFloat64Array(usize, Vec<usize>),
    Box(usize, usize),
    Unbox(usize, usize),
    Print(usize),
    LoadVtable(usize, usize),
    NewInterfaceObj(usize, usize, usize),
    /// 从导入模块加载常量（LinkStage 解析为 LoadConst）。
    LoadExternalConst(usize, usize),
    Nop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CpsTerminator {
    Jump(usize, Vec<usize>),
    Branch(usize, usize, Vec<usize>, usize, Vec<usize>),
    Return(usize),
    Call(usize, Vec<usize>, usize),
    TailCall(usize, Vec<usize>),
    CallNative(usize, Vec<usize>, usize),
    CallIndirect(usize, Vec<usize>, usize),
    /// 静态导入的外部调用。import_handle = ImportTable.entries 中的索引。
    /// LinkStage 解析为 Call(global_idx, …)。
    CallExternal {
        import_handle: usize,
        args: Vec<usize>,
        ret_block: usize,
    },
    /// 动态导入的外部调用（预留，Phase 3b 不生成）。
    /// 运行时由 VM 的 ModuleRegistry 解析。
    CallExternalDynamic {
        module_expr: String,
        func_name: String,
        args: Vec<usize>,
        ret_block: usize,
    },
    Suspend,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CpsBinOp {
    AddInt,
    SubInt,
    MulInt,
    DivInt,
    ModInt,
    FAdd,
    FSub,
    FMul,
    FDiv,
    SAdd,
    EqInt,
    NeInt,
    LtInt,
    LeInt,
    GtInt,
    GeInt,
    FEq,
    FNe,
    FLt,
    FLe,
    FGt,
    FGe,
    IToF,
    FToI,
    IToS,
    FToS,
    SToI,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CpsUnOp {
    NegInt,
    FNeg,
    Not,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constant {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDef {
    pub id: usize,
    pub name: String,
    pub fields: Vec<(String, String)>,
    pub type_bitmap: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDef {
    pub id: usize,
    pub name: String,
    pub variants: Vec<(String, u16, Vec<(String, String)>)>,
    pub variant_type_bitmaps: Vec<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_round_trips_through_json() {
        let module = CpsModule {
            functions: vec![CpsFunction {
                name: "main".to_string(),
                blocks: vec![CpsBlock {
                    id: 0,
                    params: vec![1, 2],
                    instrs: vec![
                        CpsInstr::LoadConst(0, 0),
                        CpsInstr::BinOp(1, CpsBinOp::AddInt, 0, 0),
                        CpsInstr::Print(1),
                    ],
                    term: CpsTerminator::Return(1),
                }],
                entry: 0,
                reg_count: 3,
            }],
            constants: vec![Constant::Int(42), Constant::String("hello".to_string())],
            structs: vec![StructDef {
                id: 7,
                name: "Point".to_string(),
                fields: vec![("x".to_string(), "Int64".to_string())],
                type_bitmap: 0b101,
            }],
            enums: vec![],
            vtables: vec![],
            symbol_map: HashMap::new(),
            func_owners: vec![],
        };

        let json = serde_json::to_string(&module).unwrap();
        let decoded: CpsModule = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.functions.len(), 1);
        assert_eq!(decoded.functions[0].name, "main");
        assert_eq!(decoded.functions[0].blocks[0].params, vec![1, 2]);
        assert_eq!(decoded.constants.len(), 2);
        assert_eq!(decoded.structs[0].name, "Point");
        assert_eq!(decoded.structs[0].type_bitmap, 0b101);
        assert!(decoded.symbol_map.is_empty());
        assert!(decoded.func_owners.is_empty());
    }

    #[test]
    fn instruction_variants_are_constructible() {
        let instrs = [CpsInstr::Move(0, 1), CpsInstr::NewList(2, vec![0, 1])];
        let term = CpsTerminator::Suspend;

        assert!(matches!(instrs[0], CpsInstr::Move(0, 1)));
        assert!(matches!(instrs[1], CpsInstr::NewList(2, ref items) if items == &vec![0, 1]));
        assert!(matches!(term, CpsTerminator::Suspend));
    }

    #[test]
    fn call_external_variants_are_constructible() {
        let term = CpsTerminator::CallExternal {
            import_handle: 3,
            args: vec![0, 1],
            ret_block: 7,
        };
        assert!(matches!(term, CpsTerminator::CallExternal {
            import_handle: 3,
            args: ref a,
            ret_block: 7,
        } if a == &vec![0, 1]));

        let dynamic_term = CpsTerminator::CallExternalDynamic {
            module_expr: "./math.kb".to_string(),
            func_name: "add".to_string(),
            args: vec![2, 3],
            ret_block: 8,
        };
        assert!(matches!(dynamic_term, CpsTerminator::CallExternalDynamic {
            module_expr: ref m,
            func_name: ref f,
            ..
        } if m == "./math.kb" && f == "add"));
    }

    #[test]
    fn new_tuple_and_tuple_index_constructible() {
        let t = CpsInstr::NewTuple(0, vec![1, 2, 3]);
        assert!(matches!(t, CpsInstr::NewTuple(0, ref regs) if regs == &vec![1, 2, 3]));

        let idx = CpsInstr::TupleIndex(5, 0, 1);
        assert!(matches!(idx, CpsInstr::TupleIndex(5, 0, 1)));
    }
}
