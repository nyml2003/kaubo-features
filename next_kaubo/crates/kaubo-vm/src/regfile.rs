//! 寄存器文件 — 分层值表示 (Int64 / Float64 / Ptr)

use std::alloc::{alloc, Layout};
use std::fmt;

/// GC 堆指针 (引用计数)
#[derive(Debug, Clone)]
pub struct GcPtr {
    pub ptr: *mut u8,
    pub rc: *mut u32,
}

unsafe impl Send for GcPtr {}

impl GcPtr {
    pub fn null() -> Self { GcPtr { ptr: std::ptr::null_mut(), rc: std::ptr::null_mut() } }
    pub fn is_null(&self) -> bool { self.ptr.is_null() }
}

/// 物理寄存器定位
#[derive(Debug, Clone, Copy)]
pub enum RegLoc {
    Int(usize),
    Float(usize),
    Ptr(usize),
}

/// 寄存器文件
pub struct RegFile {
    pub ints: Vec<i64>,
    pub floats: Vec<f64>,
    pub ptrs: Vec<GcPtr>,
    pub map: Vec<RegLoc>,
    pub gc: Gc,
}

impl RegFile {
    pub fn new(int_cap: usize, float_cap: usize, ptr_cap: usize) -> Self {
        RegFile {
            ints: vec![0; int_cap],
            floats: vec![0.0; float_cap],
            ptrs: vec![GcPtr::null(); ptr_cap],
            map: Vec::new(),
            gc: Gc::new(),
        }
    }

    pub fn ensure_capacity(&mut self, ints: usize, floats: usize, ptrs: usize) {
        if self.ints.len() < ints { self.ints.resize(ints, 0); }
        if self.floats.len() < floats { self.floats.resize(floats, 0.0); }
        if self.ptrs.len() < ptrs { self.ptrs.resize(ptrs, GcPtr::null()); }
    }

    pub fn frame_end(&mut self, int_base: usize, float_base: usize, ptr_base: usize, ptr_count: usize) {
        for i in ptr_base..ptr_base + ptr_count {
            if !self.ptrs[i].is_null() {
                self.gc.release(&mut self.ptrs[i]);
            }
        }
    }
}

// ── GC — 引用计数 ──

pub struct Gc;

impl Gc {
    pub fn new() -> Self { Gc }

    pub fn alloc(&mut self, size: usize) -> GcPtr {
        let layout = Layout::from_size_align(size + 4, 8).unwrap();
        let raw = unsafe { alloc(layout) };
        if raw.is_null() { panic!("OOM"); }
        unsafe {
            let rc_ptr = raw as *mut u32;
            *rc_ptr = 1;
            GcPtr { ptr: raw.add(4), rc: rc_ptr }
        }
    }

    pub fn retain(&self, p: &GcPtr) {
        if !p.is_null() {
            unsafe { *p.rc += 1; }
        }
    }

    pub fn release(&self, p: &mut GcPtr) {
        if !p.is_null() {
            unsafe {
                *p.rc -= 1;
                if *p.rc == 0 {
                    // Free memory (simplified — in production would walk object graph)
                    std::alloc::dealloc(p.ptr.sub(4), Layout::from_size_align(1, 8).unwrap());
                }
            }
        }
        *p = GcPtr::null();
    }
}

impl fmt::Display for RegFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RegFile(ints:{}, floats:{}, ptrs:{})", self.ints.len(), self.floats.len(), self.ptrs.len())
    }
}
