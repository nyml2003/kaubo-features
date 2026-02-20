//! Chunk 二进制编码/解码
//!
//! 将 Chunk 序列化为二进制格式，支持：
//! - 字节码 (code)
//! - 常量池 (constants) - 支持堆对象引用
//! - 行号信息 (lines)
//! - 方法表 (method_table)
//! - 运算符表 (operator_table)
//! - 内联缓存槽位 (inline_cache_slots)
//! - 内联缓存条目 (inline_caches)

use crate::vm::binary::data::{FunctionEntry, FunctionPool, ShapeEntry, ShapeTable, StringPool};
use crate::vm::core::bytecode::{InlineCacheSlot, MethodTableEntry, OperatorTableEntry};
use crate::vm::core::chunk::Chunk;
use crate::vm::core::object::{ObjFunction, ObjShape, ObjStruct};
use crate::vm::core::value::Value;

/// 编码上下文 - 用于在编码过程中访问和注册到各个 Pool
pub struct EncodeContext<'a> {
    /// 字符串池（用于字符串常量）
    pub string_pool: &'a mut StringPool,
    /// 函数池（用于函数字面量）
    pub function_pool: &'a mut FunctionPool,
    /// Shape 表（用于结构体定义）
    pub shape_table: &'a mut ShapeTable,
}

impl<'a> EncodeContext<'a> {
    /// 创建新的编码上下文
    pub fn new(string_pool: &'a mut StringPool, function_pool: &'a mut FunctionPool, shape_table: &'a mut ShapeTable) -> Self {
        Self {
            string_pool,
            function_pool,
            shape_table,
        }
    }
}

impl<'a> std::fmt::Debug for EncodeContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncodeContext")
            .field("string_pool_len", &self.string_pool.len())
            .field("function_pool_len", &self.function_pool.len())
            .field("shape_table_len", &self.shape_table.len())
            .finish()
    }
}

/// 解码上下文 - 用于在解码过程中从各个 Pool 解析引用
pub struct DecodeContext<'a> {
    /// 字符串池
    pub string_pool: &'a StringPool,
    /// 函数池
    pub function_pool: &'a FunctionPool,
    /// Shape 表（用于结构体实例解码）
    pub shape_table: &'a ShapeTable,
}

impl<'a> DecodeContext<'a> {
    /// 创建新的解码上下文
    pub fn new(string_pool: &'a StringPool, function_pool: &'a FunctionPool, shape_table: &'a ShapeTable) -> Self {
        Self {
            string_pool,
            function_pool,
            shape_table,
        }
    }
}

impl<'a> std::fmt::Debug for DecodeContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodeContext")
            .field("string_pool_len", &self.string_pool.len())
            .field("function_pool_len", &self.function_pool.len())
            .field("shape_table_len", &self.shape_table.len())
            .finish()
    }
}

/// Chunk 编码错误
#[derive(Debug, Clone)]
pub enum ChunkEncodeError {
    /// 常量类型不支持序列化
    UnsupportedConstantType(&'static str),
    /// 数据太大
    DataTooLarge(&'static str),
}

impl std::fmt::Display for ChunkEncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkEncodeError::UnsupportedConstantType(t) => {
                write!(f, "Unsupported constant type for serialization: {}", t)
            }
            ChunkEncodeError::DataTooLarge(what) => write!(f, "{} too large", what),
        }
    }
}

impl std::error::Error for ChunkEncodeError {}

/// Chunk 解码错误
#[derive(Debug, Clone)]
pub enum ChunkDecodeError {
    /// 数据太短
    TooShort,
    /// 无效的常量类型标记
    InvalidConstantType(u8),
    /// 无效的运算符名称
    InvalidOperatorName,
    /// 数据损坏
    CorruptedData(&'static str),
}

impl std::fmt::Display for ChunkDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkDecodeError::TooShort => write!(f, "Chunk data too short"),
            ChunkDecodeError::InvalidConstantType(t) => {
                write!(f, "Invalid constant type: {}", t)
            }
            ChunkDecodeError::InvalidOperatorName => write!(f, "Invalid operator name"),
            ChunkDecodeError::CorruptedData(what) => write!(f, "Corrupted data: {}", what),
        }
    }
}

impl std::error::Error for ChunkDecodeError {}

/// 常量类型标记
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConstantType {
    Null = 0x00,
    True = 0x01,
    False = 0x02,
    SMI = 0x03,           // 小整数
    Float = 0x04,         // 浮点数
    String = 0x05,        // 字符串（String Pool 索引）
    Function = 0x06,      // 函数字面量（Function Pool 索引）
    Struct = 0x07,        // 结构体实例（Shape ID + 字段值）
    List = 0x08,          // 列表（元素数组）
}

impl ConstantType {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(Self::Null),
            0x01 => Some(Self::True),
            0x02 => Some(Self::False),
            0x03 => Some(Self::SMI),
            0x04 => Some(Self::Float),
            0x05 => Some(Self::String),
            0x06 => Some(Self::Function),
            0x07 => Some(Self::Struct),
            0x08 => Some(Self::List),
            _ => None,
        }
    }
}

/// 将 Chunk 编码为字节数组（使用上下文）
///
/// 格式：
/// ```text
/// - code_len: u32
/// - code: [u8; code_len]
/// - lines_len: u32
/// - lines: [u32; lines_len] (每个指令对应的行号)
/// - constants_len: u32
/// - constants: [Constant; constants_len]
/// - method_table_len: u32
/// - method_table: [MethodTableEntry; method_table_len]
/// - operator_table_len: u32
/// - operator_table: [OperatorTableEntry; operator_table_len]
/// - inline_cache_slots_len: u32
/// - inline_cache_slots: [InlineCacheSlot; inline_cache_slots_len]
/// - inline_caches_len: u32
/// - inline_caches: [InlineCacheEntry; inline_caches_len]
/// ```
pub fn encode_chunk_with_context(
    chunk: &Chunk,
    ctx: &mut EncodeContext,
) -> Result<Vec<u8>, ChunkEncodeError> {
    let mut result = Vec::new();

    // 编码 code
    encode_bytes(&mut result, &chunk.code);

    // 编码 lines（转换为 u32 数组以节省空间）
    encode_u32_array(&mut result, &chunk.lines.iter().map(|&l| l as u32).collect::<Vec<_>>());

    // 编码 constants（使用上下文）
    encode_constants_with_context(&mut result, &chunk.constants, ctx)?;

    // 编码 method_table
    encode_method_table(&mut result, &chunk.method_table);

    // 编码 operator_table
    encode_operator_table(&mut result, &chunk.operator_table);

    // 编码 inline_cache_slots
    encode_inline_cache_slots(&mut result, &chunk.inline_cache_slots);

    // 编码 inline_caches
    encode_inline_caches(&mut result, &chunk.inline_caches);

    Ok(result)
}

/// 将 Chunk 编码为字节数组（简化版本，仅支持基础类型）
/// 
/// **注意**: 如果 Chunk 包含字符串、函数等堆对象，此函数会返回错误。
/// 请使用 `encode_chunk_with_context` 来支持完整的堆对象序列化。
pub fn encode_chunk(chunk: &Chunk) -> Result<Vec<u8>, ChunkEncodeError> {
    let mut string_pool = StringPool::new();
    let mut function_pool = FunctionPool::new();
    let mut shape_table = ShapeTable::new();
    let mut ctx = EncodeContext::new(&mut string_pool, &mut function_pool, &mut shape_table);
    
    encode_chunk_with_context(chunk, &mut ctx)
}

/// 从字节数组解码 Chunk（使用上下文）
pub fn decode_chunk_with_context(
    bytes: &[u8],
    offset: &mut usize,
    ctx: &DecodeContext,
) -> Result<Chunk, ChunkDecodeError> {
    // 解码 code
    let code = decode_bytes(bytes, offset)?;

    // 解码 lines
    let lines_u32 = decode_u32_array(bytes, offset)?;
    let lines = lines_u32.iter().map(|&l| l as usize).collect();

    // 解码 constants（使用上下文）
    let constants = decode_constants_with_context(bytes, offset, ctx)?;

    // 解码 method_table
    let method_table = decode_method_table(bytes, offset)?;

    // 解码 operator_table
    let operator_table = decode_operator_table(bytes, offset)?;

    // 解码 inline_cache_slots
    let inline_cache_slots = decode_inline_cache_slots(bytes, offset)?;

    // 解码 inline_caches
    let inline_caches = decode_inline_caches(bytes, offset)?;

    Ok(Chunk {
        code,
        constants,
        lines,
        method_table,
        operator_table,
        inline_cache_slots,
        inline_caches,
    })
}

/// 从字节数组解码 Chunk（简化版本）
/// 
/// **注意**: 如果 Chunk 包含字符串、函数等堆对象引用，此函数会返回错误。
/// 请使用 `decode_chunk_with_context` 来支持完整的堆对象反序列化。
pub fn decode_chunk(bytes: &[u8]) -> Result<Chunk, ChunkDecodeError> {
    let mut offset = 0;

    // 解码 code
    let code = decode_bytes(bytes, &mut offset)?;

    // 解码 lines
    let lines_u32 = decode_u32_array(bytes, &mut offset)?;
    let lines = lines_u32.iter().map(|&l| l as usize).collect();

    // 解码 constants（基础类型）
    let constants = decode_constants(bytes, &mut offset)?;

    // 解码 method_table
    let method_table = decode_method_table(bytes, &mut offset)?;

    // 解码 operator_table
    let operator_table = decode_operator_table(bytes, &mut offset)?;

    // 解码 inline_cache_slots
    let inline_cache_slots = decode_inline_cache_slots(bytes, &mut offset)?;

    // 解码 inline_caches
    let inline_caches = decode_inline_caches(bytes, &mut offset)?;

    Ok(Chunk {
        code,
        constants,
        lines,
        method_table,
        operator_table,
        inline_cache_slots,
        inline_caches,
    })
}

// ==================== 辅助编码函数 ====================

fn encode_bytes(buf: &mut Vec<u8>, data: &[u8]) {
    buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
    buf.extend_from_slice(data);
}

fn encode_u32_array(buf: &mut Vec<u8>, data: &[u32]) {
    buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
    for &val in data {
        buf.extend_from_slice(&val.to_le_bytes());
    }
}

fn encode_constants(
    buf: &mut Vec<u8>,
    constants: &[Value],
) -> Result<(), ChunkEncodeError> {
    buf.extend_from_slice(&(constants.len() as u32).to_le_bytes());

    for value in constants {
        encode_constant(buf, *value)?;
    }

    Ok(())
}

fn encode_constant(buf: &mut Vec<u8>, value: Value) -> Result<(), ChunkEncodeError> {
    if value.is_null() {
        buf.push(ConstantType::Null as u8);
    } else if value.is_true() {
        buf.push(ConstantType::True as u8);
    } else if value.is_false() {
        buf.push(ConstantType::False as u8);
    } else if let Some(n) = value.as_int() {
        buf.push(ConstantType::SMI as u8);
        buf.extend_from_slice(&n.to_le_bytes());
    } else if value.is_float() {
        buf.push(ConstantType::Float as u8);
        buf.extend_from_slice(&value.as_float().to_le_bytes());
    } else {
        // 堆类型需要使用 encode_constant_with_context
        return Err(ChunkEncodeError::UnsupportedConstantType(
            "heap object (string/function/struct) - use encode_chunk_with_context",
        ));
    }

    Ok(())
}

/// 使用上下文编码常量池
fn encode_constants_with_context(
    buf: &mut Vec<u8>,
    constants: &[Value],
    ctx: &mut EncodeContext,
) -> Result<(), ChunkEncodeError> {
    buf.extend_from_slice(&(constants.len() as u32).to_le_bytes());

    for value in constants {
        encode_constant_with_context(buf, *value, ctx)?;
    }

    Ok(())
}

/// 使用上下文编码单个常量
fn encode_constant_with_context(
    buf: &mut Vec<u8>,
    value: Value,
    ctx: &mut EncodeContext,
) -> Result<(), ChunkEncodeError> {

    if value.is_null() {
        buf.push(ConstantType::Null as u8);
    } else if value.is_true() {
        buf.push(ConstantType::True as u8);
    } else if value.is_false() {
        buf.push(ConstantType::False as u8);
    } else if let Some(n) = value.as_int() {
        buf.push(ConstantType::SMI as u8);
        buf.extend_from_slice(&n.to_le_bytes());
    } else if value.is_float() {
        buf.push(ConstantType::Float as u8);
        buf.extend_from_slice(&value.as_float().to_le_bytes());
    } else if let Some(s) = value.as_string() {
        // 字符串：注册到 String Pool，存储索引
        let s_ref = unsafe { &*s };
        let idx = ctx.string_pool.add(&s_ref.chars);
        buf.push(ConstantType::String as u8);
        buf.extend_from_slice(&idx.to_le_bytes());
    } else if let Some(func_ptr) = value.as_function() {
        // 函数：递归编码函数，注册到 Function Pool
        let func_idx = encode_function_to_pool(func_ptr, ctx)?;
        buf.push(ConstantType::Function as u8);
        buf.extend_from_slice(&func_idx.to_le_bytes());
    } else if let Some(list_ptr) = value.as_list() {
        // 列表：编码元素数组
        buf.push(ConstantType::List as u8);
        let list = unsafe { &*list_ptr };
        buf.extend_from_slice(&(list.elements.len() as u32).to_le_bytes());
        for elem in &list.elements {
            encode_constant_with_context(buf, *elem, ctx)?;
        }
    } else if let Some(struct_ptr) = value.as_struct() {
        // 结构体实例：编码 shape_id + 字段值
        let s = unsafe { &*struct_ptr };
        let shape_id = s.shape_id();
        
        // 检查 Shape 是否已注册，如果没有则注册
        if ctx.shape_table.get_by_id(shape_id).is_none() {
            if !s.shape.is_null() {
                let shape = unsafe { &*s.shape };
                
                // 编码结构体名称
                let name_idx = ctx.string_pool.add(&shape.name);
                
                // 编码字段名
                let field_name_indices: Vec<u32> = shape.field_names.iter()
                    .map(|name| ctx.string_pool.add(name))
                    .collect();
                
                // 编码字段类型（如果有）
                let field_type_indices: Vec<u32> = shape.field_types.iter()
                    .map(|ty| ctx.string_pool.add(ty))
                    .collect();
                
                let entry = ShapeEntry {
                    shape_id,
                    name_idx,
                    field_count: shape.field_names.len() as u16,
                    field_name_indices,
                    field_type_indices,
                };
                ctx.shape_table.add(entry);
            }
        }
        
        buf.push(ConstantType::Struct as u8);
        buf.extend_from_slice(&shape_id.to_le_bytes());
        buf.extend_from_slice(&(s.fields.len() as u32).to_le_bytes());
        for field in &s.fields {
            encode_constant_with_context(buf, *field, ctx)?;
        }
    } else {
        return Err(ChunkEncodeError::UnsupportedConstantType("unknown heap object"));
    }

    Ok(())
}

/// 将函数注册到 Function Pool
fn encode_function_to_pool(
    func_ptr: *mut ObjFunction,
    ctx: &mut EncodeContext,
) -> Result<u32, ChunkEncodeError> {
    // 安全地获取函数数据
    let func = unsafe { &*func_ptr };
    
    // 函数名
    let name_idx = if let Some(name) = func.name.as_ref() {
        ctx.string_pool.add(name)
    } else {
        0 // 匿名函数
    };

    // 递归编码函数的 chunk
    let chunk_data = encode_chunk_with_context(&func.chunk, ctx)?;

    // 创建 FunctionEntry
    let entry = FunctionEntry {
        name_idx,
        arity: func.arity,
        chunk_data,
    };

    let idx = ctx.function_pool.add(entry);
    Ok(idx)
}

fn encode_method_table(buf: &mut Vec<u8>, table: &[MethodTableEntry]) {
    buf.extend_from_slice(&(table.len() as u32).to_le_bytes());

    for entry in table {
        buf.extend_from_slice(&entry.shape_id.to_le_bytes());
        buf.push(entry.method_idx);
        buf.push(entry.const_idx);
    }
}

fn encode_operator_table(buf: &mut Vec<u8>, table: &[OperatorTableEntry]) {
    buf.extend_from_slice(&(table.len() as u32).to_le_bytes());

    for entry in table {
        buf.extend_from_slice(&entry.shape_id.to_le_bytes());
        // operator_name: 长度 + 字符串
        let name_bytes = entry.operator_name.as_bytes();
        buf.push(name_bytes.len() as u8);
        buf.extend_from_slice(name_bytes);
        buf.push(entry.const_idx);
    }
}

fn encode_inline_cache_slots(buf: &mut Vec<u8>, slots: &[InlineCacheSlot]) {
    buf.extend_from_slice(&(slots.len() as u32).to_le_bytes());

    for slot in slots {
        buf.extend_from_slice(&(slot.pc as u32).to_le_bytes());
        buf.push(slot.cache_idx);
    }
}

fn encode_inline_caches(buf: &mut Vec<u8>, caches: &[crate::vm::core::operators::InlineCacheEntry]) {
    buf.extend_from_slice(&(caches.len() as u32).to_le_bytes());

    for cache in caches {
        // InlineCacheEntry: left_shape (u16) + right_shape (u16) + hit_count (u64) + miss_count (u64)
        // 注意：closure 是指针，不能序列化，需要在加载时重新绑定
        buf.extend_from_slice(&cache.left_shape.to_le_bytes());
        buf.extend_from_slice(&cache.right_shape.to_le_bytes());
        buf.extend_from_slice(&cache.hit_count.to_le_bytes());
        buf.extend_from_slice(&cache.miss_count.to_le_bytes());
    }
}

// ==================== 辅助解码函数 ====================

fn decode_bytes(bytes: &[u8], offset: &mut usize) -> Result<Vec<u8>, ChunkDecodeError> {
    if bytes.len() < *offset + 4 {
        return Err(ChunkDecodeError::TooShort);
    }

    let len = u32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]) as usize;
    *offset += 4;

    if bytes.len() < *offset + len {
        return Err(ChunkDecodeError::TooShort);
    }

    let data = bytes[*offset..*offset + len].to_vec();
    *offset += len;

    Ok(data)
}

fn decode_u32_array(bytes: &[u8], offset: &mut usize) -> Result<Vec<u32>, ChunkDecodeError> {
    if bytes.len() < *offset + 4 {
        return Err(ChunkDecodeError::TooShort);
    }

    let len = u32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]) as usize;
    *offset += 4;

    if bytes.len() < *offset + len * 4 {
        return Err(ChunkDecodeError::TooShort);
    }

    let mut result = Vec::with_capacity(len);
    for _ in 0..len {
        let val = u32::from_le_bytes([
            bytes[*offset],
            bytes[*offset + 1],
            bytes[*offset + 2],
            bytes[*offset + 3],
        ]);
        result.push(val);
        *offset += 4;
    }

    Ok(result)
}

fn decode_constants(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<Vec<Value>, ChunkDecodeError> {
    if bytes.len() < *offset + 4 {
        return Err(ChunkDecodeError::TooShort);
    }

    let len = u32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]) as usize;
    *offset += 4;

    let mut result = Vec::with_capacity(len);
    for _ in 0..len {
        let value = decode_constant(bytes, offset)?;
        result.push(value);
    }

    Ok(result)
}

fn decode_constant(bytes: &[u8], offset: &mut usize) -> Result<Value, ChunkDecodeError> {
    if bytes.len() < *offset + 1 {
        return Err(ChunkDecodeError::TooShort);
    }

    let type_byte = bytes[*offset];
    *offset += 1;

    let const_type = ConstantType::from_u8(type_byte)
        .ok_or(ChunkDecodeError::InvalidConstantType(type_byte))?;

    match const_type {
        ConstantType::Null => Ok(Value::NULL),
        ConstantType::True => Ok(Value::TRUE),
        ConstantType::False => Ok(Value::FALSE),
        ConstantType::SMI => {
            if bytes.len() < *offset + 4 {
                return Err(ChunkDecodeError::TooShort);
            }
            let n = i32::from_le_bytes([
                bytes[*offset],
                bytes[*offset + 1],
                bytes[*offset + 2],
                bytes[*offset + 3],
            ]);
            *offset += 4;
            Ok(Value::int(n))
        }
        ConstantType::Float => {
            if bytes.len() < *offset + 8 {
                return Err(ChunkDecodeError::TooShort);
            }
            let f = f64::from_le_bytes([
                bytes[*offset],
                bytes[*offset + 1],
                bytes[*offset + 2],
                bytes[*offset + 3],
                bytes[*offset + 4],
                bytes[*offset + 5],
                bytes[*offset + 6],
                bytes[*offset + 7],
            ]);
            *offset += 8;
            Ok(Value::float(f))
        }
        ConstantType::String => {
            // 需要 DecodeContext 来解析 String Pool 索引
            Err(ChunkDecodeError::CorruptedData(
                "String constant requires DecodeContext - use decode_chunk_with_context",
            ))
        }
        ConstantType::Function => {
            // 需要 DecodeContext 来解析 Function Pool 索引
            Err(ChunkDecodeError::CorruptedData(
                "Function constant requires DecodeContext - use decode_chunk_with_context",
            ))
        }
        ConstantType::Struct | ConstantType::List => {
            Err(ChunkDecodeError::CorruptedData(
                "Struct/List constants require DecodeContext - use decode_chunk_with_context",
            ))
        }
    }
}

/// 使用上下文解码常量池
fn decode_constants_with_context(
    bytes: &[u8],
    offset: &mut usize,
    ctx: &DecodeContext,
) -> Result<Vec<Value>, ChunkDecodeError> {
    if bytes.len() < *offset + 4 {
        return Err(ChunkDecodeError::TooShort);
    }

    let len = u32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]) as usize;
    *offset += 4;

    let mut result = Vec::with_capacity(len);
    for _ in 0..len {
        let value = decode_constant_with_context(bytes, offset, ctx)?;
        result.push(value);
    }

    Ok(result)
}

/// 使用上下文解码单个常量
fn decode_constant_with_context(
    bytes: &[u8],
    offset: &mut usize,
    ctx: &DecodeContext,
) -> Result<Value, ChunkDecodeError> {
    if bytes.len() < *offset + 1 {
        return Err(ChunkDecodeError::TooShort);
    }

    let type_byte = bytes[*offset];
    *offset += 1;

    let const_type = ConstantType::from_u8(type_byte)
        .ok_or(ChunkDecodeError::InvalidConstantType(type_byte))?;

    match const_type {
        ConstantType::Null => Ok(Value::NULL),
        ConstantType::True => Ok(Value::TRUE),
        ConstantType::False => Ok(Value::FALSE),
        ConstantType::SMI => {
            if bytes.len() < *offset + 4 {
                return Err(ChunkDecodeError::TooShort);
            }
            let n = i32::from_le_bytes([
                bytes[*offset],
                bytes[*offset + 1],
                bytes[*offset + 2],
                bytes[*offset + 3],
            ]);
            *offset += 4;
            Ok(Value::int(n))
        }
        ConstantType::Float => {
            if bytes.len() < *offset + 8 {
                return Err(ChunkDecodeError::TooShort);
            }
            let f = f64::from_le_bytes([
                bytes[*offset],
                bytes[*offset + 1],
                bytes[*offset + 2],
                bytes[*offset + 3],
                bytes[*offset + 4],
                bytes[*offset + 5],
                bytes[*offset + 6],
                bytes[*offset + 7],
            ]);
            *offset += 8;
            Ok(Value::float(f))
        }
        ConstantType::String => {
            // 从 String Pool 解析字符串
            if bytes.len() < *offset + 4 {
                return Err(ChunkDecodeError::TooShort);
            }
            let idx = u32::from_le_bytes([
                bytes[*offset],
                bytes[*offset + 1],
                bytes[*offset + 2],
                bytes[*offset + 3],
            ]);
            *offset += 4;

            let s = ctx.string_pool.get(idx)
                .ok_or(ChunkDecodeError::CorruptedData("Invalid string pool index"))?;
            
            // 创建 ObjString
            let obj_string = crate::vm::core::object::ObjString::new(s.to_string());
            let string_ptr = Box::into_raw(Box::new(obj_string));
            Ok(Value::string(string_ptr))
        }
        ConstantType::Function => {
            // 从 Function Pool 解析函数
            if bytes.len() < *offset + 4 {
                return Err(ChunkDecodeError::TooShort);
            }
            let idx = u32::from_le_bytes([
                bytes[*offset],
                bytes[*offset + 1],
                bytes[*offset + 2],
                bytes[*offset + 3],
            ]);
            *offset += 4;

            decode_function_from_pool(idx, ctx)
        }
        ConstantType::List => {
            // 解码列表元素
            if bytes.len() < *offset + 4 {
                return Err(ChunkDecodeError::TooShort);
            }
            let len = u32::from_le_bytes([
                bytes[*offset],
                bytes[*offset + 1],
                bytes[*offset + 2],
                bytes[*offset + 3],
            ]) as usize;
            *offset += 4;

            let mut elements = Vec::with_capacity(len);
            for _ in 0..len {
                let elem = decode_constant_with_context(bytes, offset, ctx)?;
                elements.push(elem);
            }
            
            // 创建 ObjList 并转换为指针
            let list = crate::vm::core::object::ObjList::from_vec(elements);
            let list_ptr = Box::into_raw(Box::new(list));
            Ok(Value::list(list_ptr))
        }
        ConstantType::Struct => {
            // 解码结构体实例：shape_id + fields
            if bytes.len() < *offset + 6 {
                return Err(ChunkDecodeError::TooShort);
            }
            
            // 读取 shape_id
            let shape_id = u16::from_le_bytes([bytes[*offset], bytes[*offset + 1]]);
            *offset += 2;
            
            // 读取字段数量
            let len = u32::from_le_bytes([
                bytes[*offset],
                bytes[*offset + 1],
                bytes[*offset + 2],
                bytes[*offset + 3],
            ]) as usize;
            *offset += 4;
            
            // 解码字段值
            let mut fields = Vec::with_capacity(len);
            for _ in 0..len {
                let field = decode_constant_with_context(bytes, offset, ctx)?;
                fields.push(field);
            }
            
            // 从 ShapeTable 查找 ShapeEntry 并重建 ObjShape
            let shape = if let Some(entry) = ctx.shape_table.get_by_id(shape_id) {
                // 重建字段名数组
                let field_names: Vec<String> = entry.field_name_indices.iter()
                    .map(|&idx| ctx.string_pool.get(idx).unwrap_or("").to_string())
                    .collect();
                
                // 重建字段类型数组（如果有）
                let field_types: Vec<String> = entry.field_type_indices.iter()
                    .map(|&idx| ctx.string_pool.get(idx).unwrap_or("").to_string())
                    .collect();
                
                // 获取结构体名称
                let name = ctx.string_pool.get(entry.name_idx).unwrap_or("Struct").to_string();
                
                // 创建 ObjShape
                let shape = if field_types.is_empty() {
                    ObjShape::new(shape_id, name, field_names)
                } else {
                    ObjShape::new_with_types(shape_id, name, field_names, field_types)
                };
                
                let shape_ptr = Box::into_raw(Box::new(shape));
                shape_ptr
            } else {
                return Err(ChunkDecodeError::CorruptedData("Shape not found in shape table"));
            };
            
            // 创建 ObjStruct
            let s = ObjStruct::new(shape, fields);
            let struct_ptr = Box::into_raw(Box::new(s));
            Ok(Value::struct_instance(struct_ptr))
        }
    }
}

/// 从 Function Pool 解码函数
fn decode_function_from_pool(
    idx: u32,
    ctx: &DecodeContext,
) -> Result<Value, ChunkDecodeError> {
    use crate::vm::core::object::ObjFunction;

    let entry = ctx.function_pool.get(idx)
        .ok_or(ChunkDecodeError::CorruptedData("Invalid function pool index"))?;

    // 获取函数名
    let name = if entry.name_idx == 0 {
        None // 匿名函数
    } else {
        let name_str = ctx.string_pool.get(entry.name_idx)
            .ok_or(ChunkDecodeError::CorruptedData("Invalid function name index"))?;
        Some(name_str.to_string())
    };

    // 递归解码函数的 chunk
    let mut chunk_offset = 0;
    let chunk = decode_chunk_with_context(&entry.chunk_data, &mut chunk_offset, ctx)?;

    // 创建 ObjFunction
    let func = ObjFunction::new(chunk, entry.arity, name);
    let func_ptr = Box::into_raw(Box::new(func));

    Ok(Value::function(func_ptr))
}

fn decode_method_table(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<Vec<MethodTableEntry>, ChunkDecodeError> {
    if bytes.len() < *offset + 4 {
        return Err(ChunkDecodeError::TooShort);
    }

    let len = u32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]) as usize;
    *offset += 4;

    let mut result = Vec::with_capacity(len);
    for _ in 0..len {
        if bytes.len() < *offset + 4 {
            return Err(ChunkDecodeError::TooShort);
        }

        let shape_id = u16::from_le_bytes([bytes[*offset], bytes[*offset + 1]]);
        let method_idx = bytes[*offset + 2];
        let const_idx = bytes[*offset + 3];
        *offset += 4;

        result.push(MethodTableEntry {
            shape_id,
            method_idx,
            const_idx,
        });
    }

    Ok(result)
}

fn decode_operator_table(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<Vec<OperatorTableEntry>, ChunkDecodeError> {
    if bytes.len() < *offset + 4 {
        return Err(ChunkDecodeError::TooShort);
    }

    let len = u32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]) as usize;
    *offset += 4;

    let mut result = Vec::with_capacity(len);
    for _ in 0..len {
        if bytes.len() < *offset + 3 {
            return Err(ChunkDecodeError::TooShort);
        }

        let shape_id = u16::from_le_bytes([bytes[*offset], bytes[*offset + 1]]);
        *offset += 2;

        let name_len = bytes[*offset] as usize;
        *offset += 1;

        if bytes.len() < *offset + name_len + 1 {
            return Err(ChunkDecodeError::TooShort);
        }

        let name = String::from_utf8(bytes[*offset..*offset + name_len].to_vec())
            .map_err(|_| ChunkDecodeError::InvalidOperatorName)?;
        *offset += name_len;

        let const_idx = bytes[*offset];
        *offset += 1;

        result.push(OperatorTableEntry {
            shape_id,
            operator_name: name,
            const_idx,
        });
    }

    Ok(result)
}

fn decode_inline_cache_slots(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<Vec<InlineCacheSlot>, ChunkDecodeError> {
    if bytes.len() < *offset + 4 {
        return Err(ChunkDecodeError::TooShort);
    }

    let len = u32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]) as usize;
    *offset += 4;

    let mut result = Vec::with_capacity(len);
    for _ in 0..len {
        if bytes.len() < *offset + 5 {
            return Err(ChunkDecodeError::TooShort);
        }

        let pc = u32::from_le_bytes([
            bytes[*offset],
            bytes[*offset + 1],
            bytes[*offset + 2],
            bytes[*offset + 3],
        ]) as usize;
        let cache_idx = bytes[*offset + 4];
        *offset += 5;

        result.push(InlineCacheSlot { pc, cache_idx });
    }

    Ok(result)
}

fn decode_inline_caches(
    bytes: &[u8],
    offset: &mut usize,
) -> Result<Vec<crate::vm::core::operators::InlineCacheEntry>, ChunkDecodeError> {
    if bytes.len() < *offset + 4 {
        return Err(ChunkDecodeError::TooShort);
    }

    let len = u32::from_le_bytes([
        bytes[*offset],
        bytes[*offset + 1],
        bytes[*offset + 2],
        bytes[*offset + 3],
    ]) as usize;
    *offset += 4;

    let mut result = Vec::with_capacity(len);
    for _ in 0..len {
        if bytes.len() < *offset + 20 {
            return Err(ChunkDecodeError::TooShort);
        }

        let left_shape = u16::from_le_bytes([bytes[*offset], bytes[*offset + 1]]);
        let right_shape = u16::from_le_bytes([bytes[*offset + 2], bytes[*offset + 3]]);
        let hit_count = u64::from_le_bytes([
            bytes[*offset + 4],
            bytes[*offset + 5],
            bytes[*offset + 6],
            bytes[*offset + 7],
            bytes[*offset + 8],
            bytes[*offset + 9],
            bytes[*offset + 10],
            bytes[*offset + 11],
        ]);
        let miss_count = u64::from_le_bytes([
            bytes[*offset + 12],
            bytes[*offset + 13],
            bytes[*offset + 14],
            bytes[*offset + 15],
            bytes[*offset + 16],
            bytes[*offset + 17],
            bytes[*offset + 18],
            bytes[*offset + 19],
        ]);
        *offset += 20;

        result.push(crate::vm::core::operators::InlineCacheEntry {
            left_shape,
            right_shape,
            closure: std::ptr::null_mut(), // 需要在运行时重新绑定
            hit_count,
            miss_count,
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_chunk() -> Chunk {
        let mut chunk = Chunk::new();

        // 添加一些字节码
        chunk.write_op(crate::vm::core::bytecode::OpCode::LoadConst0, 1);
        chunk.write_op(crate::vm::core::bytecode::OpCode::LoadConst1, 1);
        chunk.write_op(crate::vm::core::bytecode::OpCode::Add, 1);

        // 添加常量
        chunk.add_constant(Value::int(42));
        chunk.add_constant(Value::float(3.14));
        chunk.add_constant(Value::NULL);
        chunk.add_constant(Value::TRUE);
        chunk.add_constant(Value::FALSE);

        chunk
    }

    #[test]
    fn test_chunk_roundtrip() {
        let original = create_test_chunk();
        let encoded = encode_chunk(&original).unwrap();
        let decoded = decode_chunk(&encoded).unwrap();

        assert_eq!(original.code, decoded.code);
        assert_eq!(original.lines, decoded.lines);
        assert_eq!(original.constants.len(), decoded.constants.len());

        // 验证常量
        for (orig, dec) in original.constants.iter().zip(decoded.constants.iter()) {
            assert_eq!(orig.0, dec.0); // Value 使用 NaN-boxing，直接比较 u64
        }
    }

    #[test]
    fn test_encode_constants() {
        let mut buf = Vec::new();

        encode_constant(&mut buf, Value::NULL).unwrap();
        encode_constant(&mut buf, Value::TRUE).unwrap();
        encode_constant(&mut buf, Value::FALSE).unwrap();
        encode_constant(&mut buf, Value::int(42)).unwrap();
        encode_constant(&mut buf, Value::float(3.14)).unwrap();

        assert_eq!(buf.len(), 1 + 1 + 1 + 5 + 9); // 1+1+1+1+4+1+8
    }

    #[test]
    fn test_method_table_roundtrip() {
        let entries = vec![
            MethodTableEntry {
                shape_id: 1,
                method_idx: 2,
                const_idx: 3,
            },
            MethodTableEntry {
                shape_id: 4,
                method_idx: 5,
                const_idx: 6,
            },
        ];

        let mut buf = Vec::new();
        encode_method_table(&mut buf, &entries);

        let mut offset = 0;
        let decoded = decode_method_table(&buf, &mut offset).unwrap();

        assert_eq!(entries.len(), decoded.len());
        assert_eq!(entries[0].shape_id, decoded[0].shape_id);
        assert_eq!(entries[0].method_idx, decoded[0].method_idx);
        assert_eq!(entries[0].const_idx, decoded[0].const_idx);
    }

    #[test]
    fn test_operator_table_roundtrip() {
        let entries = vec![
            OperatorTableEntry {
                shape_id: 1,
                operator_name: "add".to_string(),
                const_idx: 2,
            },
            OperatorTableEntry {
                shape_id: 3,
                operator_name: "sub".to_string(),
                const_idx: 4,
            },
        ];

        let mut buf = Vec::new();
        encode_operator_table(&mut buf, &entries);

        let mut offset = 0;
        let decoded = decode_operator_table(&buf, &mut offset).unwrap();

        assert_eq!(entries.len(), decoded.len());
        assert_eq!(entries[0].operator_name, decoded[0].operator_name);
        assert_eq!(entries[1].operator_name, decoded[1].operator_name);
    }

    #[test]
    fn test_empty_chunk() {
        let chunk = Chunk::new();
        let encoded = encode_chunk(&chunk).unwrap();
        let decoded = decode_chunk(&encoded).unwrap();

        assert!(decoded.code.is_empty());
        assert!(decoded.constants.is_empty());
        assert!(decoded.lines.is_empty());
    }
}
