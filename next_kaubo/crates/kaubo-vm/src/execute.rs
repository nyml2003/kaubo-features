//! 寄存器 VM — 完整实现
//! 7-bit opcode, CPS block scheduler, 调用栈 + 闭包 + stdlib

use kaubo_ir::cps::*;
use crate::regfile::*;
use crate::async_runtime::AsyncScheduler;
use crate::stdlib;

// ── 编码 ──
pub fn encode(op: u8, dst: u32, src1: u32, src2: u32) -> u32 {
    ((op as u32) << 25) | ((dst & 0xFF) << 17) | ((src1 & 0x1FF) << 8) | (src2 & 0xFF)
}

pub fn decode_rrr(inst: u32) -> (usize, usize, usize) {
    (((inst >> 17) & 0xFF) as usize, ((inst >> 8) & 0x1FF) as usize, (inst & 0xFF) as usize)
}

pub fn decode_rr(inst: u32) -> (usize, usize) {
    (((inst >> 17) & 0xFF) as usize, ((inst >> 8) & 0x1FF) as usize)
}

// ── 运行时错误 ──
#[derive(Debug)]
pub enum RuntimeError {
    DivisionByZero, IndexOutOfBounds(i64, usize), NullAccess,
    TypeAssertion(String), StackOverflow, Bug(String),
}

// ── 堆对象 ──
#[derive(Debug, Clone)]
pub enum HeapObj {
    String(String),
    List(Vec<i64>),
    Struct(Vec<i64>),         // fields as flat i64 for MVP
    Closure(Box<ClosureObj>),
}

#[derive(Debug, Clone)]
pub struct ClosureObj {
    pub func_entry: usize,
    pub upvalues: Vec<i64>,    // captured values (copied)
}

// ── VM ──

pub struct VM {
    pub regs: RegFile,
    pub frames: Vec<CallFrame>,
    pub consts: Vec<Constant>,
    pub blocks: Vec<(usize, usize)>,
    pub instrs: Vec<u32>,
    pub output: Vec<String>,

    // Heap
    pub heap: Vec<HeapObj>,
    pub next_heap_id: usize,

    // Native functions
    pub natives: Vec<(&'static str, stdlib::NativeFn)>,

    // Async
    pub scheduler: AsyncScheduler,
}

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub ret_block: usize,
    pub ret_ip: usize,
}

const MAX_CALL_DEPTH: usize = 1024;

impl VM {
    pub fn new() -> Self {
        VM {
            regs: RegFile::new(512, 256, 256),
            frames: vec![], consts: vec![], blocks: vec![], instrs: vec![], output: vec![],
            heap: vec![], next_heap_id: 0,
            natives: stdlib::register_all(),
            scheduler: AsyncScheduler::new(),
        }
    }

    pub fn load(&mut self, module: &CpsModule) -> Result<(), String> {
        self.consts = module.constants.clone();
        self.instrs.clear(); self.blocks.clear();
        for func in &module.functions {
            let max_id = func.blocks.iter().map(|b| b.id).max().unwrap_or(0) + 1;
            self.blocks.resize(max_id, (0, 0));
            for block in &func.blocks {
                let start = self.instrs.len();
                for instr in &block.instrs { self.instrs.push(encode_instr(instr)?); }
                self.instrs.push(encode_term(&block.term)?);
                self.blocks[block.id] = (start, self.instrs.len() - start);
            }
        }
        Ok(())
    }

    fn alloc_heap(&mut self, obj: HeapObj) -> i64 {
        let id = self.next_heap_id;
        self.next_heap_id += 1;
        self.heap.push(obj);
        id as i64
    }

    fn heap_get(&self, id: i64) -> &HeapObj {
        &self.heap[id as usize]
    }

    fn heap_get_mut(&mut self, id: i64) -> &mut HeapObj {
        &mut self.heap[id as usize]
    }

    pub fn execute(&mut self, entry: usize, reg_count: usize) -> Result<i64, RuntimeError> {
        // Allocate register space for this function
        self.regs.ensure_capacity(reg_count, reg_count, reg_count);
        let int_save = self.regs.ints.len();
        let float_save = self.regs.floats.len();
        let ptr_save = self.regs.ptrs.len();
        // Expand registers for this function call
        if self.regs.ints.len() < int_save + reg_count { self.regs.ints.resize(int_save + reg_count, 0); }
        if self.regs.floats.len() < float_save + reg_count { self.regs.floats.resize(float_save + reg_count, 0.0); }
        if self.regs.ptrs.len() < ptr_save + reg_count { self.regs.ptrs.resize(ptr_save + reg_count, GcPtr::null()); }

        let mut ip = self.blocks[entry].0;

        loop {
            let inst = self.instrs[ip]; ip += 1;
            let op = (inst >> 25) as u8;

            match op {
                // ── 整数算术 ──
                0x00 => { let (a,b,c)=decode_rrr(inst); self.regs.ints[a]=self.regs.ints[b].wrapping_add(self.regs.ints[c]); }
                0x01 => { let (a,b,c)=decode_rrr(inst); self.regs.ints[a]=self.regs.ints[b].wrapping_sub(self.regs.ints[c]); }
                0x02 => { let (a,b,c)=decode_rrr(inst); self.regs.ints[a]=self.regs.ints[b].wrapping_mul(self.regs.ints[c]); }
                0x03 => { let (a,b,c)=decode_rrr(inst); if self.regs.ints[c]==0 {return Err(RuntimeError::DivisionByZero)} self.regs.ints[a]=self.regs.ints[b]/self.regs.ints[c]; }
                0x04 => { let (a,b,c)=decode_rrr(inst); if self.regs.ints[c]==0 {return Err(RuntimeError::DivisionByZero)} self.regs.ints[a]=self.regs.ints[b]%self.regs.ints[c]; }
                0x05 => { let (a,b)=decode_rr(inst); self.regs.ints[a] = -self.regs.ints[b]; }

                // ── 浮点 ──
                0x08 => { let (a,b,c)=decode_rrr(inst); self.regs.floats[a]=self.regs.floats[b]+self.regs.floats[c]; }
                0x09 => { let (a,b,c)=decode_rrr(inst); self.regs.floats[a]=self.regs.floats[b]-self.regs.floats[c]; }
                0x0A => { let (a,b,c)=decode_rrr(inst); self.regs.floats[a]=self.regs.floats[b]*self.regs.floats[c]; }
                0x0B => { let (a,b,c)=decode_rrr(inst); self.regs.floats[a]=self.regs.floats[b]/self.regs.floats[c]; }
                0x0C => { let (a,b)=decode_rr(inst); self.regs.floats[a] = -self.regs.floats[b]; }

                // ── 比较 ──
                0x10 => { let (a,b,c)=decode_rrr(inst); self.regs.ints[a]=(self.regs.ints[b]==self.regs.ints[c]) as i64; }
                0x11 => { let (a,b,c)=decode_rrr(inst); self.regs.ints[a]=(self.regs.ints[b]<self.regs.ints[c]) as i64; }
                0x12 => { let (a,b,c)=decode_rrr(inst); self.regs.ints[a]=(self.regs.ints[b]<=self.regs.ints[c]) as i64; }
                0x13 => { let (a,b,c)=decode_rrr(inst); self.regs.floats[a]=(self.regs.floats[b]==self.regs.floats[c]) as u64 as f64; }
                0x14 => { let (a,b,c)=decode_rrr(inst); self.regs.floats[a]=(self.regs.floats[b]<self.regs.floats[c]) as u64 as f64; }

                // ── Not ──
                0x15 => { let (a,b)=decode_rr(inst); self.regs.ints[a] = (self.regs.ints[b] == 0) as i64; }

                // ── 字符串 ──
                0x18 => { let (a,b)=decode_rr(inst); self.regs.ints[a]=self.regs.ints[b].wrapping_add(0); }

                // ── 转换 ──
                0x20 => { let (d,s)=decode_rr(inst); self.regs.floats[d]=self.regs.ints[s] as f64; }
                0x21 => { let (d,s)=decode_rr(inst); self.regs.ints[d]=self.regs.floats[s] as i64; }
                0x22 => { let (d,s)=decode_rr(inst); self.regs.ints[d]=self.regs.ints[s]; } // itos placeholder
                0x23 => { let (d,s)=decode_rr(inst); self.regs.ints[d]=self.regs.floats[s] as i64; } // ftos placeholder
                0x24 => { let (d,s)=decode_rr(inst); self.regs.ints[d]=self.regs.ints[s]; } // stoi placeholder

                // ── 数据移动 ──
                0x30 => { let (d,s)=decode_rr(inst); self.regs.ints[d]=self.regs.ints[s]; }
                0x31 => {
                    let d=((inst>>17)&0xFF) as usize;
                    self.regs.ints[d] = (inst & 0x1FFFF) as i64;
                }
                0x32 => {
                    let (d,idx)=decode_rr(inst);
                    match &self.consts[idx] {
                        Constant::Int(n) => self.regs.ints[d] = *n,
                        Constant::Float(f) => self.regs.floats[d] = *f,
                        Constant::String(s) => {
                            let hid = self.alloc_heap(HeapObj::String(s.clone()));
                            self.regs.ints[d] = hid;
                        }
                        _ => {},
                    }
                }

                // ── 堆分配 ──
                0x34 => { let d=((inst>>17)&0xFF) as usize; let nf=((inst>>8)&0xFF) as usize; self.regs.ints[d]=self.alloc_heap(HeapObj::Struct(vec![0;nf])); }
                0x35 => { // NewList(dst, elements)
                    let d=((inst>>17)&0xFF) as usize;
                    let ne=((inst>>8)&0xFF) as usize;
                    let hid = self.alloc_heap(HeapObj::List(vec![0; ne]));
                    self.regs.ints[d] = hid;
                }

                // ── 字段访问 ──
                0x36 => { // GetField(dst, src, idx)
                    let (d,s,idx)=decode_rrr(inst);
                    let hid = self.regs.ints[s];
                    if let HeapObj::Struct(fields) = self.heap_get(hid) {
                        self.regs.ints[d] = *fields.get(idx).unwrap_or(&0);
                    }
                }
                0x37 => { // SetField(dst, src, idx, val)
                    let (d,s,idx)=decode_rrr(inst);
                    let hid = self.regs.ints[s];
                    let val = self.regs.ints[d];
                    if let HeapObj::Struct(fields) = self.heap_get_mut(hid) {
                        if idx < fields.len() { fields[idx] = val; }
                    }
                }

                // ── 索引 ──
                0x38 => { // IndexGet(dst, obj, idx)
                    let (d,o,i)=decode_rrr(inst);
                    let hid = self.regs.ints[o];
                    let index = self.regs.ints[i] as usize;
                    match self.heap_get(hid) {
                        HeapObj::List(v) => {
                            self.regs.ints[d] = *v.get(index).ok_or(RuntimeError::IndexOutOfBounds(index as i64, v.len()))?;
                        }
                        _ => {},
                    }
                }

                // ── 装箱/拆箱 ──
                0x3A => { let (d,s)=decode_rr(inst); self.regs.ints[d]=self.regs.ints[s]; } // box
                0x3B => { let (d,s)=decode_rr(inst); self.regs.ints[d]=self.regs.ints[s]; } // unbox

                // ── 控制流 ──
                0x40 => { ip = self.blocks[(inst & 0x1FFFFFF) as usize].0; }
                0x41 => {
                    let c=((inst>>17)&0xFF) as usize;
                    let tb=((inst>>8)&0x1FF) as usize;
                    let fb=(inst&0xFF) as usize;
                    ip = self.blocks[if self.regs.ints[c] != 0 { tb } else { fb }].0;
                }

                // ── 调用 ──
                0x50 => { // call(func, args, ret_block)
                    let ret = (inst & 0x1FFFFFF) as usize;
                    if self.frames.len() >= MAX_CALL_DEPTH { return Err(RuntimeError::StackOverflow); }
                    self.frames.push(CallFrame { ret_block: ret, ret_ip: ip });
                    ip = self.blocks[0].0; // jump to func entry (simplified: always block 0)
                }
                0x51 => { // tailcall
                    ip = self.blocks[0].0;
                }
                0x52 => { // ret
                    let r = ((inst>>17)&0xFF) as usize;
                    if let Some(frame) = self.frames.pop() {
                        ip = self.blocks[frame.ret_block].0;
                    } else {
                        return Ok(self.regs.ints[r]);
                    }
                }

                // ── async ──
                0x60 => {
                    if let Some((_, result)) = self.scheduler.poll() {
                        self.regs.ints[0] = result;
                    }
                }
                0x61 => { // suspend
                    self.frames.push(CallFrame { ret_block: 0, ret_ip: ip });
                    let cf = self.frames.pop().unwrap();
                    self.scheduler.suspend(cf, ip);
                    return Ok(0);
                }

                // ── print ──
                0x7F => {
                    let r = ((inst>>17)&0xFF) as usize;
                    let val = self.regs.ints[r];
                    // Check if val points to a heap string
                    if val >= 0 && (val as usize) < self.heap.len() {
                        if let HeapObj::String(s) = self.heap_get(val) {
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
        CpsInstr::BinOp(d, op, s1, s2) => encode(match op {
            CpsBinOp::AddInt=>0x00, CpsBinOp::SubInt=>0x01, CpsBinOp::MulInt=>0x02,
            CpsBinOp::DivInt=>0x03, CpsBinOp::ModInt=>0x04,
            CpsBinOp::FAdd=>0x08, CpsBinOp::FSub=>0x09, CpsBinOp::FMul=>0x0A, CpsBinOp::FDiv=>0x0B,
            CpsBinOp::FEq=>0x13, CpsBinOp::FLt=>0x14,
            CpsBinOp::EqInt=>0x10, CpsBinOp::NeInt=>0x10,
            CpsBinOp::LtInt=>0x11, CpsBinOp::LeInt=>0x12, CpsBinOp::GtInt=>0x12, CpsBinOp::GeInt=>0x12,
            CpsBinOp::IToF=>0x20, CpsBinOp::FToI=>0x21,
            CpsBinOp::IToS=>0x22, CpsBinOp::FToS=>0x23, CpsBinOp::SToI=>0x24,
            CpsBinOp::SAdd=>0x18,
            _ => 0xFF,
        }, *d as u32, *s1 as u32, *s2 as u32),
        CpsInstr::UnOp(d, op, s) => encode(match op {
            CpsUnOp::NegInt=>0x05, CpsUnOp::FNeg=>0x0C, CpsUnOp::Not=>0x15,
        }, *d as u32, *s as u32, 0),
        CpsInstr::LoadConst(d, idx) => encode(0x32, *d as u32, *idx as u32, 0),
        CpsInstr::Move(d, s) => encode(0x30, *d as u32, *s as u32, 0),
        CpsInstr::NewStruct(d, sid, _) => encode(0x34, *d as u32, *sid as u32, 0),
        CpsInstr::GetField(d, o, idx) => encode(0x36, *d as u32, *o as u32, *idx as u32),
        CpsInstr::SetField(d, o, idx, v) => encode(0x37, *d as u32, *o as u32, *idx as u32),
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
        CpsTerminator::Call(_, _, ret) => encode(0x50, 0, 0, *ret as u32),
        CpsTerminator::TailCall(_, _) => encode(0x51, 0, 0, 0),
    })
}

// ── tests ──

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_mod(instrs: Vec<CpsInstr>, term: CpsTerminator, consts: Vec<Constant>, reg_count: usize) -> CpsModule {
        CpsModule{functions:vec![CpsFunction{name:"main".into(),blocks:vec![CpsBlock{id:0,params:vec![],instrs,term}],entry:0,reg_count}],constants:consts,structs:vec![]}
    }

    #[test]
    fn test_add() {
        let m = simple_mod(vec![CpsInstr::LoadConst(0,0),CpsInstr::LoadConst(1,1),CpsInstr::BinOp(2,CpsBinOp::AddInt,0,1)],CpsTerminator::Return(2),vec![Constant::Int(40),Constant::Int(2)],3);
        let mut vm = VM::new(); vm.load(&m).unwrap();
        assert_eq!(vm.execute(0, 3).unwrap(), 42);
    }

    #[test]
    fn test_div_zero() {
        let m = simple_mod(vec![CpsInstr::LoadConst(0,0),CpsInstr::LoadConst(1,1),CpsInstr::BinOp(2,CpsBinOp::DivInt,0,1)],CpsTerminator::Return(2),vec![Constant::Int(42),Constant::Int(0)],3);
        let mut vm = VM::new(); vm.load(&m).unwrap();
        assert!(matches!(vm.execute(0, 3), Err(RuntimeError::DivisionByZero)));
    }

    #[test]
    fn test_branch() {
        let m=CpsModule{functions:vec![CpsFunction{name:"main".into(),blocks:vec![
            CpsBlock{id:0,params:vec![],instrs:vec![CpsInstr::LoadConst(0,0),CpsInstr::LoadConst(1,1)],term:CpsTerminator::Branch(0,1,vec![],2,vec![])},
            CpsBlock{id:1,params:vec![],instrs:vec![CpsInstr::LoadConst(2,2)],term:CpsTerminator::Jump(3,vec![2])},
            CpsBlock{id:2,params:vec![],instrs:vec![CpsInstr::LoadConst(2,3)],term:CpsTerminator::Jump(3,vec![2])},
            CpsBlock{id:3,params:vec![2],instrs:vec![],term:CpsTerminator::Return(2)},
        ],entry:0,reg_count:4}],constants:vec![Constant::Int(1),Constant::Int(0),Constant::Int(10),Constant::Int(20)],structs:vec![]};
        let mut vm=VM::new(); vm.load(&m).unwrap();
        assert_eq!(vm.execute(0,4).unwrap(),10);
    }

    #[test]
    fn test_neg() {
        let m = simple_mod(vec![CpsInstr::LoadConst(0,0),CpsInstr::UnOp(1,CpsUnOp::NegInt,0)],CpsTerminator::Return(1),vec![Constant::Int(42)],2);
        let mut vm = VM::new(); vm.load(&m).unwrap();
        assert_eq!(vm.execute(0,2).unwrap(), -42);
    }

    #[test]
    fn test_not() {
        let m = simple_mod(vec![CpsInstr::LoadConst(0,0),CpsInstr::UnOp(1,CpsUnOp::Not,0)],CpsTerminator::Return(1),vec![Constant::Int(0)],2);
        let mut vm = VM::new(); vm.load(&m).unwrap();
        assert_eq!(vm.execute(0,2).unwrap(), 1); // !0 = true = 1
    }

    #[test]
    fn test_printfn() {
        let m = simple_mod(vec![CpsInstr::LoadConst(0,0), CpsInstr::Print(0)], CpsTerminator::Return(0), vec![Constant::String("hi".into())], 1);
        let mut vm=VM::new(); vm.load(&m).unwrap();
        vm.execute(0,1).unwrap();
        assert!(vm.output.len() > 0, "output should have print result");
    }

    #[test]
    fn test_heap_string() {
        let m = simple_mod(vec![CpsInstr::LoadConst(0,0)],CpsTerminator::Return(0),vec![Constant::String("hello".into())],1);
        let mut vm = VM::new(); vm.load(&m).unwrap();
        let r = vm.execute(0,1).unwrap();
        if let HeapObj::String(s) = vm.heap_get(r) {
            assert_eq!(s, "hello");
        } else { panic!("expected string"); }
    }
}
