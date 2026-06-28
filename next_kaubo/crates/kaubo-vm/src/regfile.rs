//! 寄存器文件 — 统一 u64 寄存器组（JVM/WASM 风格）
//!
//! 所有值存储为 u64。操作码决定位模式的解释方式：
//!   AddInt → reg as i64,   FAdd → f64::from_bits(reg)

/// 寄存器文件：单组 Vec<u64>，不分 int/float
pub struct RegFile {
    pub regs: Vec<u64>,
}

impl RegFile {
    pub fn new(cap: usize) -> Self {
        RegFile { regs: vec![0; cap] }
    }

    pub fn ensure_capacity(&mut self, n: usize) {
        if self.regs.len() < n {
            self.regs.resize(n, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_capacity_grows() {
        let mut rf = RegFile::new(1);
        rf.ensure_capacity(5);
        assert!(rf.regs.len() >= 5);
    }

    #[test]
    fn new_regfile_is_zero_initialized() {
        let rf = RegFile::new(10);
        for i in 0..10 {
            assert_eq!(rf.regs[i], 0);
        }
    }
}
