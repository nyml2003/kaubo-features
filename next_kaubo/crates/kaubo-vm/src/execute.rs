//! 寄存器 VM — 块调度器主循环
//!
//! 32-bit 固定宽度指令: [7-bit op][8-bit dst][9-bit src1][8-bit src2]
//! 零 push/pop, CPS blocks 执行

use kaubo_ir::cps::*;
use crate::regfile::*;
use crate::async_runtime::AsyncScheduler;

pub fn rrr(d: usize, s1: usize, s2: usize) -> (usize, usize, usize) { (d, s1, s2) }
pub fn rr(d: usize, s: usize) -> (usize, usize) { (d, s) }
pub fn imm(d: usize, i: u32) -> (usize, u32) { (d, i) }

pub fn encode(op: u8, dst: u32, src1: u32, src2: u32) -> u32 {
    ((op as u32) << 25) | ((dst & 0xFF) << 17) | ((src1 & 0x1FF) << 8) | (src2 & 0xFF)
}

pub fn decode_rrr(inst: u32) -> (usize, usize, usize) {
    (((inst >> 17) & 0xFF) as usize, ((inst >> 8) & 0x1FF) as usize, (inst & 0xFF) as usize)
}

pub fn decode_rr(inst: u32) -> (usize, usize) {
    (((inst >> 17) & 0xFF) as usize, ((inst >> 8) & 0x1FF) as usize)
}

// ── 运行时 ──

#[derive(Debug)]
pub enum RuntimeError { DivisionByZero, IndexOutOfBounds, NullAccess, TypeAssertion(String), StackOverflow, Bug(String) }

pub struct VM {
    pub regs: RegFile,
    pub frames: Vec<CallFrame>,
    pub consts: Vec<Constant>,
    pub blocks: Vec<(usize, usize)>,
    pub func_entry: usize,
    pub instrs: Vec<u32>,
    pub output: Vec<String>,
    pub scheduler: AsyncScheduler,
}

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub int_base: usize, pub float_base: usize, pub ptr_base: usize, pub ptr_count: usize,
    pub ret_block: usize, pub func_entry: usize,
}

impl VM {
    pub fn new() -> Self { VM { regs: RegFile::new(256,256,256), frames:vec![], consts:vec![], blocks:vec![], func_entry:0, instrs:vec![], output:vec![], scheduler: AsyncScheduler::new() } }

    pub fn load(&mut self, module: &CpsModule) -> Result<(), String> {
        self.consts = module.constants.clone();
        self.instrs.clear(); self.blocks.clear();
        for func in &module.functions {
            let max_id = func.blocks.iter().map(|b|b.id).max().unwrap_or(0)+1;
            self.blocks.resize(max_id, (0,0));
            for block in &func.blocks {
                let start = self.instrs.len();
                for instr in &block.instrs { self.instrs.push(encode_instr(instr)?); }
                self.instrs.push(encode_term(&block.term)?);
                self.blocks[block.id] = (start, self.instrs.len()-start);
            }
            self.func_entry = func.entry;
        }
        Ok(())
    }

    pub fn execute(&mut self, entry: usize) -> Result<i64, RuntimeError> {
        let mut ip = self.blocks[entry].0;
        loop {
            let inst = self.instrs[ip]; ip += 1;
            let op = (inst >> 25) as u8;
            match op {
                0x00 => { let (a,b,c)=decode_rrr(inst); self.regs.ints[a]=self.regs.ints[b].wrapping_add(self.regs.ints[c]); }
                0x01 => { let (a,b,c)=decode_rrr(inst); self.regs.ints[a]=self.regs.ints[b].wrapping_sub(self.regs.ints[c]); }
                0x02 => { let (a,b,c)=decode_rrr(inst); self.regs.ints[a]=self.regs.ints[b].wrapping_mul(self.regs.ints[c]); }
                0x03 => { let (a,b,c)=decode_rrr(inst); if self.regs.ints[c]==0 {return Err(RuntimeError::DivisionByZero)} self.regs.ints[a]=self.regs.ints[b]/self.regs.ints[c]; }
                0x08 => { let (a,b,c)=decode_rrr(inst); self.regs.floats[a]=self.regs.floats[b]+self.regs.floats[c]; }
                0x09 => { let (a,b,c)=decode_rrr(inst); self.regs.floats[a]=self.regs.floats[b]-self.regs.floats[c]; }
                0x10 => { let (a,b,c)=decode_rrr(inst); self.regs.ints[a]=(self.regs.ints[b]==self.regs.ints[c]) as i64; }
                0x11 => { let (a,b,c)=decode_rrr(inst); self.regs.ints[a]=(self.regs.ints[b]<self.regs.ints[c]) as i64; }
                0x20 => { let (d,s)=decode_rr(inst); self.regs.floats[d]=self.regs.ints[s] as f64; }
                0x21 => { let (d,s)=decode_rr(inst); self.regs.ints[d]=self.regs.floats[s] as i64; }
                0x30 => { let (d,s)=decode_rr(inst); self.regs.ints[d]=self.regs.ints[s]; }
                0x31 => {
                    let d=((inst>>17)&0xFF) as usize;
                    let imm=(inst&0x1FFFF) as i64;
                    self.regs.ints[d]=imm;
                }
                0x32 => {
                    let (d,idx)=decode_rr(inst);
                    match &self.consts[idx] { Constant::Int(n)=>self.regs.ints[d]=*n, Constant::Float(f)=>self.regs.floats[d]=*f, _=>{} }
                }
                0x40 => { ip = self.blocks[(inst&0x1FFFFFF) as usize].0; }
                0x41 => {
                    let c=((inst>>17)&0xFF) as usize;
                    let tb=((inst>>8)&0x1FF) as usize;
                    let fb=(inst&0xFF) as usize;
                    ip = self.blocks[if self.regs.ints[c]!=0 {tb} else {fb}].0;
                }
                0x50 => { self.frames.push(CallFrame{int_base:0,float_base:0,ptr_base:0,ptr_count:0,ret_block:(inst&0x1FFFFFF)as usize,func_entry:self.func_entry}); ip=self.blocks[self.func_entry].0; }
                0x52 => {
                    let r=((inst>>17)&0xFF) as usize;
                    if let Some(f)=self.frames.pop() {
                        self.regs.frame_end(f.int_base,f.float_base,f.ptr_base,f.ptr_count);
                        ip=self.blocks[f.ret_block].0;
                    } else { return Ok(self.regs.ints[r]); }
                }
                // ── async/await ──
                0x60 => { // await: 尝试恢复已完成的 async 任务
                    let task_id = ((inst>>17)&0xFF) as usize;
                    if let Some((_, result)) = self.scheduler.poll() {
                        self.regs.ints[task_id] = result;
                    } else {
                        // 无已完成任务时阻塞 (v2.0: 简化为 immediate return)
                    }
                }
                0x61 => { // suspend: 挂起当前帧
                    let cf = CallFrame { int_base:0,float_base:0,ptr_base:0,ptr_count:0,ret_block:0,func_entry:self.func_entry };
                    self.scheduler.suspend(cf, ip);
                    return Ok(0); // 返回 0 表示挂起
                }
                0xF0 => { let (d,_)=decode_rr(inst); self.output.push(format!("{}",self.regs.ints[d])); }
                _ => {}
            }
        }
    }
}

fn encode_instr(instr: &CpsInstr) -> Result<u32, String> {
    Ok(match instr {
        CpsInstr::BinOp(d,op,s1,s2) => encode(match op {
            CpsBinOp::AddInt=>0x00,CpsBinOp::SubInt=>0x01,CpsBinOp::MulInt=>0x02,CpsBinOp::DivInt=>0x03,
            CpsBinOp::FAdd=>0x08,CpsBinOp::FSub=>0x09,
            CpsBinOp::EqInt=>0x10,CpsBinOp::LtInt=>0x11,
            CpsBinOp::IToF=>0x20,CpsBinOp::FToI=>0x21,
            _ => 0xFF,
        },*d as u32,*s1 as u32,*s2 as u32),
        CpsInstr::LoadConst(d,idx) => encode(0x32,*d as u32,*idx as u32,0),
        CpsInstr::Move(d,s) => encode(0x30,*d as u32,*s as u32,0),
        CpsInstr::Nop => 0,
        _ => return Err(format!("unsupported: {:?}",instr)),
    })
}

fn encode_term(term: &CpsTerminator) -> Result<u32, String> {
    Ok(match term {
        CpsTerminator::Jump(b,_) => encode(0x40,0,0,*b as u32),
        CpsTerminator::Branch(c,tb,_,fb,_) => encode(0x41,*c as u32,*tb as u32,*fb as u32),
        CpsTerminator::Suspend => encode(0x61,0,0,0),
        CpsTerminator::Return(r) => encode(0x52,*r as u32,0,0),
        CpsTerminator::Call(_,_,ret) => encode(0x50,0,0,*ret as u32),
        CpsTerminator::TailCall(_,_) => encode(0x51,0,0,0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_mod(instrs: Vec<CpsInstr>, term: CpsTerminator, consts: Vec<Constant>, regs: usize) -> CpsModule {
        CpsModule{functions:vec![CpsFunction{name:"main".into(),blocks:vec![CpsBlock{id:0,params:vec![],instrs,term}],entry:0,reg_count:regs}],constants:consts,structs:vec![]}
    }

    #[test]
    fn test_add() {
        let mut vm=VM::new();
        let m=simple_mod(vec![CpsInstr::LoadConst(0,0),CpsInstr::LoadConst(1,1),CpsInstr::BinOp(2,CpsBinOp::AddInt,0,1)],CpsTerminator::Return(2),vec![Constant::Int(40),Constant::Int(2)],3);
        vm.load(&m).unwrap(); vm.regs.ints.resize(3,0);
        assert_eq!(vm.execute(0).unwrap(),42);
    }

    #[test]
    fn test_div_zero() {
        let mut vm=VM::new();
        let m=simple_mod(vec![CpsInstr::LoadConst(0,0),CpsInstr::LoadConst(1,1),CpsInstr::BinOp(2,CpsBinOp::DivInt,0,1)],CpsTerminator::Return(2),vec![Constant::Int(42),Constant::Int(0)],3);
        vm.load(&m).unwrap(); vm.regs.ints.resize(3,0);
        assert!(matches!(vm.execute(0),Err(RuntimeError::DivisionByZero)));
    }

    #[test]
    fn test_branch() {
        let mut vm=VM::new();
        let m=CpsModule{functions:vec![CpsFunction{name:"main".into(),blocks:vec![
            CpsBlock{id:0,params:vec![],instrs:vec![CpsInstr::LoadConst(0,0),CpsInstr::LoadConst(1,1)],term:CpsTerminator::Branch(0,1,vec![],2,vec![])},
            CpsBlock{id:1,params:vec![],instrs:vec![CpsInstr::LoadConst(2,2)],term:CpsTerminator::Jump(3,vec![2])},
            CpsBlock{id:2,params:vec![],instrs:vec![CpsInstr::LoadConst(2,3)],term:CpsTerminator::Jump(3,vec![2])},
            CpsBlock{id:3,params:vec![2],instrs:vec![],term:CpsTerminator::Return(2)},
        ],entry:0,reg_count:4}],constants:vec![Constant::Int(1),Constant::Int(0),Constant::Int(10),Constant::Int(20)],structs:vec![]};
        vm.load(&m).unwrap(); vm.regs.ints.resize(4,0);
        assert_eq!(vm.execute(0).unwrap(),10);
    }

    #[test]
    fn test_async_suspend_resume() {
        let mut vm = VM::new();
        let m = CpsModule {
            functions: vec![CpsFunction {
                name: "main".into(),
                blocks: vec![
                    // block 0: entry → load constant, then suspend
                    CpsBlock { id: 0, params: vec![],
                        instrs: vec![CpsInstr::LoadConst(0, 0)],
                        term: CpsTerminator::Suspend,
                    },
                    // block 1: resume block → add 1, return
                    CpsBlock { id: 1, params: vec![0],
                        instrs: vec![CpsInstr::LoadConst(1, 1), CpsInstr::BinOp(2, CpsBinOp::AddInt, 0, 1)],
                        term: CpsTerminator::Return(2),
                    },
                ],
                entry: 0, reg_count: 3,
            }],
            constants: vec![Constant::Int(0), Constant::Int(1)],
            structs: vec![],
        };
        vm.load(&m).unwrap();
        vm.regs.ints.resize(3, 0);

        // Step 1: execute → hits Suspend
        let result = vm.execute(0);
        assert_eq!(result.unwrap(), 0); // Suspend returns 0
        assert!(vm.scheduler.has_pending());

        // Step 2: resume
        let pending = vm.scheduler.pending_ids();
        assert_eq!(pending.len(), 1);
        vm.scheduler.complete(pending[0], 42);

        // Step 3: poll for result
        assert_eq!(vm.scheduler.poll(), Some((pending[0], 42)));
    }
}
