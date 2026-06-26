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
        if self.ints.len() < ints {
            self.ints.resize(ints, 0);
        }
        if self.floats.len() < floats {
            self.floats.resize(floats, 0.0);
        }
    }
}

impl fmt::Display for RegFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "RegFile(ints:{}, floats:{})",
            self.ints.len(),
            self.floats.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_capacity_grows_both_register_sets() {
        let mut rf = RegFile::new(1, 1);
        rf.ensure_capacity(4, 3);

        assert_eq!(rf.ints.len(), 4);
        assert_eq!(rf.floats.len(), 3);
        assert_eq!(rf.ints[0], 0);
        assert_eq!(rf.floats[0], 0.0);
    }

    #[test]
    fn display_reports_sizes() {
        let rf = RegFile::new(2, 5);
        assert_eq!(rf.to_string(), "RegFile(ints:2, floats:5)");
    }

    #[test]
    fn ensure_capacity_grows() {
        let mut rf = RegFile::new(1, 1);
        rf.ensure_capacity(5, 3);
        assert!(rf.ints.len() >= 5);
        assert!(rf.floats.len() >= 3);
    }

    #[test]
    fn new_regfile_is_zero_initialized() {
        let rf = RegFile::new(10, 10);
        for i in 0..10 {
            assert_eq!(rf.ints[i], 0);
            assert_eq!(rf.floats[i], 0.0);
        }
    }
}
