//! 寄存器文件 — 分层值表示 (Int64 / Float64)

use std::fmt;

/// 寄存器文件
pub struct RegFile {
    pub ints: Vec<i64>,
    pub floats: Vec<f64>,
}

impl RegFile {
    pub fn new(int_cap: usize, float_cap: usize) -> Self {
        RegFile {
            ints: vec![0; int_cap],
            floats: vec![0.0; float_cap],
        }
    }

    pub fn ensure_capacity(&mut self, ints: usize, floats: usize) {
        if self.ints.len() < ints { self.ints.resize(ints, 0); }
        if self.floats.len() < floats { self.floats.resize(floats, 0.0); }
    }
}

impl fmt::Display for RegFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RegFile(ints:{}, floats:{})", self.ints.len(), self.floats.len())
    }
}
