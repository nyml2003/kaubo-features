//! cps_build — block-level CPS construction
//!
//! Orchestrates: register allocation, block creation, chaining, scope management.
//! Uses cps_emit for individual instruction generation.
//!
//! Supports multi-function compilation: lambda expressions create separate CpsFunctions.
//! Every build_* returns (entry, continu, reg):
//!   entry   — first block to jump INTO
//!   continu — block whose terminator is Return (can be chained FROM)
//!   reg     — register holding the result value

use std::collections::HashMap;
use kaubo_syntax::ast::*;
use crate::cps::*;
use crate::cps_emit;

pub fn build_module(module: &Module) -> Result<CpsModule, String> {
    let mut b = CpsBuilder::new();
    b.ctx.new_block(); // entry block id 0
    let mut tail: Option<usize> = None;

    for stmt in &module.stmts {
        let (entry, continu, _) = b.build_top_stmt(stmt)?;
        if entry == usize::MAX { continue; }
        if let Some(t) = tail { b.ctx.chain(t, entry)?; }
        else { b.ctx.set_block(0, block_jump(0, entry)); }
        tail = Some(continu);
    }

    b.finalize(0);
    dump_blocks("main", &b.ctx);
    Ok(CpsModule { functions: b.functions, constants: b.constants, structs: b.structs })
}

fn block_jump(id: usize, target: usize) -> CpsBlock {
    CpsBlock { id, params: vec![], instrs: vec![], term: CpsTerminator::Jump(target, vec![]) }
}

fn dump_blocks(label: &str, ctx: &FuncCtx) {
    if cfg!(debug_assertions) {
        eprintln!("[CPS {}] regs={} blocks={}", label, ctx.next_reg, ctx.blocks.len());
        for b in &ctx.blocks {
            eprintln!("  blk{} {:?} | {:?}", b.id, b.instrs, b.term);
        }
    }
}

// ── Per-function compilation context ──

pub struct FuncCtx {
    pub name: String,
    pub blocks: Vec<CpsBlock>,
    pub next_reg: usize,
    pub var_map: HashMap<String, usize>,
    pub func_map: HashMap<String, usize>,   // function name → func_idx
    pub loop_stack: Vec<(usize, usize)>,
}

impl FuncCtx {
    pub fn new(name: String) -> Self {
        FuncCtx {
            name, blocks: vec![], next_reg: 1, var_map: HashMap::new(),
            func_map: HashMap::new(), loop_stack: vec![],
        }
    }

    fn alloc(&mut self) -> usize { let r = self.next_reg; self.next_reg += 1; r }

    pub fn new_block(&mut self) -> usize {
        let id = self.blocks.len();
        self.blocks.push(CpsBlock { id, params: vec![], instrs: vec![],
            term: CpsTerminator::Return(0) });
        id
    }

    pub fn set_block(&mut self, id: usize, block: CpsBlock) {
        if id < self.blocks.len() { self.blocks[id] = block; }
    }

    /// Chain block `from` → `to`. Fails if `from` terminator is not Return.
    pub fn chain(&mut self, from: usize, to: usize) -> Result<(), String> {
        if from >= self.blocks.len() { return Ok(()); }
        if !matches!(self.blocks[from].term, CpsTerminator::Return(_)) {
            return Err(format!("chain: block {} not Return (already chained?)", from));
        }
        self.blocks[from].term = CpsTerminator::Jump(to, vec![]);
        Ok(())
    }

    /// Rewire `from` block's Return → Jump(target, args).
    fn rewire_return_args(&mut self, from: usize, target: usize, args: &[usize]) -> Result<(), String> {
        if from >= self.blocks.len() { return Ok(()); }
        if !matches!(self.blocks[from].term, CpsTerminator::Return(_)) {
            return Err(format!("rewire: block {} not Return", from));
        }
        self.blocks[from].term = CpsTerminator::Jump(target, args.to_vec());
        Ok(())
    }

    fn leaf_block(&mut self, reg: usize, const_idx: usize) -> (usize, usize) {
        let (instrs, term) = cps_emit::emit_literal(reg, const_idx);
        let id = self.new_block();
        self.set_block(id, CpsBlock { id, params: vec![], instrs, term });
        (id, id)
    }

    fn finalize(&self, entry_block: usize) -> CpsFunction {
        if self.blocks.is_empty() {
            return CpsFunction { name: self.name.clone(), blocks: vec![], entry: 0, reg_count: 0 };
        }
        if cfg!(debug_assertions) { eprintln!("[CPS FINALIZE {}] entry_in={} total_blocks={}", self.name, entry_block, self.blocks.len()); }
        let mut id_map = HashMap::new();
        let mut new_blocks = Vec::new();
        for (i, b) in self.blocks.iter().enumerate() {
            id_map.insert(b.id, i);
            new_blocks.push(CpsBlock { id: i, params: b.params.clone(),
                instrs: b.instrs.clone(), term: b.term.clone() });
        }
        for b in &mut new_blocks { remap_term_ids(b, &id_map); }
        let entry = *id_map.get(&entry_block).unwrap_or(&0);
        if cfg!(debug_assertions) { eprintln!("[CPS FINALIZE {}] entry_out={}", self.name, entry); }
        CpsFunction { name: self.name.clone(), blocks: new_blocks, entry, reg_count: self.next_reg }
    }
}

// ── Module-level builder ──

pub struct CpsBuilder {
    pub functions: Vec<CpsFunction>,
    pub constants: Vec<Constant>,
    pub structs: Vec<StructDef>,
    const_map: HashMap<String, usize>,
    pub ctx: FuncCtx,
}

impl CpsBuilder {
    pub fn new() -> Self {
        CpsBuilder {
            functions: vec![], constants: vec![], structs: vec![],
            const_map: HashMap::new(),
            ctx: FuncCtx::new("main".into()),
        }
    }

    pub fn add_const(&mut self, c: Constant) -> usize {
        let key = format!("{:?}", c);
        *self.const_map.entry(key).or_insert_with(|| {
            let i = self.constants.len(); self.constants.push(c); i
        })
    }

    fn finalize(&mut self, entry: usize) {
        let f = self.ctx.finalize(entry);
        self.functions.push(f);
    }

    // ── Top-level statement ──

    fn build_top_stmt(&mut self, stmt: &Stmt) -> Result<(usize, usize, usize), String> {
        match stmt {
            Stmt::ConstDecl { name, value, .. } => {
                if matches!(value, Expr::Lambda { .. }) {
                    let func_idx = self.build_lambda_as_function(value)?;
                    self.ctx.func_map.insert(name.clone(), func_idx);
                    Ok((usize::MAX, usize::MAX, 0))
                } else {
                    let (entry, continu, reg) = self.build_expr(value)?;
                    self.ctx.var_map.insert(name.clone(), reg);
                    Ok((entry, continu, reg))
                }
            }
            Stmt::VarDecl { name, value, .. } => {
                if let Some(v) = value {
                    if matches!(v, Expr::Lambda { .. }) {
                        let func_idx = self.build_lambda_as_function(v)?;
                        self.ctx.func_map.insert(name.clone(), func_idx);
                        Ok((usize::MAX, usize::MAX, 0))
                    } else {
                        let (entry, continu, reg) = self.build_expr(v)?;
                        self.ctx.var_map.insert(name.clone(), reg);
                        Ok((entry, continu, reg))
                    }
                } else {
                    let r = self.ctx.alloc();
                    self.ctx.var_map.insert(name.clone(), r);
                    Ok((usize::MAX, usize::MAX, r))
                }
            }
            Stmt::ExprStmt(e) => self.build_expr(e),
            Stmt::StructDef { name, fields } => {
                let mut bitmap: u64 = 0;
                for (i, f) in fields.iter().enumerate() {
                    if is_heap_type(&f.ty) { bitmap |= 1 << i; }
                }
                self.structs.push(StructDef {
                    id: self.structs.len(), name: name.clone(),
                    fields: fields.iter().map(|f| (f.name.clone(), f.ty.to_string())).collect(),
                    type_bitmap: bitmap,
                });
                Ok((usize::MAX, usize::MAX, 0))
            }
            _ => Ok((usize::MAX, usize::MAX, 0)),
        }
    }

    // ── Expression dispatch ──

    fn build_expr(&mut self, expr: &Expr) -> Result<(usize, usize, usize), String> {
        match expr {
            Expr::LitInt(n) => {
                let r = self.ctx.alloc(); let c = self.add_const(Constant::Int(*n));
                let (e, l) = self.ctx.leaf_block(r, c); Ok((e, l, r))
            }
            Expr::LitFloat(n) => {
                let r = self.ctx.alloc(); let c = self.add_const(Constant::Float(*n));
                let (e, l) = self.ctx.leaf_block(r, c); Ok((e, l, r))
            }
            Expr::LitString(s) => {
                let r = self.ctx.alloc(); let c = self.add_const(Constant::String(s.clone()));
                let (e, l) = self.ctx.leaf_block(r, c); Ok((e, l, r))
            }
            Expr::LitTrue => {
                let r = self.ctx.alloc(); let c = self.add_const(Constant::Int(1));
                let (e, l) = self.ctx.leaf_block(r, c); Ok((e, l, r))
            }
            Expr::LitFalse | Expr::LitNull => {
                let r = self.ctx.alloc(); let c = self.add_const(Constant::Int(0));
                let (e, l) = self.ctx.leaf_block(r, c); Ok((e, l, r))
            }
            Expr::VarRef(name) => {
                if let Some(&reg) = self.ctx.var_map.get(name) {
                    let id = self.ctx.new_block();
                    self.ctx.set_block(id, CpsBlock { id, params: vec![], instrs: vec![],
                        term: cps_emit::emit_varref(reg).1 });
                    Ok((id, id, reg))
                } else {
                    self.build_expr(&Expr::LitInt(0))
                }
            }
            Expr::Binary { left, op, right } => self.build_binary(left, *op, right),
            Expr::Unary { op, right } => self.build_unary(op, right),
            Expr::Lambda { params, body, .. } => self.build_lambda(params, body),
            Expr::Block(stmts) => self.build_block(stmts),
            Expr::If { cond, then_branch, else_branch } =>
                self.build_if(cond, then_branch, else_branch.as_deref()),
            Expr::While { cond, body } => self.build_while(cond, body),
            Expr::For { body, .. } => self.build_expr(body),
            Expr::Break => self.build_break(),
            Expr::Continue => self.build_continue(),
            Expr::Return(val) => self.build_return(val.as_deref()),
            Expr::Member { object, field } => self.build_member(object, field),
            Expr::Call { func, args } => self.build_call(func, args),
            Expr::ListLit(items) => self.build_list(items),
            Expr::StructLit { name, fields } => self.build_struct_lit(name, fields),
            Expr::Index { object, index } => self.build_index(object, index),
            Expr::Assign { target, value } => self.build_assign(target, value),
            Expr::Async(body) | Expr::Await(body) => self.build_expr(body),
        }
    }

    fn build_stmt(&mut self, stmt: &Stmt) -> Result<(usize, usize, usize), String> {
        match stmt {
            Stmt::ConstDecl { name, value, .. } => {
                if matches!(value, Expr::Lambda { .. }) {
                    let func_idx = self.build_lambda_as_function(value)?;
                    self.ctx.func_map.insert(name.clone(), func_idx);
                    Ok((usize::MAX, usize::MAX, 0))
                } else {
                    let (entry, continu, reg) = self.build_expr(value)?;
                    self.ctx.var_map.insert(name.clone(), reg);
                    Ok((entry, continu, reg))
                }
            }
            Stmt::VarDecl { name, value, .. } => {
                if let Some(v) = value {
                    if matches!(v, Expr::Lambda { .. }) {
                        let func_idx = self.build_lambda_as_function(v)?;
                        self.ctx.func_map.insert(name.clone(), func_idx);
                        Ok((usize::MAX, usize::MAX, 0))
                    } else {
                        let (entry, continu, reg) = self.build_expr(v)?;
                        self.ctx.var_map.insert(name.clone(), reg);
                        Ok((entry, continu, reg))
                    }
                } else {
                    let r = self.ctx.alloc();
                    self.ctx.var_map.insert(name.clone(), r);
                    Ok((usize::MAX, usize::MAX, r))
                }
            }
            Stmt::ExprStmt(e) => self.build_expr(e),
            _ => Ok((usize::MAX, usize::MAX, 0)),
        }
    }

    // ── Lambda — creates a separate CpsFunction ──

    /// For top-level lambda bindings: swap ctx, compile body, swap back, return func_idx.
    fn build_lambda_as_function(&mut self, value: &Expr) -> Result<usize, String> {
        if let Expr::Lambda { params, body, .. } = value {
            let mut callee = FuncCtx::new(format!("lambda_{}", self.functions.len()));
            for (i, p) in params.iter().enumerate() {
                callee.var_map.insert(p.name.clone(), i);
            }
            callee.next_reg = params.len().max(1);

            // Swap ctx — build_expr operates on callee
            std::mem::swap(&mut self.ctx, &mut callee);
            let (entry, continu, result_reg) = self.build_expr(body)?;
            // Ensure body ends with Return(result_reg)
            if !matches!(self.ctx.blocks[continu].term, CpsTerminator::Return(_)) {
                let ri = self.ctx.new_block();
                self.ctx.set_block(ri, CpsBlock { id: ri, params: vec![], instrs: vec![],
                    term: CpsTerminator::Return(result_reg) });
                self.ctx.chain(continu, ri)?;
            }
            // Swap back — callee now has the lambda blocks
            std::mem::swap(&mut self.ctx, &mut callee);

            let func = callee.finalize(entry);
            let func_idx = self.functions.len();
            dump_blocks(&format!("lambda_{}", func_idx), &callee);
            self.functions.push(func);
            Ok(func_idx)
        } else {
            Err("expected lambda".into())
        }
    }

    // ── build_lambda for expression position ──

    fn build_lambda(&mut self, params: &[Param], body: &Expr) -> Result<(usize, usize, usize), String> {
        let mut callee = FuncCtx::new(format!("lambda_{}", self.functions.len()));
        for (i, p) in params.iter().enumerate() {
            callee.var_map.insert(p.name.clone(), i + 1);
        }
        callee.next_reg = params.len() + 1;
        callee.next_reg = params.len();

        std::mem::swap(&mut self.ctx, &mut callee);
        let (entry, continu, result_reg) = self.build_expr(body)?;
        if !matches!(self.ctx.blocks[continu].term, CpsTerminator::Return(_)) {
            let ri = self.ctx.new_block();
            self.ctx.set_block(ri, CpsBlock { id: ri, params: vec![], instrs: vec![],
                term: CpsTerminator::Return(result_reg) });
            self.ctx.chain(continu, ri)?;
        }
        std::mem::swap(&mut self.ctx, &mut callee);

        let func = callee.finalize(entry);
        let func_idx = self.functions.len();
        dump_blocks(&format!("lambda_expr_{}", func_idx), &callee);
        self.functions.push(func);
        let r = self.ctx.alloc();
        let cidx = self.add_const(Constant::Int(func_idx as i64));
        self.ctx.func_map.insert(format!("lambda_{}", func_idx), func_idx);
        let (e, l) = self.ctx.leaf_block(r, cidx);
        Ok((e, l, r))
    }

    // ── build_call — uses func_map to find function index ──

    fn build_call(&mut self, func: &Expr, args: &[Expr]) -> Result<(usize, usize, usize), String> {
        // to_string() — compile-time rewrite to IToS / FToS
        if let Expr::Member { object, field } = func {
            if field == "to_string" && args.is_empty() {
                let (entry, continu, obj_reg) = self.build_expr(object)?;
                let dst = self.ctx.alloc();
                let id = self.ctx.new_block();
                self.ctx.set_block(id, CpsBlock { id, params: vec![],
                    instrs: vec![CpsInstr::BinOp(dst, CpsBinOp::IToS, obj_reg, 0)],
                    term: cps_emit::emit_return(dst) });
                self.ctx.chain(continu, id)?;
                return Ok((entry, id, dst));
            }
        }

        if let Expr::VarRef(name) = func {
            if name == "print" {
                if let Some(arg) = args.first() {
                    // print("str") — inline
                    if let Expr::LitString(s) = arg {
                        let r = self.ctx.alloc(); let c = self.add_const(Constant::String(s.clone()));
                        let id = self.ctx.new_block();
                        self.ctx.set_block(id, CpsBlock { id, params: vec![],
                            instrs: vec![CpsInstr::LoadConst(r, c), CpsInstr::Print(r)],
                            term: cps_emit::emit_return(r) });
                        return Ok((id, id, r))
                    }
                    // print(x) where x is not a string literal — build the arg then print
                    let (entry, continu, reg) = self.build_expr(arg)?;
                    let id = self.ctx.new_block();
                    self.ctx.set_block(id, CpsBlock { id, params: vec![],
                        instrs: vec![CpsInstr::Print(reg)],
                        term: cps_emit::emit_return(reg) });
                    self.ctx.chain(continu, id)?;
                    return Ok((entry, id, reg));
                }
            }
            // Look up function index
            if let Some(&func_idx) = self.ctx.func_map.get(name) {
                return self.build_call_with_idx(func_idx, args);
            }
        }
        // Fallback: evaluate args inline
        let mut entry = 0;
        let mut prev_c: Option<usize> = None;
        let mut last_reg = 0;
        for arg in args {
            let (e, c, r) = self.build_expr(arg)?;
            if entry == 0 { entry = e; }
            if let Some(t) = prev_c { self.ctx.chain(t, e)?; }
            prev_c = Some(c);
            last_reg = r;
        }
        let r = self.ctx.alloc();
        let id = self.ctx.new_block();
        self.ctx.set_block(id, CpsBlock { id, params: vec![],
            instrs: vec![CpsInstr::Move(r, last_reg)], term: cps_emit::emit_return(r) });
        if let Some(t) = prev_c { self.ctx.chain(t, id)?; }
        Ok((if entry != 0 { entry } else { id }, id, r))
    }

    fn build_call_with_idx(&mut self, func_idx: usize, args: &[Expr]) -> Result<(usize, usize, usize), String> {
        let mut entry = 0;
        let mut prev_c: Option<usize> = None;
        let mut arg_regs = Vec::new();
        for arg in args {
            let (e, c, r) = self.build_expr(arg)?;
            if entry == 0 { entry = e; }
            if let Some(t) = prev_c { self.ctx.chain(t, e)?; }
            prev_c = Some(c);
            arg_regs.push(r);
        }
        let result_reg = self.ctx.alloc();
        let cont_block = self.ctx.new_block();
        let move_block = self.ctx.new_block();
        self.ctx.set_block(cont_block, CpsBlock { id: cont_block, params: vec![],
            instrs: vec![], term: CpsTerminator::Jump(move_block, vec![]) });
        self.ctx.set_block(move_block, CpsBlock { id: move_block, params: vec![],
            instrs: vec![CpsInstr::Move(result_reg, 0)], term: cps_emit::emit_return(result_reg) });
        let call_block = self.ctx.new_block();
        self.ctx.set_block(call_block, CpsBlock { id: call_block, params: vec![],
            instrs: vec![], term: cps_emit::emit_call(func_idx, arg_regs, cont_block) });
        if let Some(t) = prev_c { self.ctx.chain(t, call_block)?; }
        let entry = if entry != 0 { entry } else { call_block };
        Ok((entry, move_block, result_reg))
    }

    // ── Complex expressions (delegate to ctx) ──

    fn build_binary(&mut self, left: &Expr, op: BinOp, right: &Expr) -> Result<(usize, usize, usize), String> {
        let (bl, cl, rl) = self.build_expr(left)?;
        let (br, cr, rr) = self.build_expr(right)?;
        let r = self.ctx.alloc();
        // Gt/Ge: swap operands since VM only has Lt/Le
        let (binop, sl, sr) = match op {
            BinOp::Gt => (CpsBinOp::LtInt, rr, rl),
            BinOp::Ge => (CpsBinOp::LeInt, rr, rl),
            _ => (bin_op_to_cps(op), rl, rr),
        };
        let (instrs, _) = cps_emit::emit_binary(r, binop, sl, sr);
        let id = self.ctx.new_block();
        self.ctx.set_block(id, CpsBlock { id, params: vec![], instrs, term: cps_emit::emit_return(r) });
        if br != 0 { self.ctx.chain(cl, br)?; self.ctx.chain(cr, id)?; }
        else { self.ctx.chain(cl, id)?; }
        let entry = if bl != 0 { bl } else if br != 0 { br } else { id };
        Ok((entry, id, r))
    }

    fn build_unary(&mut self, op: &UnOp, right: &Expr) -> Result<(usize, usize, usize), String> {
        let (entry, continu, r) = self.build_expr(right)?;
        let dst = self.ctx.alloc();
        let unop = match op { UnOp::Neg => CpsUnOp::NegInt, UnOp::Not => CpsUnOp::Not };
        let (instrs, _) = cps_emit::emit_unary(dst, unop, r);
        let id = self.ctx.new_block();
        self.ctx.set_block(id, CpsBlock { id, params: vec![], instrs, term: cps_emit::emit_return(dst) });
        self.ctx.chain(continu, id)?;
        Ok((entry, id, dst))
    }

    fn build_block(&mut self, stmts: &[Stmt]) -> Result<(usize, usize, usize), String> {
        let mut first: Option<usize> = None;
        let mut last_reg: usize = 0;
        let mut last_continu: Option<usize> = None;
        for stmt in stmts {
            let (entry, continu, reg) = self.build_stmt(stmt)?;
            if entry == usize::MAX { continue; }
            if first.is_none() { first = Some(entry); }
            if let Some(t) = last_continu { self.ctx.chain(t, entry)?; }
            last_continu = Some(continu);
            last_reg = reg;
        }
        Ok((first.unwrap_or(0), last_continu.unwrap_or(0), last_reg))
    }

    fn build_if(&mut self, cond: &Expr, then_b: &Expr, else_b: Option<&Expr>) -> Result<(usize, usize, usize), String> {
        let (cond_entry, cond_continu, cond_reg) = self.build_expr(cond)?;
        let (then_entry, then_continu, then_reg) = self.build_expr(then_b)?;
        if let Some(eb) = else_b {
            let (else_entry, else_continu, else_reg) = self.build_expr(eb)?;
            let branch = self.ctx.new_block();
            self.ctx.set_block(branch, CpsBlock { id: branch, params: vec![], instrs: vec![],
                term: cps_emit::emit_branch(cond_reg, then_entry, else_entry) });
            let merge_reg = self.ctx.alloc();
            let merge = self.ctx.new_block();
            self.ctx.set_block(merge, CpsBlock { id: merge, params: vec![merge_reg], instrs: vec![],
                term: CpsTerminator::Return(merge_reg) });
            self.ctx.rewire_return_args(then_continu, merge, &[then_reg])?;
            self.ctx.rewire_return_args(else_continu, merge, &[else_reg])?;
            if cond_entry != 0 { self.ctx.chain(cond_continu, branch)?; }
            let entry = if cond_entry != 0 { cond_entry } else { branch };
            Ok((entry, merge, merge_reg))
        } else {
            let skip_block = self.ctx.new_block();
            self.ctx.set_block(skip_block, CpsBlock { id: skip_block, params: vec![], instrs: vec![],
                term: CpsTerminator::Return(0) });
            let branch = self.ctx.new_block();
            self.ctx.set_block(branch, CpsBlock { id: branch, params: vec![], instrs: vec![],
                term: cps_emit::emit_branch(cond_reg, then_entry, skip_block) });
            self.ctx.chain(then_continu, skip_block)?;
            if cond_entry != 0 { self.ctx.chain(cond_continu, branch)?; }
            let entry = if cond_entry != 0 { cond_entry } else { branch };
            Ok((entry, skip_block, then_reg))
        }
    }

    fn build_while(&mut self, cond: &Expr, body: &Expr) -> Result<(usize, usize, usize), String> {
        let loop_header = self.ctx.new_block();
        let body_block = self.ctx.new_block();
        let exit_block = self.ctx.new_block();
        self.ctx.loop_stack.push((loop_header, exit_block));

        let (cond_entry, cond_continu, cond_reg) = self.build_expr(cond)?;
        let (body_entry, body_continu, _) = self.build_expr(body)?;

        self.ctx.set_block(loop_header, CpsBlock { id: loop_header, params: vec![], instrs: vec![],
            term: cps_emit::emit_branch(cond_reg, body_block, exit_block) });
        self.ctx.set_block(body_block, CpsBlock { id: body_block, params: vec![], instrs: vec![],
            term: CpsTerminator::Jump(body_entry, vec![]) });
        self.ctx.set_block(exit_block, CpsBlock { id: exit_block, params: vec![], instrs: vec![],
            term: CpsTerminator::Return(0) });

        self.ctx.chain(cond_continu, loop_header)?;
        self.ctx.chain(body_continu, cond_entry)?;
        self.ctx.loop_stack.pop();
        Ok((cond_entry, exit_block, 0))
    }

    fn build_break(&mut self) -> Result<(usize, usize, usize), String> {
        let (_, brk) = self.ctx.loop_stack.last().copied().ok_or("break outside loop")?;
        let id = self.ctx.new_block();
        self.ctx.set_block(id, CpsBlock { id, params: vec![], instrs: vec![],
            term: CpsTerminator::Jump(brk, vec![]) });
        Ok((id, id, 0))
    }

    fn build_continue(&mut self) -> Result<(usize, usize, usize), String> {
        let (cont, _) = self.ctx.loop_stack.last().copied().ok_or("continue outside loop")?;
        let id = self.ctx.new_block();
        self.ctx.set_block(id, CpsBlock { id, params: vec![], instrs: vec![],
            term: CpsTerminator::Jump(cont, vec![]) });
        Ok((id, id, 0))
    }

    fn build_return(&mut self, val: Option<&Expr>) -> Result<(usize, usize, usize), String> {
        if let Some(v) = val {
            if let Expr::LitInt(n) = v {
                let r = self.ctx.alloc(); let c = self.add_const(Constant::Int(*n));
                let id = self.ctx.new_block();
                self.ctx.set_block(id, CpsBlock { id, params: vec![],
                    instrs: vec![CpsInstr::LoadConst(r, c)], term: cps_emit::emit_return(r) });
                return Ok((id, id, r))
            }
            let (entry, continu, r) = self.build_expr(v)?;
            let id = self.ctx.new_block();
            self.ctx.set_block(id, CpsBlock { id, params: vec![], instrs: vec![],
                term: cps_emit::emit_return(r) });
            self.ctx.chain(continu, id)?;
            Ok((entry, id, r))
        } else {
            let id = self.ctx.new_block();
            self.ctx.set_block(id, CpsBlock { id, params: vec![], instrs: vec![],
                term: CpsTerminator::Return(0) });
            Ok((id, id, 0))
        }
    }

    fn build_member(&mut self, object: &Expr, field: &str) -> Result<(usize, usize, usize), String> {
        let (entry, continu, obj_reg) = self.build_expr(object)?;
        let dst = self.ctx.alloc();
        // Find field index from struct definitions
        let fi = self.structs.iter()
            .flat_map(|s| s.fields.iter().enumerate())
            .find(|(_, (n, _))| n == field)
            .map(|(i, _)| i as u16)
            .unwrap_or(0);
        let (instrs, _) = cps_emit::emit_get_field(dst, obj_reg, fi);
        let id = self.ctx.new_block();
        self.ctx.set_block(id, CpsBlock { id, params: vec![], instrs, term: cps_emit::emit_return(dst) });
        self.ctx.chain(continu, id)?;
        Ok((entry, id, dst))
    }

    fn build_list(&mut self, items: &[Expr]) -> Result<(usize, usize, usize), String> {
        let mut entry = 0; let mut prev_c: Option<usize> = None;
        let mut regs = Vec::new();
        for item in items {
            let (e, c, r) = self.build_expr(item)?;
            if entry == 0 { entry = e; }
            if let Some(t) = prev_c { self.ctx.chain(t, e)?; }
            prev_c = Some(c); regs.push(r);
        }
        let dst = self.ctx.alloc();
        let (instrs, _) = cps_emit::emit_new_list(dst, regs);
        let id = self.ctx.new_block();
        self.ctx.set_block(id, CpsBlock { id, params: vec![], instrs, term: cps_emit::emit_return(dst) });
        if let Some(t) = prev_c { self.ctx.chain(t, id)?; }
        Ok((if entry != 0 { entry } else { id }, id, dst))
    }

    fn build_struct_lit(&mut self, struct_name: &str, fields: &[(String, Expr)]) -> Result<(usize, usize, usize), String> {
        let mut entry = 0; let mut prev_c: Option<usize> = None;
        let mut regs = Vec::new();
        for (_, val) in fields {
            let (e, c, r) = self.build_expr(val)?;
            if entry == 0 { entry = e; }
            if let Some(t) = prev_c { self.ctx.chain(t, e)?; }
            prev_c = Some(c); regs.push(r);
        }
        let dst = self.ctx.alloc();
        // Find struct ID
        let sid = self.structs.iter()
            .find(|s| s.name == struct_name)
            .map(|s| s.id)
            .unwrap_or(0);
        let mut instrs = vec![CpsInstr::NewStruct(dst, sid, regs.clone())];
        for (i, &reg) in regs.iter().enumerate() {
            instrs.push(CpsInstr::SetField(reg, dst, i as u16, 0));
        }
        let id = self.ctx.new_block();
        self.ctx.set_block(id, CpsBlock { id, params: vec![], instrs, term: cps_emit::emit_return(dst) });
        if let Some(t) = prev_c { self.ctx.chain(t, id)?; }
        Ok((if entry != 0 { entry } else { id }, id, dst))
    }

    fn build_index(&mut self, object: &Expr, index: &Expr) -> Result<(usize, usize, usize), String> {
        let (e1, c1, obj) = self.build_expr(object)?;
        let (e2, c2, idx) = self.build_expr(index)?;
        let dst = self.ctx.alloc();
        let (instrs, _) = cps_emit::emit_index_get(dst, obj, idx);
        let id = self.ctx.new_block();
        self.ctx.set_block(id, CpsBlock { id, params: vec![], instrs, term: cps_emit::emit_return(dst) });
        self.ctx.chain(c1, e2)?;
        self.ctx.chain(c2, id)?;
        Ok((e1, id, dst))
    }

    fn build_assign(&mut self, target: &Expr, value: &Expr) -> Result<(usize, usize, usize), String> {
        let (val_entry, val_continu, val_reg) = self.build_expr(value)?;
        let target_reg = if let Expr::VarRef(name) = target {
            if let Some(&reg) = self.ctx.var_map.get(name) { reg } else { self.ctx.alloc() }
        } else { self.ctx.alloc() };
        let id = self.ctx.new_block();
        self.ctx.set_block(id, CpsBlock { id, params: vec![],
            instrs: vec![CpsInstr::Move(target_reg, val_reg)],
            term: cps_emit::emit_return(target_reg) });
        self.ctx.chain(val_continu, id)?;
        Ok((val_entry, id, target_reg))
    }
}

fn bin_op_to_cps(op: BinOp) -> CpsBinOp {
    match op {
        BinOp::Add => CpsBinOp::AddInt, BinOp::Sub => CpsBinOp::SubInt,
        BinOp::Mul => CpsBinOp::MulInt, BinOp::Div => CpsBinOp::DivInt,
        BinOp::Mod => CpsBinOp::ModInt,
        BinOp::Eq => CpsBinOp::EqInt, BinOp::Ne => CpsBinOp::NeInt,
        BinOp::Lt => CpsBinOp::LtInt, BinOp::Le => CpsBinOp::LeInt,
        BinOp::Gt => CpsBinOp::GtInt, BinOp::Ge => CpsBinOp::GeInt,
        _ => CpsBinOp::AddInt,
    }
}

fn is_heap_type(ty: &TypeExpr) -> bool {
    match ty {
        TypeExpr::Named(name) => {
            name == "String" || name == "List"
                || name.chars().next().map_or(false, |c| c.is_uppercase())
        }
        TypeExpr::List(_) => true,
        TypeExpr::Arrow { .. } => false,
    }
}

fn remap_term_ids(block: &mut CpsBlock, map: &HashMap<usize, usize>) {
    match &mut block.term {
        CpsTerminator::Jump(b, _) => { if let Some(&n) = map.get(b) { *b = n; } }
        CpsTerminator::Branch(_, tb, _, fb, _) => {
            if let Some(&n) = map.get(tb) { *tb = n; }
            if let Some(&n) = map.get(fb) { *fb = n; }
        }
        CpsTerminator::Call(_, _, ret) => { if let Some(&n) = map.get(ret) { *ret = n; } }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaubo_syntax::parser::Parser;

    fn build_src(src: &str) -> CpsModule {
        let m = Parser::new(src).parse().unwrap();
        build_module(&m).unwrap()
    }

    #[test] fn build_single_const() { let c = build_src("const x = 42;"); assert!(c.functions[0].blocks.len() >= 2); }
    #[test] fn build_two_consts() { let c = build_src("const x = 10; const y = 32;"); assert!(c.functions[0].blocks.len() >= 3); }
    #[test] fn build_var() { let c = build_src("var x = 10; const y = x;"); assert!(c.functions[0].blocks.len() >= 2); }
    #[test] fn build_multi_var() { let c = build_src("var x = 10; var y = 32; const z = x + y;"); assert!(c.functions[0].blocks.len() >= 5); }
    #[test] fn build_if_else() { let c = build_src("const x = if true { 1 } else { 2 };"); assert!(c.functions[0].blocks.len() >= 4); }
    #[test] fn build_while_struct() { let c = build_src("var n = 0; while n < 3 { n = n + 1; };"); assert!(c.functions[0].blocks.len() >= 3); }
    #[test] fn build_block() { let c = build_src("const r = { var x = 1; x + 1; };"); assert!(c.functions[0].blocks.len() >= 2); }

    #[test]
    fn build_lambda_creates_separate_function() {
        let c = build_src("const f = |x| { x + 1 };");
        assert!(c.functions.len() >= 2, "lambda should create separate function, got {}", c.functions.len());
    }

    #[test]
    fn build_lambda_call_emits_call_terminator() {
        let c = build_src("const f = |x| { x + 1 }; f(41);");
        // The main function should have a block with Call terminator
        let main = c.functions.last().unwrap();
        let has_call = main.blocks.iter().any(|b| matches!(b.term, CpsTerminator::Call(..)));
        assert!(has_call, "main function should contain a Call terminator");
    }

    #[test]
    fn build_lambda_with_while_body() {
        let c = build_src("const f = |n| { while n > 0 { n = n - 1; } }; f(5);");
        assert!(c.functions.len() >= 2);
    }

    #[test]
    fn build_list_not_empty() {
        let c = build_src("const xs = [1, 2, 3];");
        assert!(!c.constants.is_empty());
        assert!(c.functions[0].blocks.len() >= 2);
    }

    #[test]
    fn build_async_ok() {
        let c = build_src("const f = async |x| { x + 1 };");
        assert!(c.functions.len() >= 2);
    }

    #[test]
    fn build_to_string_emits_itos() {
        let c = build_src("const s = 42.to_string();");
        let main = c.functions.last().unwrap();
        let has_itos = main.blocks.iter().any(|b|
            b.instrs.iter().any(|i| matches!(i, CpsInstr::BinOp(_, CpsBinOp::IToS, _, _)))
        );
        assert!(has_itos, "42.to_string() should emit IToS instruction");
    }

    #[test]
    fn build_print_int_handled() {
        let c = build_src("print(42.to_string());");
        let main = c.functions.last().unwrap();
        let has_print = main.blocks.iter().any(|b|
            b.instrs.iter().any(|i| matches!(i, CpsInstr::Print(_)))
        );
        assert!(has_print, "print(42.to_string()) should emit Print instruction");
    }

    #[test]
    fn build_lambda_var_return() {
        let c = build_src("const f = |x| { var r = 42; return r; }; f(0);");
        assert!(c.functions.len() >= 2, "should have main + lambda");
        let lambda = &c.functions[c.functions.len() - 2]; // lambda before main
        let has_loadconst = lambda.blocks.iter().any(|b|
            b.instrs.iter().any(|i| matches!(i, CpsInstr::LoadConst(_, _)))
        );
        assert!(has_loadconst, "lambda should have LoadConst for var r = 42");
    }

    #[test]
    fn build_lambda_print_literal() {
        let c = build_src("const f = |x| { print(\"hi\"); return x; }; f(0);");
        assert!(c.functions.len() >= 2);
        let lambda = &c.functions[c.functions.len() - 2];
        let has_print = lambda.blocks.iter().any(|b|
            b.instrs.iter().any(|i| matches!(i, CpsInstr::Print(_)))
        );
        assert!(has_print, "lambda should contain Print instruction");
    }

    #[test]
    fn build_lambda_to_string() {
        let c = build_src("const f = |x| { print(x.to_string()); return x; }; f(99);");
        assert!(c.functions.len() >= 2);
        let lambda = &c.functions[c.functions.len() - 2];
        let has_itos = lambda.blocks.iter().any(|b|
            b.instrs.iter().any(|i| matches!(i, CpsInstr::BinOp(_, CpsBinOp::IToS, _, _)))
        );
        assert!(has_itos, "lambda should contain IToS for x.to_string()");
    }

    #[test]
    fn build_lambda_while_loop() {
        let c = build_src("const f = |n| { var i = 0; while i < n { i = i + 1; }; return i; }; f(5);");
        assert!(c.functions.len() >= 2);
        let lambda = &c.functions[c.functions.len() - 2];
        assert!(lambda.blocks.len() >= 4, "while should create header+body+exit+cond blocks, got {}", lambda.blocks.len());
    }
}
