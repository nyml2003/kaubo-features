//! 寄存器 VM — 完整实现
//! 7-bit opcode, CPS block scheduler, 调用栈 + 闭包 + stdlib

use crate::async_runtime::AsyncScheduler;
use crate::gc_heap::GcHeap;
use crate::regfile::*;
use crate::stdlib;
use kaubo_cps::*;

// ── 编码 ──
pub fn encode(op: u8, dst: u32, src1: u32, src2: u32) -> u32 {
    ((op as u32) << 25) | ((dst & 0xFF) << 17) | ((src1 & 0x1FF) << 8) | (src2 & 0xFF)
}

// ── 运行时错误 ──
#[derive(Debug)]
pub enum RuntimeError {
    DivisionByZero,
    IndexOutOfBounds(i64, usize),
    NullAccess,
    TypeAssertion(String),
    StackOverflow,
    Bug(String),
}

// ── 堆对象 ──
#[derive(Debug, Clone)]
pub enum HeapObj {
    String(String),
    List(Vec<i64>),
    Struct(usize, Vec<i64>), // (struct_id, field_values)
    Closure(Box<ClosureObj>),
}

#[derive(Debug, Clone)]
pub struct ClosureObj {
    pub func_entry: usize,
    pub upvalues: Vec<i64>, // captured values (copied)
}

// ── VM ──

pub struct VM {
    pub regs: RegFile,
    pub frames: Vec<CallFrame>,
    pub consts: Vec<Constant>,
    // Per-function data
    pub func_blocks: Vec<Vec<(usize, usize)>>,
    pub func_params: Vec<Vec<Vec<usize>>>,
    pub jump_args: Vec<Vec<usize>>, // flat global jump_args by absolute IP
    pub func_entries: Vec<usize>,
    pub func_reg_counts: Vec<usize>,
    pub func_instr_base: Vec<usize>, // start IP in flat instrs array
    pub block_starts: Vec<usize>,    // flat: block_id → start IP (per func)
    pub func_block_base: Vec<usize>, // offset into block_starts per function
    pub current_func: usize,
    pub instrs: Vec<u32>,
    pub output: Vec<String>,
    pub debug: bool, // transient

    pub heap: super::gc_heap::GcHeap,
    pub struct_bitmaps: Vec<u64>,
    pub struct_field_counts: Vec<usize>,
    pub natives: Vec<(&'static str, stdlib::NativeFn)>,
    pub scheduler: AsyncScheduler,
}

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub func_idx: usize,
    pub ret_block: usize,
    pub saved_ints: Vec<i64>,
    pub saved_floats: Vec<f64>,
    pub result_reg: usize,
}

const MAX_CALL_DEPTH: usize = 1024;

impl VM {
    pub fn new() -> Self {
        VM {
            regs: RegFile::new(512, 256),
            frames: vec![],
            consts: vec![],
            func_blocks: vec![],
            func_params: vec![],
            func_entries: vec![],
            func_reg_counts: vec![],
            func_instr_base: vec![],
            current_func: 0,
            block_starts: vec![],
            func_block_base: vec![],
            jump_args: vec![],
            instrs: vec![],
            output: vec![],
            debug: false,
            heap: GcHeap::new(),
            struct_bitmaps: vec![],
            struct_field_counts: vec![],
            natives: stdlib::register_all(),
            scheduler: AsyncScheduler::new(),
        }
    }

    pub fn load(&mut self, module: &CpsModule) -> Result<(), String> {
        self.consts = module.constants.clone();
        self.instrs.clear();
        self.func_blocks.clear();
        self.func_params.clear();
        self.func_entries.clear();
        self.func_reg_counts.clear();
        self.func_instr_base.clear();
        self.block_starts.clear();
        self.func_block_base.clear();
        self.jump_args.clear();
        self.struct_bitmaps.clear();
        self.struct_field_counts.clear();

        for sd in &module.structs {
            let id = sd.id;
            if id >= self.struct_bitmaps.len() {
                self.struct_bitmaps.resize(id + 1, 0);
                self.struct_field_counts.resize(id + 1, 0);
            }
            self.struct_bitmaps[id] = sd.type_bitmap;
            self.struct_field_counts[id] = sd.fields.len();
        }

        for func in &module.functions {
            let base_ip = self.instrs.len();
            let max_id = func
                .blocks
                .iter()
                .filter(|b| b.id != usize::MAX)
                .map(|b| b.id)
                .max()
                .unwrap_or(0)
                + 1;
            let mut blocks = vec![(0, 0); max_id];
            let mut params = vec![vec![]; max_id];

            for block in &func.blocks {
                if block.id == usize::MAX {
                    continue;
                }
                let start = self.instrs.len();
                for instr in &block.instrs {
                    self.instrs.push(encode_instr(instr)?);
                    self.jump_args.push(vec![]); // placeholder for instruction
                }
                let args = match &block.term {
                    CpsTerminator::Jump(_, a) => a.clone(),
                    CpsTerminator::Branch(_, _, a, _, _) => {
                        let mut all = a.clone();
                        all.push(0);
                        all
                    }
                    CpsTerminator::Call(_, a, _) => a.clone(),
                    CpsTerminator::CallNative(_, a, _) => a.clone(),
                    _ => vec![],
                };
                self.jump_args.push(args); // args for terminator
                self.instrs.push(encode_term(&block.term)?);
                blocks[block.id] = (start, self.instrs.len() - start);
                params[block.id] = block.params.clone();
            }
            let entry_ip = blocks[func.entry].0;
            // Build flat block_starts before moving blocks
            self.func_block_base.push(self.block_starts.len());
            for b in &blocks {
                self.block_starts.push(b.0);
            }
            self.func_blocks.push(blocks);
            self.func_params.push(params);
            self.func_entries.push(entry_ip);
            self.func_reg_counts.push(func.reg_count);
            self.func_instr_base.push(base_ip);
        }
        Ok(())
    }

    fn block_ip(&self, block_id: usize) -> usize {
        self.block_starts[self.func_block_base[self.current_func] + block_id]
    }

    fn jump_args(&self, abs_ip: usize) -> &[usize] {
        &self.jump_args[abs_ip]
    }

    fn bind_params(&mut self, block_id: usize, args: &[usize]) {
        let params = &self.func_params[self.current_func][block_id];
        if params.is_empty() && args.is_empty() {
            return;
        }
        for (i, &arg_reg) in args.iter().enumerate() {
            if i < params.len() {
                self.regs.ints[params[i]] = self.regs.ints[arg_reg];
                self.regs.floats[params[i]] = self.regs.floats[arg_reg];
            }
        }
    }

    fn alloc_heap(&mut self, obj: HeapObj) -> i64 {
        self.heap.alloc(obj) as i64
    }

    fn heap_get(&self, id: i64) -> &HeapObj {
        self.heap.get(id as usize)
    }

    fn heap_get_mut(&mut self, id: i64) -> &mut HeapObj {
        self.heap.get_mut(id as usize)
    }

    pub fn execute(&mut self, entry_func: usize, _reg_count: usize) -> Result<i64, RuntimeError> {
        self.current_func = entry_func;
        let reg_needed = self.func_reg_counts[entry_func];
        self.regs.ensure_capacity(reg_needed, reg_needed);
        if self.regs.ints.len() < reg_needed {
            self.regs.ints.resize(reg_needed, 0);
        }

        let mut ip = self.func_entries[entry_func];

        loop {
            let inst = self.instrs[ip];
            ip += 1;
            let op = (inst >> 25) as u8;

            if self.debug && cfg!(debug_assertions) {
                eprintln!(
                    "[VM fn={} ip={}] op={:#04x} inst={:#010x} ints[0..4]={:?}",
                    self.current_func,
                    ip - 1,
                    op,
                    inst,
                    &self.regs.ints[..4.min(self.regs.ints.len())]
                );
            }

            match op {
                // ── 整数算术 ──
                0x00 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let c = (inst & 0xFF) as usize;
                    self.regs.ints[a] = self.regs.ints[b].wrapping_add(self.regs.ints[c]);
                }
                0x01 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let c = (inst & 0xFF) as usize;
                    self.regs.ints[a] = self.regs.ints[b].wrapping_sub(self.regs.ints[c]);
                }
                0x02 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let c = (inst & 0xFF) as usize;
                    self.regs.ints[a] = self.regs.ints[b].wrapping_mul(self.regs.ints[c]);
                }
                0x03 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let c = (inst & 0xFF) as usize;
                    if self.regs.ints[c] == 0 {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    self.regs.ints[a] = self.regs.ints[b] / self.regs.ints[c];
                }
                0x04 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let c = (inst & 0xFF) as usize;
                    if self.regs.ints[c] == 0 {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    self.regs.ints[a] = self.regs.ints[b] % self.regs.ints[c];
                }
                0x05 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    self.regs.ints[a] = -self.regs.ints[b];
                }

                // ── 浮点 ──
                0x08 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let c = (inst & 0xFF) as usize;
                    let f = self.regs.floats[b] + self.regs.floats[c];
                    self.regs.floats[a] = f;
                    self.regs.ints[a] = f.to_bits() as i64;
                }
                0x09 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let c = (inst & 0xFF) as usize;
                    let f = self.regs.floats[b] - self.regs.floats[c];
                    self.regs.floats[a] = f;
                    self.regs.ints[a] = f.to_bits() as i64;
                }
                0x0A => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let c = (inst & 0xFF) as usize;
                    let f = self.regs.floats[b] * self.regs.floats[c];
                    self.regs.floats[a] = f;
                    self.regs.ints[a] = f.to_bits() as i64;
                }
                0x0B => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let c = (inst & 0xFF) as usize;
                    let f = self.regs.floats[b] / self.regs.floats[c];
                    self.regs.floats[a] = f;
                    self.regs.ints[a] = f.to_bits() as i64;
                }
                0x0C => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let f = -self.regs.floats[b];
                    self.regs.floats[a] = f;
                    self.regs.ints[a] = f.to_bits() as i64;
                }

                // ── 比较 ──
                0x10 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let c = (inst & 0xFF) as usize;
                    self.regs.ints[a] = (self.regs.ints[b] == self.regs.ints[c]) as i64;
                }
                0x11 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let c = (inst & 0xFF) as usize;
                    self.regs.ints[a] = (self.regs.ints[b] < self.regs.ints[c]) as i64;
                }
                0x12 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let c = (inst & 0xFF) as usize;
                    self.regs.ints[a] = (self.regs.ints[b] <= self.regs.ints[c]) as i64;
                }
                0x13 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let c = (inst & 0xFF) as usize;
                    self.regs.floats[a] =
                        (self.regs.floats[b] == self.regs.floats[c]) as u64 as f64;
                }
                0x14 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let c = (inst & 0xFF) as usize;
                    self.regs.floats[a] = (self.regs.floats[b] < self.regs.floats[c]) as u64 as f64;
                }

                // ── Not ──
                0x15 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    self.regs.ints[a] = (self.regs.ints[b] == 0) as i64;
                }
                0x16 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    let c = (inst & 0xFF) as usize;
                    self.regs.ints[a] = (self.regs.ints[b] != self.regs.ints[c]) as i64;
                }

                // ── 字符串 ──
                0x18 => {
                    let a = ((inst >> 17) & 0xFF) as usize;
                    let b = ((inst >> 8) & 0x1FF) as usize;
                    self.regs.ints[a] = self.regs.ints[b].wrapping_add(0);
                }

                // ── 转换 ──
                0x20 => {
                    let d = ((inst >> 17) & 0xFF) as usize;
                    let s = ((inst >> 8) & 0x1FF) as usize;
                    let f = self.regs.ints[s] as f64;
                    self.regs.floats[d] = f;
                    self.regs.ints[d] = f.to_bits() as i64;
                }
                0x21 => {
                    let d = ((inst >> 17) & 0xFF) as usize;
                    let s = ((inst >> 8) & 0x1FF) as usize;
                    let f = self.regs.floats[s];
                    self.regs.ints[d] = f as i64;
                    self.regs.floats[d] = f;
                }
                0x22 => {
                    // itos
                    let d = ((inst >> 17) & 0xFF) as usize;
                    let s = ((inst >> 8) & 0x1FF) as usize;
                    let st = format!("{}", self.regs.ints[s]);
                    let hid = self.alloc_heap(HeapObj::String(st));
                    self.regs.ints[d] = hid;
                }
                0x23 => {
                    // ftos
                    let d = ((inst >> 17) & 0xFF) as usize;
                    let s = ((inst >> 8) & 0x1FF) as usize;
                    let s = format!("{}", self.regs.floats[s]);
                    let hid = self.alloc_heap(HeapObj::String(s));
                    self.regs.ints[d] = hid;
                }
                0x24 => {
                    let d = ((inst >> 17) & 0xFF) as usize;
                    let s = ((inst >> 8) & 0x1FF) as usize;
                    self.regs.ints[d] = self.regs.ints[s];
                } // stoi placeholder

                // ── 数据移动 ──
                0x30 => {
                    let d = ((inst >> 17) & 0xFF) as usize;
                    let s = ((inst >> 8) & 0x1FF) as usize;
                    self.regs.ints[d] = self.regs.ints[s];
                    self.regs.floats[d] = self.regs.floats[s];
                }
                0x31 => {
                    let d = ((inst >> 17) & 0xFF) as usize;
                    self.regs.ints[d] = (inst & 0x1FFFF) as i64;
                }
                0x32 => {
                    let d = ((inst >> 17) & 0xFF) as usize;
                    let idx = ((inst >> 8) & 0xFF) as usize;
                    match &self.consts[idx] {
                        Constant::Int(n) => self.regs.ints[d] = *n,
                        Constant::Float(f) => {
                            self.regs.floats[d] = *f;
                            self.regs.ints[d] = f.to_bits() as i64;
                        }
                        Constant::String(s) => {
                            let hid = self.alloc_heap(HeapObj::String(s.clone()));
                            self.regs.ints[d] = hid;
                        }
                        _ => {}
                    }
                }

                // ── 堆分配 ──
                0x34 => {
                    let d = ((inst >> 17) & 0xFF) as usize;
                    let sid = ((inst >> 8) & 0xFF) as usize;
                    let nf = self.struct_field_counts.get(sid).copied().unwrap_or(0);
                    let bitmap = self.struct_bitmaps.get(sid).copied().unwrap_or(0);
                    let mut fields = vec![0i64; nf];
                    for i in 0..nf {
                        if (bitmap >> i) & 1 != 0 {
                            fields[i] = -1; // null sentinel for heap-type fields
                        }
                    }
                    self.regs.ints[d] = self.alloc_heap(HeapObj::Struct(sid, fields));
                }
                0x35 => {
                    // NewList(dst, elements)
                    let d = ((inst >> 17) & 0xFF) as usize;
                    let ne = ((inst >> 8) & 0xFF) as usize;
                    let hid = self.alloc_heap(HeapObj::List(vec![0; ne]));
                    self.regs.ints[d] = hid;
                }

                // ── 字段访问 ──
                0x36 => {
                    // GetField(dst, src, idx)
                    let d = ((inst >> 17) & 0xFF) as usize;
                    let s = ((inst >> 8) & 0x1FF) as usize;
                    let idx = (inst & 0xFF) as usize;
                    let hid = self.regs.ints[s];
                    if let HeapObj::Struct(_, fields) = self.heap_get(hid) {
                        let val = *fields.get(idx).unwrap_or(&0);
                        self.regs.ints[d] = val;
                        self.regs.floats[d] = f64::from_bits(val as u64);
                    }
                }
                0x37 => {
                    // SetField(dst, src, idx, val)
                    let d = ((inst >> 17) & 0xFF) as usize;
                    let s = ((inst >> 8) & 0x1FF) as usize;
                    let idx = (inst & 0xFF) as usize;
                    let hid = self.regs.ints[s];
                    let val = self.regs.ints[d];

                    // Read struct_id and old field value
                    let (sid, old_val) = if let HeapObj::Struct(sid, fields) = self.heap_get(hid) {
                        (*sid, fields.get(idx).copied().unwrap_or(0))
                    } else {
                        (0, 0)
                    };
                    // Check if this field is a heap type
                    let is_heap =
                        (self.struct_bitmaps.get(sid).copied().unwrap_or(0) >> idx) & 1 != 0;

                    // Release old value if heap type
                    if is_heap && old_val >= 0 {
                        self.heap.release(old_val as usize);
                    }

                    // Write new value
                    if let HeapObj::Struct(_, fields) = self.heap_get_mut(hid) {
                        if idx < fields.len() {
                            fields[idx] = val;
                        }
                    }

                    // Retain new value if heap type
                    if is_heap && val >= 0 {
                        self.heap.retain(val as usize);
                    }
                }

                // ── 索引 ──
                0x38 => {
                    // IndexGet(dst, obj, idx)
                    let d = ((inst >> 17) & 0xFF) as usize;
                    let o = ((inst >> 8) & 0x1FF) as usize;
                    let i = (inst & 0xFF) as usize;
                    let hid = self.regs.ints[o];
                    let index = self.regs.ints[i] as usize;
                    match self.heap_get(hid) {
                        HeapObj::List(v) => {
                            self.regs.ints[d] = *v
                                .get(index)
                                .ok_or(RuntimeError::IndexOutOfBounds(index as i64, v.len()))?;
                        }
                        _ => {}
                    }
                }

                // ── 装箱/拆箱 ──
                0x3A => {
                    let d = ((inst >> 17) & 0xFF) as usize;
                    let s = ((inst >> 8) & 0x1FF) as usize;
                    self.regs.ints[d] = self.regs.ints[s];
                    self.regs.floats[d] = self.regs.floats[s];
                } // box
                0x3B => {
                    let d = ((inst >> 17) & 0xFF) as usize;
                    let s = ((inst >> 8) & 0x1FF) as usize;
                    self.regs.ints[d] = self.regs.ints[s];
                    self.regs.floats[d] = self.regs.floats[s];
                } // unbox

                // ── 控制流 ──
                0x40 => {
                    let block_id = (inst & 0x1FFFFFF) as usize;
                    self.bind_params(block_id, &self.jump_args(ip - 1).to_vec());
                    ip = self.block_ip(block_id);
                }
                0x41 => {
                    let c = ((inst >> 17) & 0xFF) as usize;
                    let tb = ((inst >> 8) & 0x1FF) as usize;
                    let fb = (inst & 0xFF) as usize;
                    let block_id = if self.regs.ints[c] != 0 { tb } else { fb };
                    self.bind_params(block_id, &self.jump_args(ip - 1).to_vec());
                    ip = self.block_ip(block_id);
                }

                // ── 调用 ──
                0x50 => {
                    // Call(func_idx, args, cont_block)
                    let func_idx = ((inst >> 17) & 0xFF) as usize;
                    let cont_block = (inst & 0x1FFFF) as usize;
                    if self.frames.len() >= MAX_CALL_DEPTH {
                        return Err(RuntimeError::StackOverflow);
                    }
                    let callee_regs = self.func_reg_counts[func_idx];
                    let mut callee_ints = vec![0; callee_regs];
                    let mut callee_floats = vec![0.0; callee_regs];
                    let args = &self.jump_args(ip - 1).to_vec();
                    for (i, &arg_reg) in args.iter().enumerate() {
                        if i < callee_regs {
                            callee_ints[i] = self.regs.ints[arg_reg];
                            callee_floats[i] = self.regs.floats[arg_reg];
                        }
                    }
                    let saved_ints = std::mem::replace(&mut self.regs.ints, callee_ints);
                    let saved_floats = std::mem::replace(&mut self.regs.floats, callee_floats);
                    self.frames.push(CallFrame {
                        func_idx: self.current_func,
                        ret_block: cont_block,
                        saved_ints,
                        saved_floats,
                        result_reg: 0,
                    });
                    self.current_func = func_idx;
                    ip = self.func_entries[func_idx];
                }
                0x51 => {
                    ip = self.func_entries[self.current_func];
                }
                0x52 => {
                    // ret
                    let r = ((inst >> 17) & 0xFF) as usize;
                    if let Some(frame) = self.frames.pop() {
                        let result_i = self.regs.ints[r];
                        let result_f = self.regs.floats[r];
                        self.regs.ints = frame.saved_ints;
                        self.regs.floats = frame.saved_floats;
                        self.current_func = frame.func_idx;
                        if self.regs.ints.len() <= frame.result_reg {
                            self.regs.ints.resize(frame.result_reg + 1, 0);
                            self.regs.floats.resize(frame.result_reg + 1, 0.0);
                        }
                        self.regs.ints[frame.result_reg] = result_i;
                        self.regs.floats[frame.result_reg] = result_f;
                        ip = self.block_ip(frame.ret_block);
                    } else {
                        return Ok(self.regs.ints[r]);
                    }
                }

                // ── native call ──
                0x5F => {
                    let fi = ((inst >> 17) & 0xFF) as usize;
                    let ret_block = (inst & 0x1FFFF) as usize;
                    let args: Vec<i64> = self
                        .jump_args(ip - 1)
                        .iter()
                        .map(|&r| self.regs.ints[r])
                        .collect();
                    if fi < self.natives.len() {
                        let result = (self.natives[fi].1)(&args).unwrap_or(0);
                        self.regs.ints[0] = result;
                        self.regs.floats[0] = f64::from_bits(result as u64);
                    }
                    ip = self.block_ip(ret_block);
                }

                // ── async ──
                0x60 => {
                    if let Some((_, result)) = self.scheduler.poll() {
                        self.regs.ints[0] = result;
                    }
                }
                0x61 => {
                    // suspend
                    let cf = CallFrame {
                        func_idx: self.current_func,
                        ret_block: 0,
                        saved_ints: self.regs.ints.clone(),
                        saved_floats: self.regs.floats.clone(),
                        result_reg: 0,
                    };
                    self.scheduler.suspend(cf, ip);
                    return Ok(0);
                }

                // ── print ──
                0x7F => {
                    let r = ((inst >> 17) & 0xFF) as usize;
                    let val = self.regs.ints[r];
                    if val >= 0 {
                        if let Some(HeapObj::String(s)) = self.heap.try_get(val as usize) {
                            self.output.push(s.clone());
                        } else {
                            self.output.push(format!("{}", val));
                        }
                    } else {
                        self.output.push(format!("{}", val));
                    }
                }

                _ => {}
            }
        }
    }
}

// ── 指令编码 ──

fn encode_instr(instr: &CpsInstr) -> Result<u32, String> {
    Ok(match instr {
        CpsInstr::BinOp(d, op, s1, s2) => encode(
            match op {
                CpsBinOp::AddInt => 0x00,
                CpsBinOp::SubInt => 0x01,
                CpsBinOp::MulInt => 0x02,
                CpsBinOp::DivInt => 0x03,
                CpsBinOp::ModInt => 0x04,
                CpsBinOp::FAdd => 0x08,
                CpsBinOp::FSub => 0x09,
                CpsBinOp::FMul => 0x0A,
                CpsBinOp::FDiv => 0x0B,
                CpsBinOp::FEq => 0x13,
                CpsBinOp::FLt => 0x14,
                CpsBinOp::EqInt => 0x10,
                CpsBinOp::NeInt => 0x16,
                CpsBinOp::LtInt => 0x11,
                CpsBinOp::LeInt => 0x12,
                CpsBinOp::GtInt => 0x12,
                CpsBinOp::GeInt => 0x12,
                CpsBinOp::IToF => 0x20,
                CpsBinOp::FToI => 0x21,
                CpsBinOp::IToS => 0x22,
                CpsBinOp::FToS => 0x23,
                CpsBinOp::SToI => 0x24,
                CpsBinOp::SAdd => 0x18,
            },
            *d as u32,
            *s1 as u32,
            *s2 as u32,
        ),
        CpsInstr::UnOp(d, op, s) => encode(
            match op {
                CpsUnOp::NegInt => 0x05,
                CpsUnOp::FNeg => 0x0C,
                CpsUnOp::Not => 0x15,
            },
            *d as u32,
            *s as u32,
            0,
        ),
        CpsInstr::LoadConst(d, idx) => encode(0x32, *d as u32, *idx as u32, 0),
        CpsInstr::Move(d, s) => encode(0x30, *d as u32, *s as u32, 0),
        CpsInstr::NewStruct(d, sid, _) => encode(0x34, *d as u32, *sid as u32, 0),
        CpsInstr::GetField(d, o, idx) => encode(0x36, *d as u32, *o as u32, *idx as u32),
        CpsInstr::SetField(d, o, idx, _) => encode(0x37, *d as u32, *o as u32, *idx as u32),
        CpsInstr::NewList(d, _) => encode(0x35, *d as u32, 0, 0),
        CpsInstr::IndexGet(d, o, i) => encode(0x38, *d as u32, *o as u32, *i as u32),
        CpsInstr::IndexSet(_, _, _, _) => encode(0x39, 0, 0, 0),
        CpsInstr::Box(d, s) => encode(0x3A, *d as u32, *s as u32, 0),
        CpsInstr::Unbox(d, s) => encode(0x3B, *d as u32, *s as u32, 0),
        CpsInstr::Print(r) => encode(0x7F, *r as u32, 0, 0),
        CpsInstr::Nop => 0,
    })
}

fn encode_term(term: &CpsTerminator) -> Result<u32, String> {
    Ok(match term {
        CpsTerminator::Jump(b, _) => encode(0x40, 0, 0, *b as u32),
        CpsTerminator::Branch(c, tb, _, fb, _) => encode(0x41, *c as u32, *tb as u32, *fb as u32),
        CpsTerminator::Suspend => encode(0x61, 0, 0, 0),
        CpsTerminator::Return(r) => encode(0x52, *r as u32, 0, 0),
        CpsTerminator::Call(fi, _, ret) => encode(0x50, *fi as u32, 0, *ret as u32),
        CpsTerminator::CallNative(fi, _, ret) => encode(0x5F, *fi as u32, 0, *ret as u32),
        CpsTerminator::TailCall(_, _) => encode(0x51, 0, 0, 0),
    })
}

// ── tests ──

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_mod(
        instrs: Vec<CpsInstr>,
        term: CpsTerminator,
        consts: Vec<Constant>,
        reg_count: usize,
    ) -> CpsModule {
        CpsModule {
            functions: vec![CpsFunction {
                name: "main".into(),
                blocks: vec![CpsBlock {
                    id: 0,
                    params: vec![],
                    instrs,
                    term,
                }],
                entry: 0,
                reg_count,
            }],
            constants: consts,
            structs: vec![],
        }
    }

    #[test]
    fn test_add() {
        let m = simple_mod(
            vec![
                CpsInstr::LoadConst(0, 0),
                CpsInstr::LoadConst(1, 1),
                CpsInstr::BinOp(2, CpsBinOp::AddInt, 0, 1),
            ],
            CpsTerminator::Return(2),
            vec![Constant::Int(40), Constant::Int(2)],
            3,
        );
        let mut vm = VM::new();
        vm.load(&m).unwrap();
        assert_eq!(vm.execute(0, 3).unwrap(), 42);
    }

    #[test]
    fn test_div_zero() {
        let m = simple_mod(
            vec![
                CpsInstr::LoadConst(0, 0),
                CpsInstr::LoadConst(1, 1),
                CpsInstr::BinOp(2, CpsBinOp::DivInt, 0, 1),
            ],
            CpsTerminator::Return(2),
            vec![Constant::Int(42), Constant::Int(0)],
            3,
        );
        let mut vm = VM::new();
        vm.load(&m).unwrap();
        assert!(matches!(
            vm.execute(0, 3),
            Err(RuntimeError::DivisionByZero)
        ));
    }

    #[test]
    fn test_branch() {
        let m = CpsModule {
            functions: vec![CpsFunction {
                name: "main".into(),
                blocks: vec![
                    CpsBlock {
                        id: 0,
                        params: vec![],
                        instrs: vec![CpsInstr::LoadConst(0, 0), CpsInstr::LoadConst(1, 1)],
                        term: CpsTerminator::Branch(0, 1, vec![], 2, vec![]),
                    },
                    CpsBlock {
                        id: 1,
                        params: vec![],
                        instrs: vec![CpsInstr::LoadConst(2, 2)],
                        term: CpsTerminator::Jump(3, vec![2]),
                    },
                    CpsBlock {
                        id: 2,
                        params: vec![],
                        instrs: vec![CpsInstr::LoadConst(2, 3)],
                        term: CpsTerminator::Jump(3, vec![2]),
                    },
                    CpsBlock {
                        id: 3,
                        params: vec![2],
                        instrs: vec![],
                        term: CpsTerminator::Return(2),
                    },
                ],
                entry: 0,
                reg_count: 4,
            }],
            constants: vec![
                Constant::Int(1),
                Constant::Int(0),
                Constant::Int(10),
                Constant::Int(20),
            ],
            structs: vec![],
        };
        let mut vm = VM::new();
        vm.load(&m).unwrap();
        assert_eq!(vm.execute(0, 4).unwrap(), 10);
    }

    #[test]
    fn test_neg() {
        let m = simple_mod(
            vec![
                CpsInstr::LoadConst(0, 0),
                CpsInstr::UnOp(1, CpsUnOp::NegInt, 0),
            ],
            CpsTerminator::Return(1),
            vec![Constant::Int(42)],
            2,
        );
        let mut vm = VM::new();
        vm.load(&m).unwrap();
        assert_eq!(vm.execute(0, 2).unwrap(), -42);
    }

    #[test]
    fn test_not() {
        let m = simple_mod(
            vec![
                CpsInstr::LoadConst(0, 0),
                CpsInstr::UnOp(1, CpsUnOp::Not, 0),
            ],
            CpsTerminator::Return(1),
            vec![Constant::Int(0)],
            2,
        );
        let mut vm = VM::new();
        vm.load(&m).unwrap();
        assert_eq!(vm.execute(0, 2).unwrap(), 1); // !0 = true = 1
    }

    #[test]
    fn test_printfn() {
        let m = simple_mod(
            vec![CpsInstr::LoadConst(0, 0), CpsInstr::Print(0)],
            CpsTerminator::Return(0),
            vec![Constant::String("hi".into())],
            1,
        );
        let mut vm = VM::new();
        vm.load(&m).unwrap();
        vm.execute(0, 1).unwrap();
        assert!(vm.output.len() > 0, "output should have print result");
    }

    #[test]
    fn test_heap_string() {
        let m = simple_mod(
            vec![CpsInstr::LoadConst(0, 0)],
            CpsTerminator::Return(0),
            vec![Constant::String("hello".into())],
            1,
        );
        let mut vm = VM::new();
        vm.load(&m).unwrap();
        let r = vm.execute(0, 1).unwrap();
        if let HeapObj::String(s) = vm.heap_get(r) {
            assert_eq!(s, "hello");
        } else {
            panic!("expected string");
        }
    }

    fn simple_mod_with_structs(
        instrs: Vec<CpsInstr>,
        term: CpsTerminator,
        consts: Vec<Constant>,
        structs: Vec<StructDef>,
        reg_count: usize,
    ) -> CpsModule {
        CpsModule {
            functions: vec![CpsFunction {
                name: "main".into(),
                blocks: vec![CpsBlock {
                    id: 0,
                    params: vec![],
                    instrs,
                    term,
                }],
                entry: 0,
                reg_count,
            }],
            constants: consts,
            structs,
        }
    }

    #[test]
    fn t16_struct_set_get_field_retains_rc() {
        // struct Foo { x: String } → bitmap = 0b01 (field 0 is heap type)
        let structs = vec![StructDef {
            id: 0,
            name: "Foo".into(),
            fields: vec![("x".into(), "String".into())],
            type_bitmap: 0b01,
        }];
        // Alloc string at slot 0, alloc struct Foo at slot 1
        // SetField(value=r0, obj=r1, field=0) → set field 0 of struct to string
        let cps = simple_mod_with_structs(
            vec![
                CpsInstr::LoadConst(0, 0), // r0 = "hello" string (heap slot 0, rc=1)
                CpsInstr::NewStruct(1, 0, vec![]), // r1 = struct Foo (heap slot 1, rc=1)
                CpsInstr::SetField(0, 1, 0, 0), // struct[r1].field[0] = r0 → retains r0
            ],
            CpsTerminator::Return(0), // return string ref
            vec![Constant::String("hello".into())],
            structs,
            2,
        );
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        let result = vm.execute(0, 2).unwrap();
        // r0 (string) should have rc=2: one from LoadConst, one from SetField retain
        assert_eq!(vm.heap.ref_count(result as usize), 2);
        assert_eq!(vm.heap.ref_count(0), 2);
        // struct at slot 1 should have rc=1
        assert_eq!(vm.heap.ref_count(1), 1);
    }

    #[test]
    fn t17_setfield_overwrite_releases_old() {
        // struct Foo { x: String }
        let structs = vec![StructDef {
            id: 0,
            name: "Foo".into(),
            fields: vec![("x".into(), "String".into())],
            type_bitmap: 0b01,
        }];
        // 1. Load "hello" → r0 (slot 0)
        // 2. Load "world" → r1 (slot 1)
        // 3. NewStruct → r2 (slot 2)
        // 4. SetField(value=r0, obj=r2, field=0) → slot0 rc 1→2
        // 5. SetField(value=r1, obj=r2, field=0) → slot0 released (rc 2→1), slot1 retained (rc 1→2)
        let cps = simple_mod_with_structs(
            vec![
                CpsInstr::LoadConst(0, 0),         // r0 = "hello"
                CpsInstr::LoadConst(1, 1),         // r1 = "world"
                CpsInstr::NewStruct(2, 0, vec![]), // r2 = struct
                CpsInstr::SetField(0, 2, 0, 0),    // struct.field0 = "hello"
                CpsInstr::SetField(1, 2, 0, 0),    // struct.field0 = "world" (overwrites)
            ],
            CpsTerminator::Return(2), // return struct
            vec![
                Constant::String("hello".into()),
                Constant::String("world".into()),
            ],
            structs,
            3,
        );
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        let _ = vm.execute(0, 3).unwrap();
        // "hello" (slot 0) was released by overwrite → rc should be 1 (only LoadConst reference left)
        assert_eq!(
            vm.heap.ref_count(0),
            1,
            "old string should be released back to rc=1"
        );
        // "world" (slot 1) was retained by SetField → rc should be 2
        assert_eq!(
            vm.heap.ref_count(1),
            2,
            "new string should be retained to rc=2"
        );
        // struct (slot 2) → rc=1
        assert_eq!(vm.heap.ref_count(2), 1);
    }

    #[test]
    fn t18_setfield_non_heap_does_nothing() {
        // struct Bar { n: Int64 } → bitmap = 0 (no heap fields)
        let structs = vec![StructDef {
            id: 0,
            name: "Bar".into(),
            fields: vec![("n".into(), "Int64".into())],
            type_bitmap: 0,
        }];
        let cps = simple_mod_with_structs(
            vec![
                CpsInstr::LoadConst(0, 0),         // r0 = 42 (int constant)
                CpsInstr::NewStruct(1, 0, vec![]), // r1 = struct
                CpsInstr::SetField(0, 1, 0, 0),    // struct.field0 = 42
            ],
            CpsTerminator::Return(1),
            vec![Constant::Int(42)],
            structs,
            2,
        );
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        vm.execute(0, 2).unwrap();
        // No heap objects manipulated by SetField; struct at slot 0 should have rc=1
        assert_eq!(vm.heap.ref_count(0), 1);
    }

    #[test]
    fn t19_many_allocs_no_panic() {
        let structs = vec![StructDef {
            id: 0,
            name: "Node".into(),
            fields: vec![("val".into(), "String".into())],
            type_bitmap: 0b01,
        }];
        let mut instrs = vec![];
        let mut consts: Vec<Constant> = vec![];
        for i in 0usize..20 {
            consts.push(Constant::String(format!("s{}", i)));
            instrs.push(CpsInstr::LoadConst(i, i));
        }
        for i in 0usize..20 {
            let si = 20 + i;
            instrs.push(CpsInstr::NewStruct(si, 0, vec![]));
            instrs.push(CpsInstr::SetField(i, si, 0, 0));
        }
        for i in 0usize..19 {
            let si = 20 + i;
            instrs.push(CpsInstr::SetField(i + 1, si, 0, 0));
        }
        let cps = simple_mod_with_structs(instrs, CpsTerminator::Return(39), consts, structs, 40);
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        let _ = vm.execute(0, 40).unwrap();
    }

    #[test]
    fn test_itos() {
        // IToS: convert i64 to heap string
        let cps = simple_mod(
            vec![
                CpsInstr::LoadConst(0, 0),                // r0 = 42
                CpsInstr::BinOp(1, CpsBinOp::IToS, 0, 0), // r1 = itos(r0)
            ],
            CpsTerminator::Return(1),
            vec![Constant::Int(42)],
            2,
        );
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        let r = vm.execute(0, 2).unwrap();
        if let HeapObj::String(s) = vm.heap_get(r) {
            assert_eq!(s, "42");
        } else {
            panic!("expected heap string, got idx {}", r);
        }
    }

    #[test]
    fn itos_ignores_float_register_residue() {
        let cps = simple_mod(
            vec![
                CpsInstr::LoadConst(0, 0),
                CpsInstr::BinOp(1, CpsBinOp::IToF, 0, 0),
                CpsInstr::Move(2, 0),
                CpsInstr::BinOp(3, CpsBinOp::IToS, 2, 0),
            ],
            CpsTerminator::Return(3),
            vec![Constant::Int(200)],
            4,
        );
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        let r = vm.execute(0, 4).unwrap();
        if let HeapObj::String(s) = vm.heap_get(r) {
            assert_eq!(s, "200");
        } else {
            panic!("expected heap string, got idx {}", r);
        }
    }

    #[test]
    fn test_ftos() {
        let cps = simple_mod(
            vec![
                CpsInstr::LoadConst(0, 0),                // r0 = 3.14
                CpsInstr::BinOp(1, CpsBinOp::FToS, 0, 0), // r1 = ftos(r0)
            ],
            CpsTerminator::Return(1),
            vec![Constant::Float(3.14)],
            2,
        );
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        let r = vm.execute(0, 2).unwrap();
        if let HeapObj::String(s) = vm.heap_get(r) {
            assert_eq!(s, "3.14");
        } else {
            panic!("expected heap string");
        }
    }

    #[test]
    fn test_float_ops_and_comparisons() {
        let cps = simple_mod(
            vec![
                CpsInstr::LoadConst(0, 0),
                CpsInstr::LoadConst(1, 1),
                CpsInstr::BinOp(2, CpsBinOp::FAdd, 0, 1),
                CpsInstr::BinOp(3, CpsBinOp::FSub, 2, 1),
                CpsInstr::BinOp(4, CpsBinOp::FMul, 3, 1),
                CpsInstr::BinOp(5, CpsBinOp::FDiv, 4, 1),
                CpsInstr::BinOp(6, CpsBinOp::FEq, 5, 5),
                CpsInstr::BinOp(7, CpsBinOp::FLt, 0, 1),
                CpsInstr::BinOp(8, CpsBinOp::FToI, 6, 0),
                CpsInstr::BinOp(9, CpsBinOp::FToI, 7, 0),
                CpsInstr::BinOp(10, CpsBinOp::AddInt, 8, 9),
            ],
            CpsTerminator::Return(10),
            vec![Constant::Float(1.5), Constant::Float(2.0)],
            11,
        );
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        assert_eq!(vm.execute(0, 11).unwrap(), 2);
    }

    #[test]
    fn test_index_out_of_bounds() {
        let cps = simple_mod(
            vec![
                CpsInstr::LoadConst(0, 0),
                CpsInstr::NewList(1, vec![]),
                CpsInstr::IndexGet(2, 1, 0),
            ],
            CpsTerminator::Return(2),
            vec![Constant::Int(99)],
            3,
        );
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        assert!(matches!(
            vm.execute(0, 3),
            Err(RuntimeError::IndexOutOfBounds(_, _))
        ));
    }

    #[test]
    fn test_native_call_and_suspend_paths() {
        let native = CpsModule {
            functions: vec![CpsFunction {
                name: "main".into(),
                blocks: vec![
                    CpsBlock {
                        id: 0,
                        params: vec![],
                        instrs: vec![CpsInstr::LoadConst(0, 0)],
                        term: CpsTerminator::CallNative(0, vec![0], 1),
                    },
                    CpsBlock {
                        id: 1,
                        params: vec![],
                        instrs: vec![],
                        term: CpsTerminator::Return(0),
                    },
                ],
                entry: 0,
                reg_count: 1,
            }],
            constants: vec![Constant::Int(7)],
            structs: vec![],
        };
        let mut vm = VM::new();
        vm.load(&native).unwrap();
        assert_eq!(vm.execute(0, 1).unwrap(), 7);

        let suspended = simple_mod(vec![], CpsTerminator::Suspend, vec![], 1);
        let mut vm = VM::new();
        vm.load(&suspended).unwrap();
        assert_eq!(vm.execute(0, 1).unwrap(), 0);
        assert!(vm.scheduler.has_pending());
        let result = vm.scheduler.flush_all(9);
        assert_eq!(result.len(), 1);
        assert!(vm.scheduler.poll().is_some());
    }

    fn two_func_mod(
        lambda_instrs: Vec<CpsInstr>,
        lambda_term: CpsTerminator,
        lambda_regs: usize,
    ) -> CpsModule {
        // main: calls lambda (func 0), reads result from reg 0
        // lambda: executes lambda_instrs, ends with lambda_term
        let main = CpsFunction {
            name: "main".into(),
            blocks: vec![
                CpsBlock {
                    id: 0,
                    params: vec![],
                    instrs: vec![],
                    term: CpsTerminator::Call(0, vec![], 1),
                },
                CpsBlock {
                    id: 1,
                    params: vec![],
                    instrs: vec![],
                    term: CpsTerminator::Return(0),
                },
            ],
            entry: 0,
            reg_count: 2,
        };
        let lambda = CpsFunction {
            name: "lambda".into(),
            blocks: vec![CpsBlock {
                id: 0,
                params: vec![],
                instrs: lambda_instrs,
                term: lambda_term,
            }],
            entry: 0,
            reg_count: lambda_regs,
        };
        CpsModule {
            functions: vec![lambda, main], // lambda first (func_idx=0), main last (entry)
            constants: vec![Constant::Int(42), Constant::String("hi".into())],
            structs: vec![],
        }
    }

    #[test]
    fn vm_call_var_return() {
        let cps = two_func_mod(
            vec![CpsInstr::LoadConst(1, 0)], // LoadConst(r1, const_int(42))
            CpsTerminator::Return(1),
            2,
        );
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        let result = vm.execute(1, 2).unwrap();
        assert_eq!(result, 42, "lambda should return 42");
    }

    #[test]
    fn vm_call_print_inside() {
        let cps = two_func_mod(
            vec![CpsInstr::LoadConst(0, 1), CpsInstr::Print(0)], // LoadConst(r0, "hi"), Print(r0)
            CpsTerminator::Return(0),
            1,
        );
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        vm.execute(1, 2).unwrap();
        assert!(
            vm.output.iter().any(|s| s.contains("hi")),
            "print inside lambda: output={:?}",
            vm.output
        );
    }
}
