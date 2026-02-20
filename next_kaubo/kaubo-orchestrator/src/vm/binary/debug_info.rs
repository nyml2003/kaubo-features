//! 调试信息 (Debug Info)
//!
//! 包含行号表和局部变量名信息，用于错误堆栈和调试器。

use super::section::{SectionData, SectionError, SectionKind};

/// 调试信息
#[derive(Debug, Clone)]
pub struct DebugInfo {
    /// 行号表：每个指令对应的源码行号
    pub line_table: LineTable,
    /// 局部变量名表
    pub local_names: LocalNameTable,
    /// 源文件路径（在字符串池中的索引）
    pub source_path_idx: u32,
}

impl DebugInfo {
    /// 创建空的调试信息
    pub fn new() -> Self {
        Self {
            line_table: LineTable::new(),
            local_names: LocalNameTable::new(),
            source_path_idx: u32::MAX,
        }
    }

    /// 设置源文件路径
    pub fn set_source_path(&mut self, idx: u32) {
        self.source_path_idx = idx;
    }
}

impl Default for DebugInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl SectionData for DebugInfo {
    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // 源文件路径索引
        result.extend_from_slice(&self.source_path_idx.to_le_bytes());

        // 行号表
        let line_table_data = self.line_table.serialize();
        result.extend_from_slice(&(line_table_data.len() as u32).to_le_bytes());
        result.extend_from_slice(&line_table_data);

        // 局部变量名表
        let local_names_data = self.local_names.serialize();
        result.extend_from_slice(&(local_names_data.len() as u32).to_le_bytes());
        result.extend_from_slice(&local_names_data);

        result
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SectionError> {
        let mut offset = 0;

        if bytes.len() < 4 {
            return Err(SectionError::TooShort);
        }

        // 源文件路径索引
        let source_path_idx = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;

        // 行号表
        if bytes.len() < offset + 4 {
            return Err(SectionError::TooShort);
        }
        let line_table_len = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        if bytes.len() < offset + line_table_len {
            return Err(SectionError::TooShort);
        }
        let line_table = LineTable::deserialize(&bytes[offset..offset + line_table_len])?;
        offset += line_table_len;

        // 局部变量名表
        if bytes.len() < offset + 4 {
            return Err(SectionError::TooShort);
        }
        let local_names_len = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        if bytes.len() < offset + local_names_len {
            return Err(SectionError::TooShort);
        }
        let local_names = LocalNameTable::deserialize(&bytes[offset..offset + local_names_len])?;

        Ok(Self {
            line_table,
            local_names,
            source_path_idx,
        })
    }

    fn section_kind() -> SectionKind {
        SectionKind::DebugInfo
    }
}

/// 行号表
///
/// 将指令偏移量映射到源码行号。使用差分编码压缩存储。
#[derive(Debug, Clone)]
pub struct LineTable {
    /// 条目：指令偏移量 -> 行号
    pub entries: Vec<LineEntry>,
}

/// 行号条目
#[derive(Debug, Clone, Copy)]
pub struct LineEntry {
    /// 指令偏移量（字节码位置）
    pub pc: u32,
    /// 源码行号
    pub line: u32,
}

impl LineTable {
    /// 创建空的行号表
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// 添加条目
    pub fn add(&mut self, pc: u32, line: u32) {
        self.entries.push(LineEntry { pc, line });
    }

    /// 查找指定 PC 对应的行号
    pub fn lookup(&self, pc: u32) -> Option<u32> {
        // 二分查找
        let mut low = 0;
        let mut high = self.entries.len();

        while low < high {
            let mid = (low + high) / 2;
            if self.entries[mid].pc <= pc {
                low = mid + 1;
            } else {
                high = mid;
            }
        }

        if low > 0 {
            Some(self.entries[low - 1].line)
        } else {
            None
        }
    }

    /// 从 Chunk 的 lines 数组构建行号表
    pub fn from_chunk_lines(lines: &[usize]) -> Self {
        let mut table = Self::new();
        let mut current_pc = 0u32;

        for (i, &line) in lines.iter().enumerate() {
            // 只在行号变化时添加条目（差分编码）
            if i == 0 || line != lines[i - 1] {
                table.add(current_pc, line as u32);
            }
            current_pc += 1;
        }

        table
    }

    /// 序列化（差分编码）
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // 条目数量
        result.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());

        // 差分编码的条目
        let mut prev_pc = 0u32;
        let mut prev_line = 0u32;

        for entry in &self.entries {
            let pc_delta = entry.pc.wrapping_sub(prev_pc);
            let line_delta = entry.line.wrapping_sub(prev_line) as i32;

            // 使用变长编码存储差值
            encode_varint(&mut result, pc_delta as u64);
            encode_signed_varint(&mut result, line_delta);

            prev_pc = entry.pc;
            prev_line = entry.line;
        }

        result
    }

    /// 反序列化（差分编码）
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SectionError> {
        if bytes.len() < 4 {
            return Err(SectionError::TooShort);
        }

        let mut offset = 0;
        let count = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        let mut entries = Vec::with_capacity(count);
        let mut prev_pc = 0u32;
        let mut prev_line = 0u32;

        for _ in 0..count {
            let (pc_delta, bytes_read) = decode_varint(&bytes[offset..])
                .map_err(|_| SectionError::InvalidSize)?;
            offset += bytes_read;

            let (line_delta, bytes_read) = decode_signed_varint(&bytes[offset..])
                .map_err(|_| SectionError::InvalidSize)?;
            offset += bytes_read;

            prev_pc = prev_pc.wrapping_add(pc_delta as u32);
            prev_line = prev_line.wrapping_add(line_delta as u32);

            entries.push(LineEntry {
                pc: prev_pc,
                line: prev_line,
            });
        }

        Ok(Self { entries })
    }
}

impl Default for LineTable {
    fn default() -> Self {
        Self::new()
    }
}

/// 局部变量名表
#[derive(Debug, Clone)]
pub struct LocalNameTable {
    /// 条目：局部变量索引 -> 名称（字符串池索引）
    pub entries: Vec<LocalNameEntry>,
}

/// 局部变量名条目
#[derive(Debug, Clone, Copy)]
pub struct LocalNameEntry {
    /// 局部变量索引（在栈中的位置）
    pub local_idx: u32,
    /// 变量名（在字符串池中的索引）
    pub name_idx: u32,
    /// 作用域开始指令
    pub start_pc: u32,
    /// 作用域结束指令
    pub end_pc: u32,
}

impl LocalNameTable {
    /// 创建空的局部变量名表
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// 添加条目
    pub fn add(&mut self, local_idx: u32, name_idx: u32, start_pc: u32, end_pc: u32) {
        self.entries.push(LocalNameEntry {
            local_idx,
            name_idx,
            start_pc,
            end_pc,
        });
    }

    /// 查找指定位置和 PC 的变量名
    pub fn lookup(&self, local_idx: u32, pc: u32) -> Option<u32> {
        self.entries
            .iter()
            .find(|e| e.local_idx == local_idx && e.start_pc <= pc && pc < e.end_pc)
            .map(|e| e.name_idx)
    }

    /// 序列化
    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        result.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());

        for entry in &self.entries {
            result.extend_from_slice(&entry.local_idx.to_le_bytes());
            result.extend_from_slice(&entry.name_idx.to_le_bytes());
            result.extend_from_slice(&entry.start_pc.to_le_bytes());
            result.extend_from_slice(&entry.end_pc.to_le_bytes());
        }

        result
    }

    /// 反序列化
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SectionError> {
        if bytes.len() < 4 {
            return Err(SectionError::TooShort);
        }

        let mut offset = 0;
        let count = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        if bytes.len() < offset + count * 16 {
            return Err(SectionError::TooShort);
        }

        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let local_idx = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            offset += 4;

            let name_idx = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            offset += 4;

            let start_pc = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            offset += 4;

            let end_pc = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            offset += 4;

            entries.push(LocalNameEntry {
                local_idx,
                name_idx,
                start_pc,
                end_pc,
            });
        }

        Ok(Self { entries })
    }
}

impl Default for LocalNameTable {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== 变长整数编码 ====================

/// 编码无符号变长整数 (LEB128 风格)
fn encode_varint(buf: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// 解码无符号变长整数
fn decode_varint(bytes: &[u8]) -> Result<(u64, usize), &'static str> {
    let mut result = 0u64;
    let mut shift = 0;
    let mut offset = 0;

    loop {
        if offset >= bytes.len() {
            return Err("Incomplete varint");
        }

        let byte = bytes[offset];
        offset += 1;

        let value = (byte & 0x7F) as u64;
        result |= value << shift;

        if (byte & 0x80) == 0 {
            break;
        }

        shift += 7;
        if shift >= 64 {
            return Err("Varint too large");
        }
    }

    Ok((result, offset))
}

/// 编码有符号变长整数 (ZigZag + LEB128)
fn encode_signed_varint(buf: &mut Vec<u8>, value: i32) {
    // ZigZag 编码: 将符号位移动到最低位
    let encoded = ((value << 1) ^ (value >> 31)) as u32;
    encode_varint(buf, encoded as u64);
}

/// 解码有符号变长整数
fn decode_signed_varint(bytes: &[u8]) -> Result<(i32, usize), &'static str> {
    let (encoded, bytes_read) = decode_varint(bytes)?;
    // ZigZag 解码
    let value = ((encoded >> 1) as i32) ^ (-((encoded & 1) as i32));
    Ok((value, bytes_read))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_table_basic() {
        let mut table = LineTable::new();
        table.add(0, 1);
        table.add(3, 2);
        table.add(7, 3);

        assert_eq!(table.lookup(0), Some(1));
        assert_eq!(table.lookup(2), Some(1));
        assert_eq!(table.lookup(3), Some(2));
        assert_eq!(table.lookup(6), Some(2));
        assert_eq!(table.lookup(7), Some(3));
        assert_eq!(table.lookup(100), Some(3));
    }

    #[test]
    fn test_line_table_from_chunk() {
        // 模拟 Chunk::lines 数组 [1, 1, 1, 2, 2, 3, 3, 3]
        let lines = vec![1, 1, 1, 2, 2, 3, 3, 3];
        let table = LineTable::from_chunk_lines(&lines);

        assert_eq!(table.lookup(0), Some(1));
        assert_eq!(table.lookup(2), Some(1));
        assert_eq!(table.lookup(3), Some(2));
        assert_eq!(table.lookup(5), Some(3));
    }

    #[test]
    fn test_line_table_roundtrip() {
        let mut table = LineTable::new();
        table.add(0, 10);
        table.add(10, 12);
        table.add(25, 15);
        table.add(50, 20);

        let bytes = table.serialize();
        let table2 = LineTable::deserialize(&bytes).unwrap();

        assert_eq!(table.entries.len(), table2.entries.len());
        for (a, b) in table.entries.iter().zip(table2.entries.iter()) {
            assert_eq!(a.pc, b.pc);
            assert_eq!(a.line, b.line);
        }
    }

    #[test]
    fn test_local_name_table() {
        let mut table = LocalNameTable::new();
        table.add(0, 1, 0, 10); // local 0, name 1, scope [0, 10)
        table.add(1, 2, 0, 20); // local 1, name 2, scope [0, 20)
        table.add(2, 3, 5, 15); // local 2, name 3, scope [5, 15)

        assert_eq!(table.lookup(0, 5), Some(1));
        assert_eq!(table.lookup(1, 15), Some(2));
        assert_eq!(table.lookup(2, 10), Some(3));
        assert_eq!(table.lookup(2, 3), None); // 不在作用域内
    }

    #[test]
    fn test_local_name_table_roundtrip() {
        let mut table = LocalNameTable::new();
        table.add(0, 1, 0, 100);
        table.add(1, 2, 50, 150);

        let bytes = table.serialize();
        let table2 = LocalNameTable::deserialize(&bytes).unwrap();

        assert_eq!(table.entries.len(), table2.entries.len());
        assert_eq!(table.entries[0].local_idx, table2.entries[0].local_idx);
        assert_eq!(table.entries[0].name_idx, table2.entries[0].name_idx);
    }

    #[test]
    fn test_debug_info_roundtrip() {
        let mut info = DebugInfo::new();
        info.set_source_path(42);
        info.line_table.add(0, 10);
        info.line_table.add(10, 12);
        info.local_names.add(0, 1, 0, 100);

        let bytes = info.serialize();
        let info2 = DebugInfo::deserialize(&bytes).unwrap();

        assert_eq!(info.source_path_idx, info2.source_path_idx);
        assert_eq!(info.line_table.entries.len(), info2.line_table.entries.len());
        assert_eq!(info.local_names.entries.len(), info2.local_names.entries.len());
    }

    #[test]
    fn test_varint() {
        let test_values = [0u64, 1, 127, 128, 255, 256, 16383, 16384, 65535, 65536, u64::MAX];

        for &value in &test_values {
            let mut buf = Vec::new();
            encode_varint(&mut buf, value);
            let (decoded, bytes_read) = decode_varint(&buf).unwrap();
            assert_eq!(decoded, value, "Failed for value {}", value);
            assert_eq!(bytes_read, buf.len());
        }
    }

    #[test]
    fn test_signed_varint() {
        let test_values = [0i32, 1, -1, 127, -127, 128, -128, 1000, -1000, i32::MAX, i32::MIN];

        for &value in &test_values {
            let mut buf = Vec::new();
            encode_signed_varint(&mut buf, value);
            let (decoded, bytes_read) = decode_signed_varint(&buf).unwrap();
            assert_eq!(decoded, value, "Failed for value {}", value);
            assert_eq!(bytes_read, buf.len());
        }
    }
}
