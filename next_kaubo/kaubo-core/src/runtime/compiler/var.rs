//! 变量解析和管理

use super::{CompileError, Compiler};

/// 局部变量信息
#[derive(Debug, Clone)]
pub struct Local {
    pub name: String,
    pub depth: usize,
    pub is_initialized: bool,
    pub is_captured: bool, // 是否被内层闭包捕获
}

/// Upvalue 描述（编译时）
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Upvalue {
    pub name: String,
    pub index: u8,      // upvalue 索引
    pub is_local: bool, // true=捕获外层局部变量, false=继承外层的 upvalue
}

/// 变量类型（解析结果）
#[derive(Debug, Clone, Copy)]
pub enum Variable {
    Local(u8),   // 局部变量索引
    Upvalue(u8), // upvalue 索引
}

/// 进入新作用域
pub fn begin_scope(compiler: &mut Compiler) {
    compiler.scope_depth += 1;
}

/// 退出作用域，返回弹出的变量数量
pub fn end_scope(compiler: &mut Compiler) -> usize {
    compiler.scope_depth -= 1;

    let mut popped = 0;
    while let Some(local) = compiler.locals.last() {
        if local.depth <= compiler.scope_depth {
            break;
        }
        compiler.locals.pop();
        popped += 1;
    }
    popped
}

/// 添加局部变量，返回其在栈中的索引
pub fn add_local(compiler: &mut Compiler, name: &str) -> Result<u8, CompileError> {
    // 检查局部变量数量上限
    if compiler.locals.len() >= 256 {
        return Err(CompileError::TooManyLocals);
    }

    // 检查同作用域内是否已有同名变量
    for local in compiler.locals.iter().rev() {
        if local.depth < compiler.scope_depth {
            break;
        }
        if local.name == name {
            return Err(CompileError::VariableAlreadyExists(name.to_string()));
        }
    }

    compiler.locals.push(Local {
        name: name.to_string(),
        depth: compiler.scope_depth,
        is_initialized: false,
        is_captured: false,
    });

    // 更新最大局部变量数
    compiler.max_locals = compiler.max_locals.max(compiler.locals.len());

    Ok((compiler.locals.len() - 1) as u8)
}

/// 标记最后一个变量为已初始化
pub fn mark_initialized(compiler: &mut Compiler) {
    if let Some(local) = compiler.locals.last_mut() {
        local.is_initialized = true;
    }
}

/// 解析变量名，返回其在局部变量表中的索引
pub fn resolve_local(compiler: &Compiler, name: &str) -> Option<u8> {
    for (i, local) in compiler.locals.iter().enumerate().rev() {
        if local.name == name {
            if !local.is_initialized {
                // 不能在初始化完成前使用
                return None;
            }
            return Some(i as u8);
        }
    }
    None
}

/// 标记局部变量被捕获
pub fn mark_captured(compiler: &mut Compiler, index: usize) {
    if let Some(local) = compiler.locals.get_mut(index) {
        local.is_captured = true;
    }
}

/// 添加 upvalue 描述，返回其索引
pub fn add_upvalue(compiler: &mut Compiler, name: &str, index: u8, is_local: bool) -> u8 {
    // 检查是否已存在相同的 upvalue
    for (i, upvalue) in compiler.upvalues.iter().enumerate() {
        if upvalue.index == index && upvalue.is_local == is_local {
            return i as u8;
        }
    }

    let upvalue = Upvalue {
        name: name.to_string(),
        index,
        is_local,
    };
    compiler.upvalues.push(upvalue);
    (compiler.upvalues.len() - 1) as u8
}

/// 递归解析 upvalue
/// 返回 Some(idx) 如果是 upvalue，None 如果未找到
pub fn resolve_upvalue(compiler: &mut Compiler, name: &str) -> Option<u8> {
    // 没有外层编译器，无法解析
    if compiler.enclosing.is_null() {
        return None;
    }

    unsafe {
        // 1. 在外层编译器查找局部变量
        if let Some(local_idx) = (*compiler.enclosing).resolve_local(name) {
            // 找到了！标记为被捕获
            (*compiler.enclosing).mark_captured(local_idx as usize);
            // 添加 upvalue 描述（指向外层局部变量）
            return Some(add_upvalue(compiler, name, local_idx, true));
        }

        // 2. 递归在外层查找 upvalue
        if let Some(upvalue_idx) = (*compiler.enclosing).resolve_upvalue(name) {
            // 继承外层的 upvalue
            return Some(add_upvalue(compiler, name, upvalue_idx, false));
        }
    }

    None
}

/// 统一变量解析：Local 或 Upvalue
pub fn resolve_variable(compiler: &mut Compiler, name: &str) -> Option<Variable> {
    // 1. 先查找局部变量
    if let Some(idx) = resolve_local(compiler, name) {
        return Some(Variable::Local(idx));
    }

    // 2. 查找 upvalue（需要可变引用，因为可能添加 upvalue 描述）
    if let Some(idx) = resolve_upvalue(compiler, name) {
        return Some(Variable::Upvalue(idx));
    }

    None
}

/// 发射常量加载指令 (优化：使用 LoadConst0-15)
pub fn emit_constant(compiler: &mut Compiler, idx: u8) {
    use crate::core::OpCode;
    
    let op = match idx {
        0 => OpCode::LoadConst0,
        1 => OpCode::LoadConst1,
        2 => OpCode::LoadConst2,
        3 => OpCode::LoadConst3,
        4 => OpCode::LoadConst4,
        5 => OpCode::LoadConst5,
        6 => OpCode::LoadConst6,
        7 => OpCode::LoadConst7,
        8 => OpCode::LoadConst8,
        9 => OpCode::LoadConst9,
        10 => OpCode::LoadConst10,
        11 => OpCode::LoadConst11,
        12 => OpCode::LoadConst12,
        13 => OpCode::LoadConst13,
        14 => OpCode::LoadConst14,
        15 => OpCode::LoadConst15,
        _ => {
            compiler.chunk.write_op_u8(OpCode::LoadConst, idx, 0);
            return;
        }
    };
    compiler.chunk.write_op(op, 0);
}

/// 发射局部变量加载指令
pub fn emit_load_local(compiler: &mut Compiler, idx: u8) {
    use crate::core::OpCode;
    
    match idx {
        0 => compiler.chunk.write_op(OpCode::LoadLocal0, 0),
        1 => compiler.chunk.write_op(OpCode::LoadLocal1, 0),
        2 => compiler.chunk.write_op(OpCode::LoadLocal2, 0),
        3 => compiler.chunk.write_op(OpCode::LoadLocal3, 0),
        4 => compiler.chunk.write_op(OpCode::LoadLocal4, 0),
        5 => compiler.chunk.write_op(OpCode::LoadLocal5, 0),
        6 => compiler.chunk.write_op(OpCode::LoadLocal6, 0),
        7 => compiler.chunk.write_op(OpCode::LoadLocal7, 0),
        _ => compiler.chunk.write_op_u8(OpCode::LoadLocal, idx, 0),
    }
}

/// 发射局部变量存储指令
pub fn emit_store_local(compiler: &mut Compiler, idx: u8) {
    use crate::core::OpCode;
    
    match idx {
        0 => compiler.chunk.write_op(OpCode::StoreLocal0, 0),
        1 => compiler.chunk.write_op(OpCode::StoreLocal1, 0),
        2 => compiler.chunk.write_op(OpCode::StoreLocal2, 0),
        3 => compiler.chunk.write_op(OpCode::StoreLocal3, 0),
        4 => compiler.chunk.write_op(OpCode::StoreLocal4, 0),
        5 => compiler.chunk.write_op(OpCode::StoreLocal5, 0),
        6 => compiler.chunk.write_op(OpCode::StoreLocal6, 0),
        7 => compiler.chunk.write_op(OpCode::StoreLocal7, 0),
        _ => compiler.chunk.write_op_u8(OpCode::StoreLocal, idx, 0),
    }
}

/// 发射 upvalue 加载指令
pub fn emit_load_upvalue(compiler: &mut Compiler, idx: u8) {
    use crate::core::OpCode;
    compiler.chunk.write_op_u8(OpCode::GetUpvalue, idx, 0);
}

/// 发射 upvalue 存储指令
pub fn emit_store_upvalue(compiler: &mut Compiler, idx: u8) {
    use crate::core::OpCode;
    compiler.chunk.write_op_u8(OpCode::SetUpvalue, idx, 0);
}
