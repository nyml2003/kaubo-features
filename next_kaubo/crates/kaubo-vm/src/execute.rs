//! 寄存器 VM — 完整实现
//! 7-bit opcode, CPS block scheduler, 调用栈 + 闭包 + stdlib

use crate::async_runtime::AsyncScheduler;
use crate::gc_heap::GcHeap;
use crate::regfile::*;
use crate::stdlib;
use kaubo_cps::*;
use kaubo_log::emit;
use kaubo_log::EventHandler;
use std::collections::HashMap;

// ── 编码 ──
pub fn encode(op: u8, dst: u32, src1: u32, src2: u32) -> u32 {
    ((op as u32) << 25) | ((dst & 0xFF) << 17) | ((src1 & 0x1FF) << 8) | (src2 & 0xFF)
}

// ── Opcode 枚举 ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    // 整数算术
    AddInt = 0x00,
    SubInt = 0x01,
    MulInt = 0x02,
    DivInt = 0x03,
    ModInt = 0x04,
    NegInt = 0x05,
    // 浮点
    FAdd = 0x08,
    FSub = 0x09,
    FMul = 0x0A,
    FDiv = 0x0B,
    FNeg = 0x0C,
    // 比较
    EqInt = 0x10,
    LtInt = 0x11,
    LeInt = 0x12,
    FEq = 0x13,
    FLt = 0x14,
    Not = 0x15,
    NeInt = 0x16,
    GtInt = 0x17,
    SAdd = 0x18,
    GeInt = 0x19,
    FNe = 0x1A,
    FLe = 0x1B,
    FGt = 0x1C,
    FGe = 0x1D,
    // 转换
    IToF = 0x20,
    FToI = 0x21,
    IToS = 0x22,
    FToS = 0x23,
    SToI = 0x24,
    BToS = 0x25,
    // 数据移动
    Move = 0x30,
    LoadImm = 0x31,
    LoadConst = 0x32,
    // 堆分配
    ListLen = 0x33,
    NewStruct = 0x34,
    NewList = 0x35,
    // 字段/索引
    GetField = 0x36,
    SetField = 0x37,
    IndexGet = 0x38,
    IndexSet = 0x39,
    // 装箱
    Box_ = 0x3A,
    Unbox = 0x3B,
    // Enum/Variant
    NewVariant = 0x3C,
    GetVariantTag = 0x3D,
    GetVariantField = 0x3E,
    SetVariantField = 0x3F,
    // 控制流
    Jump = 0x40,
    Branch = 0x41,
    // 调用
    Call = 0x50,
    TailCall = 0x51,
    Return = 0x52,
    CallIndirect = 0x53,
    LoadVtable = 0x54,
    NewInterfaceObj = 0x55,
    CallNative = 0x5F,
    // 元组
    NewTuple = 0x56,
    TupleIndex = 0x57,
    // TypedArray
    NewInt64Array = 0x58,
    NewFloat64Array = 0x59,
    // Async
    AsyncPoll = 0x60,
    Suspend = 0x61,
    // I/O
    Print = 0x7F,
}

impl Opcode {
    #[inline(always)]
    pub fn from_inst(inst: u32) -> Self {
        unsafe { std::mem::transmute(((inst >> 25) & 0x7F) as u8) }
    }
}

// ── 指令解码 ──

#[derive(Debug, Clone, Copy)]
pub struct Inst(pub u32);

impl Inst {
    #[inline(always)]
    pub fn opcode(self) -> Opcode {
        Opcode::from_inst(self.0)
    }

    #[inline(always)]
    pub fn dst(self) -> usize {
        ((self.0 >> 17) & 0xFF) as usize
    }

    #[inline(always)]
    pub fn src1(self) -> usize {
        ((self.0 >> 8) & 0x1FF) as usize
    }

    #[inline(always)]
    pub fn src2(self) -> usize {
        (self.0 & 0xFF) as usize
    }

    #[inline(always)]
    pub fn imm25(self) -> usize {
        (self.0 & 0x1FF_FFFF) as usize
    }

    /// 17-bit 立即数 (bits 0–16), 用于 Call/CallNative cont_block
    #[inline(always)]
    pub fn imm17(self) -> usize {
        (self.0 & 0x1FFFF) as usize
    }

    #[inline(always)]
    pub fn abc(self) -> (usize, usize, usize) {
        (self.dst(), self.src1(), self.src2())
    }
}

// ── 运行时错误 ──
#[derive(Debug)]
pub enum RuntimeError {
    DivisionByZero,
    IndexOutOfBounds(i64, usize),
    FieldOutOfBounds { index: usize, len: usize },
    InvalidHeapHandle(i64),
    TypeMismatch(String),
    UnsupportedInstruction(String),
    InvalidOpcode(u8),
    NativeError(String),
    NullAccess,
    TypeAssertion(String),
    StackOverflow,
    LoopExceeded { block_id: usize, limit: u64 },
    Bug(String),
}

// ── 堆对象 ──
#[derive(Debug, Clone)]
pub enum HeapObj {
    String(String),
    List(Vec<i64>),
    Struct(usize, Vec<i64>),       // (struct_id, field_values)
    Variant(usize, u16, Vec<i64>), // (enum_id, tag, field_values)
    InterfaceObj { vtable_idx: usize, data: i64 },
    Closure(Box<ClosureObj>),
    TupleObj(Vec<usize>),
    Int64Array(Vec<i64>),
    Float64Array(Vec<f64>),
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

    /// Maximum allowed loop iterations per (func_idx, block_id) before
    /// raising `LoopExceeded`.  Default: `u64::MAX` (no practical limit).
    /// Set lower via `--max-loop-iterations` for playground / sandbox use.
    pub max_loop_iterations: u64,
    /// Per-block loop iteration counters.  Key: `(func_idx, block_id)`.
    /// Cleared on every `load()` call for execution isolation.
    loop_iter_counts: HashMap<(usize, usize), u64>,

    pub heap: super::gc_heap::GcHeap,
    pub struct_bitmaps: Vec<u64>,
    pub struct_field_counts: Vec<usize>,
    pub enum_variant_bitmaps: Vec<Vec<u64>>,
    pub enum_variant_counts: Vec<Vec<usize>>,
    pub vtables: Vec<VtableDef>,
    pub natives: Vec<(&'static str, stdlib::NativeFn)>,
    pub scheduler: AsyncScheduler,
}

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub func_idx: usize,
    pub ret_block: usize,
    pub saved_regs: Vec<u64>,
    pub result_reg: usize,
}

const MAX_CALL_DEPTH: usize = 1024;

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

impl VM {
    pub fn new() -> Self {
        VM {
            regs: RegFile::new(512),
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
            max_loop_iterations: u64::MAX,
            loop_iter_counts: HashMap::new(),
            heap: GcHeap::new(),
            struct_bitmaps: vec![],
            struct_field_counts: vec![],
            enum_variant_bitmaps: vec![],
            enum_variant_counts: vec![],
            vtables: vec![],
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
        self.loop_iter_counts.clear();

        for sd in &module.structs {
            let id = sd.id;
            if id >= self.struct_bitmaps.len() {
                self.struct_bitmaps.resize(id + 1, 0);
                self.struct_field_counts.resize(id + 1, 0);
            }
            self.struct_bitmaps[id] = sd.type_bitmap;
            self.struct_field_counts[id] = sd.fields.len();
        }

        for ed in &module.enums {
            let id = ed.id;
            if id >= self.enum_variant_counts.len() {
                self.enum_variant_counts.resize(id + 1, vec![]);
                self.enum_variant_bitmaps.resize(id + 1, vec![]);
            }
            let counts: Vec<usize> = ed.variants.iter().map(|(_, _, f)| f.len()).collect();
            let bitmaps: Vec<u64> = ed.variant_type_bitmaps.clone();
            self.enum_variant_counts[id] = counts;
            self.enum_variant_bitmaps[id] = bitmaps;
        }

        self.vtables = module.vtables.clone();

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
                    CpsTerminator::Branch(_, _, ta, _, fa) => {
                        // Store both arg sets: [true_count, true_args..., false_count, false_args...]
                        let mut v = vec![ta.len()];
                        v.extend_from_slice(ta);
                        v.push(fa.len());
                        v.extend_from_slice(fa);
                        v
                    }
                    CpsTerminator::Call(_, a, _) => a.clone(),
                    CpsTerminator::CallNative(_, a, _) => a.clone(),
                    CpsTerminator::CallIndirect(_, a, _) => a.clone(),
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

    fn block_id_from_ip(&self, ip: usize) -> usize {
        let base = self.func_block_base[self.current_func];
        let func_blocks = &self.func_blocks[self.current_func];
        let starts = &self.block_starts[base..base + func_blocks.len()];
        for id in (0..starts.len()).rev() {
            // Skip inlined blocks (start=0) — entry block always has non-zero start
            if starts[id] > 0 && starts[id] <= ip {
                return id;
            }
        }
        0
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
                self.regs.regs[params[i]] = self.regs.regs[arg_reg];
            }
        }
    }

    /// Check and update the loop iteration counter for a backward jump.
    ///
    /// Returns `Err(LoopExceeded)` if the per-block iteration count exceeds
    /// `max_loop_iterations`.  Emits `LoopIteration` and `LoopNearLimit` events.
    fn check_loop_iteration(
        &mut self,
        block_id: usize,
        events: Option<&dyn EventHandler>,
    ) -> Result<(), RuntimeError> {
        let key = (self.current_func, block_id);
        let count = self.loop_iter_counts.entry(key).or_insert(0);
        *count += 1;

        emit!(
            events,
            kaubo_log::ToolchainEvent::Vm(kaubo_log::VmEvent::LoopIteration {
                func_idx: self.current_func,
                block_id: block_id,
                count: *count,
            })
        );

        if *count > self.max_loop_iterations {
            return Err(RuntimeError::LoopExceeded {
                block_id,
                limit: self.max_loop_iterations,
            });
        }

        if *count >= self.max_loop_iterations.saturating_mul(8) / 10 {
            emit!(
                events,
                kaubo_log::ToolchainEvent::Vm(kaubo_log::VmEvent::LoopNearLimit {
                    func_idx: self.current_func,
                    block_id: block_id,
                    count: *count,
                    limit: self.max_loop_iterations,
                })
            );
        }

        Ok(())
    }

    fn write_int(&mut self, reg: usize, value: i64) {
        self.regs.regs[reg] = value as u64;
    }

    fn write_bool(&mut self, reg: usize, value: bool) {
        self.regs.regs[reg] = if value { 1 } else { 0 };
    }

    fn write_float(&mut self, reg: usize, value: f64) {
        self.regs.regs[reg] = value.to_bits();
    }

    fn write_heap(&mut self, reg: usize, obj: HeapObj) {
        let hid = self.alloc_heap(obj);
        self.write_int(reg, hid);
    }

    fn alloc_heap(&mut self, obj: HeapObj) -> i64 {
        self.heap.alloc(obj) as i64
    }

    fn heap_get(&self, id: i64) -> Result<&HeapObj, RuntimeError> {
        if id < 0 {
            return Err(RuntimeError::InvalidHeapHandle(id));
        }
        self.heap
            .try_get(id as usize)
            .ok_or(RuntimeError::InvalidHeapHandle(id))
    }

    fn heap_get_mut(&mut self, id: i64) -> Result<&mut HeapObj, RuntimeError> {
        if id < 0 || self.heap.try_get(id as usize).is_none() {
            return Err(RuntimeError::InvalidHeapHandle(id));
        }
        Ok(self.heap.get_mut(id as usize))
    }

    pub fn execute(
        &mut self,
        entry_func: usize,
        _reg_count: usize,
        events: Option<&dyn kaubo_log::EventHandler>,
    ) -> Result<i64, RuntimeError> {
        self.current_func = entry_func;
        let reg_needed = self.func_reg_counts[entry_func];
        self.regs.ensure_capacity(reg_needed);
        if self.regs.regs.len() < reg_needed {
            self.regs.regs.resize(reg_needed, 0);
        }

        let mut ip = self.func_entries[entry_func];

        loop {
            let inst = Inst(self.instrs[ip]);
            ip += 1;

            emit!(
                events,
                kaubo_log::ToolchainEvent::Vm(kaubo_log::VmEvent::Instruction {
                    func: self.current_func,
                    ip: ip - 1,
                    opcode: (inst.0 >> 25) as u8,
                    inst: inst.0,
                })
            );

            let opcode = inst.opcode();
            match opcode {
                // ── 整数算术 ──
                Opcode::AddInt => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    self.write_int(
                        a,
                        (self.regs.regs[b] as i64).wrapping_add(self.regs.regs[c] as i64),
                    );
                }
                Opcode::SubInt => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    self.write_int(
                        a,
                        (self.regs.regs[b] as i64).wrapping_sub(self.regs.regs[c] as i64),
                    );
                }
                Opcode::MulInt => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    self.write_int(
                        a,
                        (self.regs.regs[b] as i64).wrapping_mul(self.regs.regs[c] as i64),
                    );
                }
                Opcode::DivInt => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    if self.regs.regs[c] == 0 {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    self.write_int(
                        a,
                        (self.regs.regs[b] as i64).wrapping_div(self.regs.regs[c] as i64),
                    );
                }
                Opcode::ModInt => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    if self.regs.regs[c] == 0 {
                        return Err(RuntimeError::DivisionByZero);
                    }
                    self.write_int(
                        a,
                        (self.regs.regs[b] as i64).wrapping_rem(self.regs.regs[c] as i64),
                    );
                }
                Opcode::NegInt => {
                    let a = inst.dst();
                    let b = inst.src1();
                    self.write_int(a, (self.regs.regs[b] as i64).wrapping_neg());
                }

                // ── 浮点 ──
                Opcode::FAdd => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    let fb = f64::from_bits(self.regs.regs[b]);
                    let fc = f64::from_bits(self.regs.regs[c]);
                    self.write_float(a, fb + fc);
                }
                Opcode::FSub => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    let fb = f64::from_bits(self.regs.regs[b]);
                    let fc = f64::from_bits(self.regs.regs[c]);
                    self.write_float(a, fb - fc);
                }
                Opcode::FMul => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    let fb = f64::from_bits(self.regs.regs[b]);
                    let fc = f64::from_bits(self.regs.regs[c]);
                    self.write_float(a, fb * fc);
                }
                Opcode::FDiv => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    let fb = f64::from_bits(self.regs.regs[b]);
                    let fc = f64::from_bits(self.regs.regs[c]);
                    self.write_float(a, fb / fc);
                }
                Opcode::FNeg => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let fb = f64::from_bits(self.regs.regs[b]);
                    self.write_float(a, -fb);
                }

                // ── 比较 ──
                Opcode::EqInt => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    self.write_bool(a, (self.regs.regs[b] as i64) == self.regs.regs[c] as i64);
                }
                Opcode::LtInt => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    self.write_bool(a, (self.regs.regs[b] as i64) < self.regs.regs[c] as i64);
                }
                Opcode::LeInt => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    self.write_bool(a, (self.regs.regs[b] as i64) <= self.regs.regs[c] as i64);
                }
                Opcode::FEq => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    self.write_bool(
                        a,
                        f64::from_bits(self.regs.regs[b]) == f64::from_bits(self.regs.regs[c]),
                    );
                }
                Opcode::FLt => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    self.write_bool(
                        a,
                        f64::from_bits(self.regs.regs[b]) < f64::from_bits(self.regs.regs[c]),
                    );
                }

                // ── Not ──
                Opcode::Not => {
                    let a = inst.dst();
                    let b = inst.src1();
                    self.write_bool(a, self.regs.regs[b] == 0);
                }
                Opcode::NeInt => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    self.write_bool(a, self.regs.regs[b] as i64 != self.regs.regs[c] as i64);
                }

                Opcode::GtInt => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    self.write_bool(a, (self.regs.regs[b] as i64) > self.regs.regs[c] as i64);
                }

                // ── 字符串拼接 ──
                Opcode::SAdd => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    let lhs = self.heap_get(self.regs.regs[b] as i64)?.clone();
                    let rhs = self.heap_get(self.regs.regs[c] as i64)?.clone();
                    let result = match (lhs, rhs) {
                        (HeapObj::String(l), HeapObj::String(r)) => HeapObj::String(l + &r),
                        _ => {
                            return Err(RuntimeError::TypeMismatch(
                                "SAdd requires two string operands".into(),
                            ))
                        }
                    };
                    self.write_heap(a, result);
                }
                Opcode::GeInt => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    self.write_bool(a, self.regs.regs[b] as i64 >= self.regs.regs[c] as i64);
                }
                Opcode::FNe => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    self.write_bool(
                        a,
                        f64::from_bits(self.regs.regs[b]) != f64::from_bits(self.regs.regs[c]),
                    );
                }
                Opcode::FLe => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    self.write_bool(
                        a,
                        f64::from_bits(self.regs.regs[b]) <= f64::from_bits(self.regs.regs[c]),
                    );
                }
                Opcode::FGt => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    self.write_bool(
                        a,
                        f64::from_bits(self.regs.regs[b]) > f64::from_bits(self.regs.regs[c]),
                    );
                }
                Opcode::FGe => {
                    let a = inst.dst();
                    let b = inst.src1();
                    let c = inst.src2();
                    self.write_bool(
                        a,
                        f64::from_bits(self.regs.regs[b]) >= f64::from_bits(self.regs.regs[c]),
                    );
                }

                // ── 转换 ──
                Opcode::IToF => {
                    let d = inst.dst();
                    let s = inst.src1();
                    self.write_float(d, self.regs.regs[s] as i64 as f64);
                }
                Opcode::FToI => {
                    let d = inst.dst();
                    let s = inst.src1();
                    let f = f64::from_bits(self.regs.regs[s]);
                    self.write_int(d, f as i64);
                }
                Opcode::IToS => {
                    // itos
                    let d = inst.dst();
                    let s = inst.src1();
                    let st = format!("{}", self.regs.regs[s] as i64);
                    self.write_heap(d, HeapObj::String(st));
                }
                Opcode::BToS => {
                    // btos
                    let d = inst.dst();
                    let s = inst.src1();
                    let st = if self.regs.regs[s] as i64 != 0 { "true" } else { "false" };
                    self.write_heap(d, HeapObj::String(st.to_string()));
                }
                Opcode::FToS => {
                    // ftos
                    let d = inst.dst();
                    let s = inst.src1();
                    let st = format!("{}", f64::from_bits(self.regs.regs[s]));
                    self.write_heap(d, HeapObj::String(st));
                }
                Opcode::SToI => {
                    // stoi
                    let d = inst.dst();
                    let s = inst.src1();
                    let hid = self.regs.regs[s] as i64;
                    if hid < 0 {
                        return Err(RuntimeError::TypeMismatch(
                            "SToI: expected string heap handle".into(),
                        ));
                    }
                    match self.heap.try_get(hid as usize) {
                        Some(HeapObj::String(st)) => {
                            let val: i64 = st.parse().map_err(|_| {
                                RuntimeError::TypeMismatch(format!(
                                    "SToI: cannot parse '{st}' as integer"
                                ))
                            })?;
                            self.write_int(d, val);
                        }
                        _ => {
                            return Err(RuntimeError::TypeMismatch(
                                "SToI: expected string argument".into(),
                            ));
                        }
                    }
                }

                // ── 数据移动 ──
                Opcode::Move => {
                    let d = inst.dst();
                    let s = inst.src1();
                    self.regs.regs[d] = self.regs.regs[s];
                }
                Opcode::LoadImm => {
                    let d = inst.dst();
                    self.write_int(d, inst.imm17() as i64);
                }
                Opcode::LoadConst => {
                    let d = inst.dst();
                    let idx = inst.src1();
                    let constant = self
                        .consts
                        .get(idx)
                        .cloned()
                        .ok_or_else(|| RuntimeError::Bug(format!("constant index {idx}")))?;
                    match constant {
                        Constant::Int(n) => self.write_int(d, n),
                        Constant::Float(f) => self.write_float(d, f),
                        Constant::String(s) => self.write_heap(d, HeapObj::String(s)),
                        Constant::Bool(b) => self.write_bool(d, b),
                        Constant::Null => self.write_int(d, 0),
                    }
                }

                // ── 堆分配 ──
                Opcode::NewStruct => {
                    let d = inst.dst();
                    let sid = inst.src1();
                    let nf = self
                        .struct_field_counts
                        .get(sid)
                        .copied()
                        .ok_or_else(|| RuntimeError::Bug(format!("unknown struct id {sid}")))?;
                    let bitmap = self.struct_bitmaps[sid];
                    let mut fields = vec![0i64; nf];
                    for (i, f) in fields.iter_mut().enumerate() {
                        if (bitmap >> i) & 1 != 0 {
                            *f = -1; // null sentinel for heap-type fields
                        }
                    }
                    self.write_heap(d, HeapObj::Struct(sid, fields));
                }
                Opcode::NewList => {
                    // NewList(dst, count, _) — element regs read from block params
                    let d = inst.dst();
                    let count = inst.src1();
                    let block_id = self.block_id_from_ip(ip);
                    let params = &self.func_params[self.current_func][block_id];
                    let mut elements: Vec<i64> = Vec::with_capacity(count);
                    for i in 0..count {
                        let val: i64 = if i < params.len() {
                            self.regs.regs[params[i]] as i64
                        } else {
                            0
                        };
                        elements.push(val);
                    }
                    self.write_heap(d, HeapObj::List(elements));
                }
                Opcode::NewTuple => {
                    let d = inst.dst();
                    let count = inst.src1();
                    let block_id = self.block_id_from_ip(ip);
                    let params = &self.func_params[self.current_func][block_id];
                    let mut elements: Vec<usize> = Vec::with_capacity(count);
                    for i in 0..count {
                        let val = if i < params.len() {
                            self.regs.regs[params[i]] as usize
                        } else {
                            0
                        };
                        elements.push(val);
                    }
                    let hid = self.heap.alloc(HeapObj::TupleObj(elements));
                    self.regs.regs[d] = hid as u64;
                }
                Opcode::TupleIndex => {
                    let d = inst.dst();
                    let tuple_reg = inst.src1();
                    let index = inst.src2();
                    let hid = self.regs.regs[tuple_reg] as i64;
                    let val = match self.heap_get(hid)? {
                        HeapObj::TupleObj(elements) => elements[index],
                        other => {
                            return Err(RuntimeError::TypeMismatch(format!(
                                "expected Tuple, got {other:?}"
                            )))
                        }
                    };
                    self.regs.regs[d] = val as u64;
                }
                Opcode::NewInt64Array => {
                    let d = inst.dst();
                    let count = inst.src1();
                    let block_id = self.block_id_from_ip(ip);
                    let params = &self.func_params[self.current_func][block_id];
                    let mut elements: Vec<i64> = Vec::with_capacity(count);
                    for i in 0..count {
                        let val: i64 = if i < params.len() {
                            self.regs.regs[params[i]] as i64
                        } else {
                            0
                        };
                        elements.push(val);
                    }
                    let hid = self.heap.alloc(HeapObj::Int64Array(elements));
                    self.regs.regs[d] = hid as u64;
                }
                Opcode::NewFloat64Array => {
                    let d = inst.dst();
                    let count = inst.src1();
                    let block_id = self.block_id_from_ip(ip);
                    let params = &self.func_params[self.current_func][block_id];
                    let mut elements: Vec<f64> = Vec::with_capacity(count);
                    for i in 0..count {
                        let val: f64 = if i < params.len() {
                            f64::from_bits(self.regs.regs[params[i]])
                        } else {
                            0.0
                        };
                        elements.push(val);
                    }
                    let hid = self.heap.alloc(HeapObj::Float64Array(elements));
                    self.regs.regs[d] = hid as u64;
                }
                Opcode::ListLen => {
                    // ListLen(dst, obj) — return list length
                    let d = inst.dst();
                    let obj = inst.src1();
                    let hid = self.regs.regs[obj] as i64;
                    let len = match self.heap_get(hid)? {
                        HeapObj::List(v) => v.len() as i64,
                        HeapObj::Int64Array(v) => v.len() as i64,
                        HeapObj::Float64Array(v) => v.len() as i64,
                        other => {
                            return Err(RuntimeError::TypeMismatch(format!(
                                "ListLen: expected indexable, got {other:?}"
                            )));
                        }
                    };
                    self.write_int(d, len);
                }

                // ── 字段访问 ──
                Opcode::GetField => {
                    // GetField(dst, src, idx)
                    let d = inst.dst();
                    let s = inst.src1();
                    let idx = inst.src2();
                    let hid = self.regs.regs[s] as i64;
                    let val = match self.heap_get(hid)? {
                        HeapObj::Struct(_, fields) => {
                            fields
                                .get(idx)
                                .copied()
                                .ok_or(RuntimeError::FieldOutOfBounds {
                                    index: idx,
                                    len: fields.len(),
                                })?
                        }
                        other => {
                            return Err(RuntimeError::TypeMismatch(format!(
                                "GetField expected struct, got {other:?}"
                            )))
                        }
                    };
                    self.write_int(d, val);
                }
                Opcode::SetField => {
                    // SetField(dst, src, idx, val)
                    let d = inst.dst();
                    let s = inst.src1();
                    let idx = inst.src2();
                    let hid = self.regs.regs[s] as i64;
                    let val = self.regs.regs[d] as i64;

                    // Read struct_id and old field value
                    let (sid, old_val, len) =
                        match self.heap_get(hid)? {
                            HeapObj::Struct(sid, fields) => {
                                let old_val = fields.get(idx).copied().ok_or(
                                    RuntimeError::FieldOutOfBounds {
                                        index: idx,
                                        len: fields.len(),
                                    },
                                )?;
                                (*sid, old_val, fields.len())
                            }
                            other => {
                                return Err(RuntimeError::TypeMismatch(format!(
                                    "SetField expected struct, got {other:?}"
                                )))
                            }
                        };
                    // Check if this field is a heap type
                    let is_heap = (self.struct_bitmaps[sid] >> idx) & 1 != 0;

                    // GC: release old value, retain new value (if heap type and not self-assign)
                    if is_heap {
                        // -1 is the null sentinel; self-assign of same handle is a no-op
                        let self_assign = old_val == val && old_val != -1;
                        if !self_assign && old_val != -1 {
                            self.heap.release(old_val as usize);
                        }
                        // Write new value
                        if let HeapObj::Struct(_, fields) = self.heap_get_mut(hid)? {
                            if idx >= len {
                                return Err(RuntimeError::FieldOutOfBounds { index: idx, len });
                            }
                            fields[idx] = val;
                        }
                        if !self_assign && val != -1 {
                            self.heap.retain(val as usize);
                        }
                    } else {
                        // Non-heap field: just write
                        if let HeapObj::Struct(_, fields) = self.heap_get_mut(hid)? {
                            if idx >= len {
                                return Err(RuntimeError::FieldOutOfBounds { index: idx, len });
                            }
                            fields[idx] = val;
                        }
                    }
                }

                // ── 索引 ──
                Opcode::IndexGet => {
                    // IndexGet(dst, obj, idx)
                    let d = inst.dst();
                    let o = inst.src1();
                    let i = inst.src2();
                    let hid = self.regs.regs[o] as i64;
                    let index = self.regs.regs[i] as i64 as usize;
                    match self.heap_get(hid)? {
                        HeapObj::List(v) => {
                            let val = *v
                                .get(index)
                                .ok_or(RuntimeError::IndexOutOfBounds(index as i64, v.len()))?;
                            self.write_int(d, val);
                        }
                        HeapObj::Int64Array(v) => {
                            let val = *v
                                .get(index)
                                .ok_or(RuntimeError::IndexOutOfBounds(index as i64, v.len()))?;
                            self.write_int(d, val);
                        }
                        HeapObj::Float64Array(v) => {
                            let val = *v
                                .get(index)
                                .ok_or(RuntimeError::IndexOutOfBounds(index as i64, v.len()))?;
                            self.regs.regs[d] = val.to_bits();
                        }
                        other => {
                            return Err(RuntimeError::TypeMismatch(format!(
                                "IndexGet: expected indexable, got {other:?}"
                            )))
                        }
                    }
                }
                Opcode::IndexSet => {
                    // IndexSet(val, obj, idx)
                    let val = inst.dst();
                    let obj = inst.src1();
                    let idx = inst.src2();
                    let hid = self.regs.regs[obj] as i64;
                    let index = self.regs.regs[idx] as i64 as usize;
                    let value = self.regs.regs[val] as i64;
                    let f64_val = f64::from_bits(self.regs.regs[val]); // read before heap_get_mut borrow
                    match self.heap_get_mut(hid)? {
                        HeapObj::List(v) => {
                            if index >= v.len() {
                                return Err(RuntimeError::IndexOutOfBounds(index as i64, v.len()));
                            }
                            v[index] = value;
                        }
                        HeapObj::Int64Array(v) => {
                            if index >= v.len() {
                                return Err(RuntimeError::IndexOutOfBounds(index as i64, v.len()));
                            }
                            v[index] = value;
                        }
                        HeapObj::Float64Array(v) => {
                            if index >= v.len() {
                                return Err(RuntimeError::IndexOutOfBounds(index as i64, v.len()));
                            }
                            v[index] = f64_val;
                        }
                        other => {
                            return Err(RuntimeError::TypeMismatch(format!(
                                "IndexSet: expected indexable, got {other:?}"
                            )));
                        }
                    }
                    self.write_int(val, value);
                }

                // ── Enum/Variant ──
                Opcode::NewVariant => {
                    let d = inst.dst();
                    let enum_id = inst.src1();
                    let tag = inst.src2() as u16;
                    let nf = self.enum_variant_counts[enum_id][tag as usize];
                    let bitmap = self.enum_variant_bitmaps[enum_id][tag as usize];
                    let mut fields = vec![0i64; nf];
                    for (i, f) in fields.iter_mut().enumerate() {
                        if (bitmap >> i) & 1 != 0 {
                            *f = -1;
                        }
                    }
                    self.write_heap(d, HeapObj::Variant(enum_id, tag, fields));
                }
                Opcode::GetVariantTag => {
                    let d = inst.dst();
                    let s = inst.src1();
                    let hid = self.regs.regs[s] as i64;
                    let tag = match self.heap_get(hid)? {
                        HeapObj::Variant(_, tag, _) => *tag as i64,
                        other => {
                            return Err(RuntimeError::TypeMismatch(format!(
                                "GetVariantTag expected variant, got {other:?}"
                            )))
                        }
                    };
                    self.write_int(d, tag);
                }
                Opcode::GetVariantField => {
                    let d = inst.dst();
                    let s = inst.src1();
                    let fi = inst.src2();
                    let hid = self.regs.regs[s] as i64;
                    let val = match self.heap_get(hid)? {
                        HeapObj::Variant(_, _, fields) => {
                            fields
                                .get(fi)
                                .copied()
                                .ok_or(RuntimeError::FieldOutOfBounds {
                                    index: fi,
                                    len: fields.len(),
                                })?
                        }
                        other => {
                            return Err(RuntimeError::TypeMismatch(format!(
                                "GetVariantField expected variant, got {other:?}"
                            )))
                        }
                    };
                    self.write_int(d, val);
                }
                Opcode::SetVariantField => {
                    // SetVariantField(val_reg, obj_reg, field_idx)
                    let d = inst.dst(); // val reg
                    let s = inst.src1(); // obj reg
                    let fi = inst.src2(); // field idx
                    let hid = self.regs.regs[s] as i64;
                    let val = self.regs.regs[d] as i64;
                    let (old_val, is_heap) =
                        match self.heap_get(hid)? {
                            HeapObj::Variant(eid, t, fields) => {
                                let old = fields.get(fi).copied().ok_or(
                                    RuntimeError::FieldOutOfBounds {
                                        index: fi,
                                        len: fields.len(),
                                    },
                                )?;
                                let bitmap = self
                                    .enum_variant_bitmaps
                                    .get(*eid)
                                    .and_then(|bm| bm.get(*t as usize))
                                    .copied()
                                    .unwrap_or(0);
                                let heap = (bitmap >> fi) & 1 != 0;
                                (old, heap)
                            }
                            other => {
                                return Err(RuntimeError::TypeMismatch(format!(
                                    "SetVariantField expected variant, got {other:?}"
                                )))
                            }
                        };
                    if is_heap {
                        // GC: release old, write new, retain new (skip if self-assign)
                        let self_assign = old_val == val && old_val != -1;
                        if !self_assign && old_val != -1 {
                            self.heap.release(old_val as usize);
                        }
                        if let HeapObj::Variant(_, _, fields) = self.heap_get_mut(hid)? {
                            fields[fi] = val;
                        }
                        if !self_assign && val != -1 {
                            self.heap.retain(val as usize);
                        }
                    } else if let HeapObj::Variant(_, _, fields) = self.heap_get_mut(hid)? {
                        fields[fi] = val;
                    }
                }

                // ── 装箱/拆箱 ──
                Opcode::Box_ => {
                    // Box(dst, src) — wrap value in a single-field struct
                    let d = inst.dst();
                    let s = inst.src1();
                    let val = self.regs.regs[s] as i64;
                    // Use struct id 0 as a "Box" marker, single field
                    self.write_heap(d, HeapObj::Struct(0, vec![val]));
                }
                Opcode::Unbox => {
                    // Unbox(dst, src) — extract value from boxed struct
                    let d = inst.dst();
                    let s = inst.src1();
                    let hid = self.regs.regs[s] as i64;
                    let val = match self.heap_get(hid)? {
                        HeapObj::Struct(_, fields) => *fields.first().ok_or_else(|| {
                            RuntimeError::TypeMismatch("Unbox: empty boxed struct".into())
                        })?,
                        other => {
                            return Err(RuntimeError::TypeMismatch(format!(
                                "Unbox: expected struct, got {other:?}"
                            )));
                        }
                    };
                    self.write_int(d, val);
                }

                // ── 控制流 ──
                Opcode::Jump => {
                    let block_id = (inst.src1() << 8) | inst.src2();
                    #[allow(clippy::unnecessary_to_owned)]
                    let args = self.jump_args(ip - 1).to_vec();
                    self.bind_params(block_id, &args);
                    let target_ip = self.block_ip(block_id);
                    // Backward jump detection: target IP at or before current IP
                    // means we're looping.
                    if target_ip <= ip {
                        self.check_loop_iteration(block_id, events)?;
                    }
                    ip = target_ip;
                }
                Opcode::Branch => {
                    let c = inst.dst();
                    let tb = inst.src1();
                    let fb = inst.src2();
                    let take_true = self.regs.regs[c] as i64 != 0;
                    let block_id = if take_true { tb } else { fb };
                    // Stored as: [true_count, true_args..., false_count, false_args...]
                    // Clone only the needed args (typically 0-4 elements) to drop jump_args borrow
                    let stored = self.jump_args(ip - 1);
                    let true_cnt = stored[0];
                    let false_cnt = stored[1 + true_cnt];
                    let args: Vec<usize> = if take_true {
                        stored[1..1 + true_cnt].to_vec()
                    } else {
                        stored[2 + true_cnt..2 + true_cnt + false_cnt].to_vec()
                    };
                    self.bind_params(block_id, &args);
                    let target_ip = self.block_ip(block_id);
                    // Backward jump detection: branch to at or before current IP
                    // means we're looping.
                    if target_ip <= ip {
                        self.check_loop_iteration(block_id, events)?;
                    }
                    ip = target_ip;
                }

                // ── 调用 ──
                Opcode::Call => {
                    // Call(func_idx, args, cont_block)
                    let func_idx = inst.dst();
                    let cont_block = (inst.src1() << 8) | inst.src2();
                    if self.frames.len() >= MAX_CALL_DEPTH {
                        return Err(RuntimeError::StackOverflow);
                    }
                    let callee_regs = self.func_reg_counts[func_idx];
                    // Take the caller's register vec (O(1) pointer swap)
                    let saved_regs = std::mem::take(&mut self.regs.regs);
                    // Prepare callee registers by reusing the just-emptied vec
                    self.regs.regs.resize(callee_regs, 0);
                    // Copy args from saved caller registers into callee positions
                    let args = self.jump_args(ip - 1).to_vec();
                    for (i, &arg_reg) in args.iter().enumerate() {
                        if i < callee_regs {
                            self.regs.regs[i] = saved_regs[arg_reg];
                        }
                    }
                    self.frames.push(CallFrame {
                        func_idx: self.current_func,
                        ret_block: cont_block,
                        saved_regs,
                        result_reg: 0,
                    });
                    self.current_func = func_idx;
                    ip = self.func_entries[func_idx];
                }
                Opcode::TailCall => {
                    // Tail call: bind args from jump_args into first N regs, jump to entry
                    let args = self.jump_args(ip - 1).to_vec();
                    // Bind args to the entry block's param registers
                    let params = &self.func_params[self.current_func]
                        .first()
                        .cloned()
                        .unwrap_or_default();
                    for (i, &arg_reg) in args.iter().enumerate() {
                        if i < params.len() {
                            self.regs.regs[params[i]] = self.regs.regs[arg_reg];
                        }
                    }
                    ip = self.block_ip(0); // jump to entry block 0
                }
                Opcode::Return => {
                    // ret
                    let r = inst.dst();
                    if let Some(frame) = self.frames.pop() {
                        let result = self.regs.regs[r];
                        self.regs.regs = frame.saved_regs;
                        self.current_func = frame.func_idx;
                        if self.regs.regs.len() <= frame.result_reg {
                            self.regs.regs.resize(frame.result_reg + 1, 0);
                        }
                        self.regs.regs[frame.result_reg] = result;
                        ip = self.block_ip(frame.ret_block);
                    } else {
                        return Ok(self.regs.regs[r] as i64);
                    }
                }

                // ── native call ──
                Opcode::CallNative => {
                    let fi = inst.dst();
                    let ret_block = (inst.src1() << 8) | inst.src2();
                    let args: Vec<i64> = self
                        .jump_args(ip - 1)
                        .iter()
                        .map(|&r| self.regs.regs[r] as i64)
                        .collect();
                    if fi < self.natives.len() {
                        let result = (self.natives[fi].1)(&args, &self.heap)
                            .map_err(RuntimeError::NativeError)?;
                        self.write_int(0, result);
                    } else {
                        return Err(RuntimeError::Bug(format!("unknown native index {fi}")));
                    }
                    ip = self.block_ip(ret_block);
                }

                // ── async ──
                Opcode::AsyncPoll => {
                    if let Some((_, result)) = self.scheduler.poll() {
                        self.write_int(0, result);
                    }
                }
                Opcode::Suspend => {
                    // suspend
                    let cf = CallFrame {
                        func_idx: self.current_func,
                        ret_block: 0,
                        saved_regs: self.regs.regs.clone(),
                        result_reg: 0,
                    };
                    self.scheduler.suspend(cf, ip);
                    return Ok(0);
                }

                // ── interface dispatch ──
                Opcode::LoadVtable => {
                    // LoadVtable(dst, vtable_idx) — store vtable_idx in register
                    let d = inst.dst();
                    let vi = inst.src1();
                    // Store vtable index as a special tagged value (negative to distinguish from heap handles)
                    // We use a negative sentinel: -(vtable_idx + 1) so it's never 0
                    self.write_int(d, -((vi as i64) + 1));
                }
                Opcode::NewInterfaceObj => {
                    // NewInterfaceObj(dst, vtable_reg, struct_reg)
                    let d = inst.dst();
                    let vr = inst.src1();
                    let sr = inst.src2();
                    // Decode vtable_idx from the tagged register value
                    let raw = self.regs.regs[vr] as i64;
                    let vtable_idx = ((-raw) - 1) as usize;
                    let data = self.regs.regs[sr] as i64;
                    // Retain the data handle since InterfaceObj now holds a reference
                    if data >= 0 {
                        self.heap.retain(data as usize);
                    }
                    self.write_heap(d, HeapObj::InterfaceObj { vtable_idx, data });
                }
                Opcode::CallIndirect => {
                    // CallIndirect(slot, args..., cont_block)
                    let slot = inst.dst();
                    let cont_block = (inst.src1() << 8) | inst.src2();
                    if self.frames.len() >= MAX_CALL_DEPTH {
                        return Err(RuntimeError::StackOverflow);
                    }
                    let args = self.jump_args(ip - 1).to_vec();
                    if args.is_empty() {
                        return Err(RuntimeError::Bug(
                            "CallIndirect: no args (need at least self)".into(),
                        ));
                    }
                    // First arg is the InterfaceObj handle
                    let iface_handle = self.regs.regs[args[0]] as i64;
                    let (vtable_idx, data_handle) = match self.heap_get(iface_handle)? {
                        HeapObj::InterfaceObj { vtable_idx, data } => (*vtable_idx, *data),
                        other => {
                            return Err(RuntimeError::TypeMismatch(format!(
                                "CallIndirect: expected InterfaceObj, got {other:?}"
                            )))
                        }
                    };
                    // Look up the method func_idx from the vtable
                    let vtable = self.vtables.get(vtable_idx).ok_or_else(|| {
                        RuntimeError::Bug(format!("vtable index {vtable_idx} out of bounds"))
                    })?;
                    let (_, func_idx) = vtable.methods.get(slot).ok_or_else(|| {
                        RuntimeError::Bug(format!(
                            "vtable slot {slot} out of bounds (vtable '{}' has {} methods)",
                            vtable.interface_name,
                            vtable.methods.len()
                        ))
                    })?;
                    let func_idx = *func_idx;
                    let callee_regs = self.func_reg_counts[func_idx];
                    // Save caller regs
                    let saved_regs = std::mem::take(&mut self.regs.regs);
                    // Prepare callee registers
                    self.regs.regs.resize(callee_regs, 0);
                    // Copy args from saved caller registers into callee positions
                    // Replace first arg (InterfaceObj handle) with the actual data handle
                    for (i, &arg_reg) in args.iter().enumerate() {
                        if i < callee_regs {
                            if i == 0 {
                                self.regs.regs[i] = data_handle as u64;
                            } else {
                                self.regs.regs[i] = saved_regs[arg_reg];
                            }
                        }
                    }
                    self.frames.push(CallFrame {
                        func_idx: self.current_func,
                        ret_block: cont_block,
                        saved_regs,
                        result_reg: 0,
                    });
                    self.current_func = func_idx;
                    ip = self.func_entries[func_idx];
                }

                // ── print ──
                Opcode::Print => {
                    let r = inst.dst();
                    let val = self.regs.regs[r] as i64;
                    if val >= 0 {
                        if let Some(HeapObj::String(s)) = self.heap.try_get(val as usize) {
                            self.output.push(s.clone());
                        } else {
                            self.output.push(format!("{val}"));
                        }
                    } else {
                        self.output.push(format!("{val}"));
                    }
                }

                #[allow(unreachable_patterns)]
                _ => return Err(RuntimeError::InvalidOpcode(opcode as u8)),
            }
        }
    }
}

// ── 指令编码 ──

fn encode_instr(instr: &CpsInstr) -> Result<u32, String> {
    Ok(match instr {
        CpsInstr::BinOp(d, op, s1, s2) => encode(
            match op {
                CpsBinOp::AddInt => Opcode::AddInt as u8,
                CpsBinOp::SubInt => Opcode::SubInt as u8,
                CpsBinOp::MulInt => Opcode::MulInt as u8,
                CpsBinOp::DivInt => Opcode::DivInt as u8,
                CpsBinOp::ModInt => Opcode::ModInt as u8,
                CpsBinOp::FAdd => Opcode::FAdd as u8,
                CpsBinOp::FSub => Opcode::FSub as u8,
                CpsBinOp::FMul => Opcode::FMul as u8,
                CpsBinOp::FDiv => Opcode::FDiv as u8,
                CpsBinOp::FEq => Opcode::FEq as u8,
                CpsBinOp::FNe => Opcode::FNe as u8,
                CpsBinOp::FLt => Opcode::FLt as u8,
                CpsBinOp::FLe => Opcode::FLe as u8,
                CpsBinOp::FGt => Opcode::FGt as u8,
                CpsBinOp::FGe => Opcode::FGe as u8,
                CpsBinOp::EqInt => Opcode::EqInt as u8,
                CpsBinOp::NeInt => Opcode::NeInt as u8,
                CpsBinOp::LtInt => Opcode::LtInt as u8,
                CpsBinOp::LeInt => Opcode::LeInt as u8,
                CpsBinOp::GtInt => Opcode::GtInt as u8,
                CpsBinOp::GeInt => Opcode::GeInt as u8,
                CpsBinOp::IToF => Opcode::IToF as u8,
                CpsBinOp::FToI => Opcode::FToI as u8,
                CpsBinOp::IToS => Opcode::IToS as u8,
                CpsBinOp::FToS => Opcode::FToS as u8,
                CpsBinOp::SToI => Opcode::SToI as u8,
                CpsBinOp::BToS => Opcode::BToS as u8,
                CpsBinOp::SAdd => Opcode::SAdd as u8,
            },
            *d as u32,
            *s1 as u32,
            *s2 as u32,
        ),
        CpsInstr::UnOp(d, op, s) => encode(
            match op {
                CpsUnOp::NegInt => Opcode::NegInt as u8,
                CpsUnOp::FNeg => Opcode::FNeg as u8,
                CpsUnOp::Not => Opcode::Not as u8,
            },
            *d as u32,
            *s as u32,
            0,
        ),
        CpsInstr::LoadConst(d, idx) => encode(Opcode::LoadConst as u8, *d as u32, *idx as u32, 0),
        CpsInstr::Move(d, s) => encode(Opcode::Move as u8, *d as u32, *s as u32, 0),
        CpsInstr::NewStruct(d, sid, _) => {
            encode(Opcode::NewStruct as u8, *d as u32, *sid as u32, 0)
        }
        CpsInstr::GetField(d, o, idx) => {
            encode(Opcode::GetField as u8, *d as u32, *o as u32, *idx as u32)
        }
        CpsInstr::SetField(d, o, idx, _) => {
            encode(Opcode::SetField as u8, *d as u32, *o as u32, *idx as u32)
        }
        CpsInstr::NewVariant(d, eid, tag, _) => encode(
            Opcode::NewVariant as u8,
            *d as u32,
            *eid as u32,
            *tag as u32,
        ),
        CpsInstr::GetVariantTag(d, o) => {
            encode(Opcode::GetVariantTag as u8, *d as u32, *o as u32, 0)
        }
        CpsInstr::GetVariantField(d, o, fi) => encode(
            Opcode::GetVariantField as u8,
            *d as u32,
            *o as u32,
            *fi as u32,
        ),
        CpsInstr::SetVariantField(d, o, fi, _) => encode(
            Opcode::SetVariantField as u8,
            *d as u32,
            *o as u32,
            *fi as u32,
        ),
        CpsInstr::NewList(d, elements) => {
            encode(Opcode::NewList as u8, *d as u32, elements.len() as u32, 0)
        }
        CpsInstr::NewTuple(d, elements) => {
            encode(Opcode::NewTuple as u8, *d as u32, elements.len() as u32, 0)
        }
        CpsInstr::TupleIndex(d, t, idx) => {
            encode(Opcode::TupleIndex as u8, *d as u32, *t as u32, *idx as u32)
        }
        CpsInstr::NewInt64Array(d, elements) => encode(
            Opcode::NewInt64Array as u8,
            *d as u32,
            elements.len() as u32,
            0,
        ),
        CpsInstr::NewFloat64Array(d, elements) => encode(
            Opcode::NewFloat64Array as u8,
            *d as u32,
            elements.len() as u32,
            0,
        ),
        CpsInstr::ListLen(d, obj) => encode(Opcode::ListLen as u8, *d as u32, *obj as u32, 0),
        CpsInstr::IndexGet(d, o, i) => {
            encode(Opcode::IndexGet as u8, *d as u32, *o as u32, *i as u32)
        }
        CpsInstr::IndexSet(d, o, i, _) => {
            encode(Opcode::IndexSet as u8, *d as u32, *o as u32, *i as u32)
        }
        CpsInstr::Box(d, s) => encode(Opcode::Box_ as u8, *d as u32, *s as u32, 0),
        CpsInstr::Unbox(d, s) => encode(Opcode::Unbox as u8, *d as u32, *s as u32, 0),
        CpsInstr::Print(r) => encode(Opcode::Print as u8, *r as u32, 0, 0),
        CpsInstr::LoadVtable(d, vi) => encode(Opcode::LoadVtable as u8, *d as u32, *vi as u32, 0),
        CpsInstr::NewInterfaceObj(d, vr, sr) => encode(
            Opcode::NewInterfaceObj as u8,
            *d as u32,
            *vr as u32,
            *sr as u32,
        ),
        CpsInstr::LoadExternalConst(..) => {
            return Err(
                "LoadExternalConst should be resolved by LinkStage before VM execution".into(),
            )
        }
        CpsInstr::Nop => return Err("nop is not executable".into()),
    })
}

fn encode_term(term: &CpsTerminator) -> Result<u32, String> {
    Ok(match term {
        CpsTerminator::Jump(b, _) => {
            encode(Opcode::Jump as u8, 0, (*b >> 8) as u32, (*b & 0xFF) as u32)
        }
        CpsTerminator::Branch(c, tb, _, fb, _) => {
            encode(Opcode::Branch as u8, *c as u32, *tb as u32, *fb as u32)
        }
        CpsTerminator::Suspend => encode(Opcode::Suspend as u8, 0, 0, 0),
        CpsTerminator::Return(r) => encode(Opcode::Return as u8, *r as u32, 0, 0),
        CpsTerminator::Call(fi, _, ret) => encode(
            Opcode::Call as u8,
            *fi as u32,
            (*ret >> 8) as u32,
            (*ret & 0xFF) as u32,
        ),
        CpsTerminator::CallNative(fi, _, ret) => encode(
            Opcode::CallNative as u8,
            *fi as u32,
            (*ret >> 8) as u32,
            (*ret & 0xFF) as u32,
        ),
        CpsTerminator::CallIndirect(slot, _, ret) => encode(
            Opcode::CallIndirect as u8,
            *slot as u32,
            (*ret >> 8) as u32,
            (*ret & 0xFF) as u32,
        ),
        CpsTerminator::TailCall(_, _) => encode(Opcode::TailCall as u8, 0, 0, 0),
        // CallExternal: should be resolved by LinkStage before reaching VM.
        // Encode same as Call for unlinked single-module test usage.
        CpsTerminator::CallExternal {
            import_handle,
            ret_block,
            ..
        } => encode(
            Opcode::Call as u8,
            *import_handle as u32,
            (*ret_block >> 8) as u32,
            (*ret_block & 0xFF) as u32,
        ),
        CpsTerminator::CallExternalDynamic { .. } => {
            return Err("CallExternalDynamic not supported by VM (use LinkStage)".into())
        }
    })
}

// ── tests ──

#[cfg(test)]
#[allow(clippy::approx_constant)]
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
            enums: vec![],
            vtables: vec![],
            symbol_map: HashMap::new(),
            func_owners: vec![],
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
        assert_eq!(vm.execute(0, 3, None).unwrap(), 42);
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
            vm.execute(0, 3, None),
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
            enums: vec![],
            vtables: vec![],
            symbol_map: HashMap::new(),
            func_owners: vec![],
        };
        let mut vm = VM::new();
        vm.load(&m).unwrap();
        assert_eq!(vm.execute(0, 4, None).unwrap(), 10);
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
        assert_eq!(vm.execute(0, 2, None).unwrap(), -42);
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
        assert_eq!(vm.execute(0, 2, None).unwrap(), 1); // !0 = true = 1
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
        vm.execute(0, 1, None).unwrap();
        assert!(!vm.output.is_empty(), "output should have print result");
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
        let r = vm.execute(0, 1, None).unwrap();
        if let HeapObj::String(s) = vm.heap_get(r).unwrap() {
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
            enums: vec![],
            vtables: vec![],
            symbol_map: HashMap::new(),
            func_owners: vec![],
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
        let result = vm.execute(0, 2, None).unwrap();
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
        let _ = vm.execute(0, 3, None).unwrap();
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
        vm.execute(0, 2, None).unwrap();
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
            consts.push(Constant::String(format!("s{i}")));
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
        let _ = vm.execute(0, 40, None).unwrap();
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
        let r = vm.execute(0, 2, None).unwrap();
        if let HeapObj::String(s) = vm.heap_get(r).unwrap() {
            assert_eq!(s, "42");
        } else {
            panic!("expected heap string, got idx {r}");
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
        let r = vm.execute(0, 4, None).unwrap();
        if let HeapObj::String(s) = vm.heap_get(r).unwrap() {
            assert_eq!(s, "200");
        } else {
            panic!("expected heap string, got idx {r}");
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
        let r = vm.execute(0, 2, None).unwrap();
        if let HeapObj::String(s) = vm.heap_get(r).unwrap() {
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
                CpsInstr::Move(8, 6),
                CpsInstr::Move(9, 7),
                CpsInstr::BinOp(10, CpsBinOp::AddInt, 8, 9),
            ],
            CpsTerminator::Return(10),
            vec![Constant::Float(1.5), Constant::Float(2.0)],
            11,
        );
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        assert_eq!(vm.execute(0, 11, None).unwrap(), 2);
    }

    #[test]
    fn float_comparisons_write_boolean_int_registers() {
        let cps = simple_mod(
            vec![
                CpsInstr::LoadConst(0, 0),
                CpsInstr::LoadConst(1, 1),
                CpsInstr::BinOp(2, CpsBinOp::FEq, 0, 0),
                CpsInstr::BinOp(3, CpsBinOp::FNe, 0, 1),
                CpsInstr::BinOp(4, CpsBinOp::FLt, 0, 1),
                CpsInstr::BinOp(5, CpsBinOp::FLe, 0, 0),
                CpsInstr::BinOp(6, CpsBinOp::FGt, 1, 0),
                CpsInstr::BinOp(7, CpsBinOp::FGe, 1, 1),
                CpsInstr::BinOp(8, CpsBinOp::AddInt, 2, 3),
                CpsInstr::BinOp(8, CpsBinOp::AddInt, 8, 4),
                CpsInstr::BinOp(8, CpsBinOp::AddInt, 8, 5),
                CpsInstr::BinOp(8, CpsBinOp::AddInt, 8, 6),
                CpsInstr::BinOp(8, CpsBinOp::AddInt, 8, 7),
            ],
            CpsTerminator::Return(8),
            vec![Constant::Float(1.5), Constant::Float(2.0)],
            9,
        );
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        assert_eq!(vm.execute(0, 9, None).unwrap(), 6);
    }

    #[test]
    fn load_bool_and_null_constants() {
        let cps = simple_mod(
            vec![
                CpsInstr::LoadConst(0, 0),
                CpsInstr::LoadConst(1, 1),
                CpsInstr::LoadConst(2, 2),
                CpsInstr::BinOp(3, CpsBinOp::AddInt, 0, 1),
                CpsInstr::BinOp(4, CpsBinOp::AddInt, 3, 2),
            ],
            CpsTerminator::Return(4),
            vec![Constant::Bool(true), Constant::Bool(false), Constant::Null],
            5,
        );
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        assert_eq!(vm.execute(0, 5, None).unwrap(), 1);
    }

    #[test]
    fn invalid_heap_access_returns_runtime_errors() {
        let get = simple_mod(
            vec![
                CpsInstr::LoadConst(0, 0),
                CpsInstr::LoadConst(1, 1),
                CpsInstr::IndexGet(2, 0, 1),
            ],
            CpsTerminator::Return(2),
            vec![Constant::Int(99), Constant::Int(0)],
            3,
        );
        let mut vm = VM::new();
        vm.load(&get).unwrap();
        assert!(matches!(
            vm.execute(0, 3, None),
            Err(RuntimeError::InvalidHeapHandle(99))
        ));

        let field = simple_mod_with_structs(
            vec![
                CpsInstr::NewStruct(0, 0, vec![]),
                CpsInstr::GetField(1, 0, 1),
            ],
            CpsTerminator::Return(1),
            vec![],
            vec![StructDef {
                id: 0,
                name: "One".to_string(),
                fields: vec![("x".to_string(), "Int64".to_string())],
                type_bitmap: 0,
            }],
            2,
        );
        let mut vm = VM::new();
        vm.load(&field).unwrap();
        assert!(matches!(
            vm.execute(0, 2, None),
            Err(RuntimeError::FieldOutOfBounds { index: 1, len: 1 })
        ));
    }

    #[test]
    fn native_errors_are_returned_not_zeroed() {
        let native = CpsModule {
            functions: vec![CpsFunction {
                name: "main".into(),
                blocks: vec![
                    CpsBlock {
                        id: 0,
                        params: vec![],
                        instrs: vec![CpsInstr::LoadConst(0, 0)],
                        term: CpsTerminator::CallNative(2, vec![0], 1),
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
            constants: vec![Constant::Int(0)],
            structs: vec![],
            enums: vec![],
            vtables: vec![],
            symbol_map: HashMap::new(),
            func_owners: vec![],
        };
        let mut vm = VM::new();
        vm.load(&native).unwrap();
        assert!(matches!(
            vm.execute(0, 1, None),
            Err(RuntimeError::NativeError(_))
        ));
    }

    #[test]
    fn test_index_out_of_bounds() {
        let cps = simple_mod(
            vec![CpsInstr::LoadConst(0, 0), CpsInstr::IndexGet(2, 1, 0)],
            CpsTerminator::Return(2),
            vec![Constant::Int(99)],
            3,
        );
        let mut vm = VM::new();
        vm.regs.regs[1] = vm.heap.alloc(HeapObj::List(vec![])) as u64;
        vm.load(&cps).unwrap();
        vm.regs.regs[1] = 0;
        assert!(matches!(
            vm.execute(0, 3, None),
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
            enums: vec![],
            vtables: vec![],
            symbol_map: HashMap::new(),
            func_owners: vec![],
        };
        let mut vm = VM::new();
        vm.load(&native).unwrap();
        assert_eq!(vm.execute(0, 1, None).unwrap(), 7);

        let suspended = simple_mod(vec![], CpsTerminator::Suspend, vec![], 1);
        let mut vm = VM::new();
        vm.load(&suspended).unwrap();
        assert_eq!(vm.execute(0, 1, None).unwrap(), 0);
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
            enums: vec![],
            vtables: vec![],
            symbol_map: HashMap::new(),
            func_owners: vec![],
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
        let result = vm.execute(1, 2, None).unwrap();
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
        vm.execute(1, 2, None).unwrap();
        assert!(
            vm.output.iter().any(|s| s.contains("hi")),
            "print inside lambda: output={:?}",
            vm.output
        );
    }

    // ── Bug regression tests ──

    #[test]
    fn t20_loadconst_above_255() {
        // LoadConst index encoded in src1 (9 bits), handler must read all 9 bits
        let mut consts: Vec<Constant> = (0..256).map(|_| Constant::Int(0)).collect();
        consts.push(Constant::Int(42)); // index 256
        let cps = CpsModule {
            functions: vec![CpsFunction {
                name: "main".into(),
                blocks: vec![CpsBlock {
                    id: 0,
                    params: vec![],
                    instrs: vec![CpsInstr::LoadConst(0, 256)],
                    term: CpsTerminator::Return(0),
                }],
                entry: 0,
                reg_count: 1,
            }],
            constants: consts,
            structs: vec![],
            enums: vec![],
            vtables: vec![],
            symbol_map: HashMap::new(),
            func_owners: vec![],
        };
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        assert_eq!(vm.execute(0, 1, None).unwrap(), 42);
    }

    #[test]
    fn t21_overflow_neg_div_mod() {
        // NegInt on i64::MIN, DivInt/ModInt on i64::MIN / -1 should not panic
        let cps = CpsModule {
            functions: vec![CpsFunction {
                name: "main".into(),
                blocks: vec![CpsBlock {
                    id: 0,
                    params: vec![],
                    instrs: vec![
                        CpsInstr::LoadConst(0, 0),                  // r0 = i64::MIN
                        CpsInstr::LoadConst(1, 1),                  // r1 = -1
                        CpsInstr::UnOp(2, CpsUnOp::NegInt, 0),      // r2 = -r0
                        CpsInstr::BinOp(3, CpsBinOp::DivInt, 0, 1), // r3 = r0 / r1
                        CpsInstr::BinOp(4, CpsBinOp::ModInt, 0, 1), // r4 = r0 % r1
                        CpsInstr::BinOp(5, CpsBinOp::AddInt, 2, 3), // r5 = r2 + r3
                        CpsInstr::BinOp(6, CpsBinOp::AddInt, 5, 4), // r6 = r5 + r4
                    ],
                    term: CpsTerminator::Return(6),
                }],
                entry: 0,
                reg_count: 7,
            }],
            constants: vec![Constant::Int(i64::MIN), Constant::Int(-1)],
            structs: vec![],
            enums: vec![],
            vtables: vec![],
            symbol_map: HashMap::new(),
            func_owners: vec![],
        };
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        // Must not panic — wrapping behavior
        let _ = vm.execute(0, 7, None).unwrap();
    }

    #[test]
    fn t22_setfield_self_assign_consistent_rc() {
        // struct Foo { s: String }
        let structs = vec![StructDef {
            id: 0,
            name: "Foo".into(),
            fields: vec![("s".into(), "String".into())],
            type_bitmap: 0b01,
        }];
        // Load "hi" → r0, NewStruct → r1, SetField(r0,r1,0), SetField(r0,r1,0) again.
        // Self-assign must not corrupt ref-counts: string rc should be 2 (LoadConst + 1 retain from field)
        let instrs = vec![
            CpsInstr::LoadConst(0, 0),         // r0 = "hi" (slot 0, rc=1)
            CpsInstr::NewStruct(1, 0, vec![]), // r1 = struct (slot 1, rc=1)
            CpsInstr::SetField(0, 1, 0, 0),    // retain: rc 1→2
            CpsInstr::SetField(0, 1, 0, 0),    // self-assign: release-retain should cancel
        ];
        let cps = CpsModule {
            functions: vec![CpsFunction {
                name: "main".into(),
                blocks: vec![CpsBlock {
                    id: 0,
                    params: vec![],
                    instrs,
                    term: CpsTerminator::Return(1),
                }],
                entry: 0,
                reg_count: 2,
            }],
            constants: vec![Constant::String("hi".into())],
            structs,
            enums: vec![],
            vtables: vec![],
            symbol_map: HashMap::new(),
            func_owners: vec![],
        };
        let mut vm = VM::new();
        vm.load(&cps).unwrap();
        let result = vm.execute(0, 2, None).unwrap();
        // String rc: LoadConst=1, SetField retain=+1, self-assign release=-1 then retain=+1 → net +1 → rc=2
        assert_eq!(vm.heap.ref_count(0), 2, "string rc should be 2");
        assert_eq!(vm.heap.ref_count(1), 1, "struct rc should be 1");
        // Field still points to the string
        if let HeapObj::Struct(_, fields) = vm.heap_get(result).unwrap() {
            assert_eq!(fields[0], 0);
        } else {
            panic!("expected struct");
        }
    }

    #[test]
    fn box_unbox_roundtrip() {
        let m = simple_mod(
            vec![
                CpsInstr::LoadConst(0, 0), // r0 = 42
                CpsInstr::Box(1, 0),       // r1 = Box(r0)
                CpsInstr::Unbox(2, 1),     // r2 = Unbox(r1)
            ],
            CpsTerminator::Return(2),
            vec![Constant::Int(42)],
            3,
        );
        let mut vm = VM::new();
        vm.load(&m).unwrap();
        assert_eq!(vm.execute(0, 3, None).unwrap(), 42);
    }
}
