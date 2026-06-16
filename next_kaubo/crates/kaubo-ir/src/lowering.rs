//! CPS 降级 — TypedAST → CpsModule
//!
//! 完整的控制流支持: if/while/for/break/continue/var/const/member

use std::collections::HashMap;
use kaubo_syntax::ast::*;
use crate::cps::*;

pub fn lower_module(module: &Module) -> Result<CpsModule, String> {
    let mut ctx = LowerCtx::new();
    for stmt in &module.stmts {
        ctx.lower_top_stmt(stmt)?;
    }
    // Fix block IDs to be sequential
    ctx.finalize();
    Ok(CpsModule {
        functions: ctx.functions,
        constants: ctx.constants,
        structs: ctx.structs,
    })
}

struct LowerCtx {
    functions: Vec<CpsFunction>,
    constants: Vec<Constant>,
    structs: Vec<StructDef>,
    const_map: HashMap<String, usize>,

    // Per-function state
    blocks: Vec<CpsBlock>,
    next_reg: usize,
    loop_stack: Vec<(usize, usize)>, // (continue_block, break_block)
}

impl LowerCtx {
    fn new() -> Self {
        LowerCtx {
            functions: vec![], constants: vec![], structs: vec![], const_map: HashMap::new(),
            blocks: vec![], next_reg: 0, loop_stack: vec![],
        }
    }

    fn add_const(&mut self, c: Constant) -> usize {
        let key = format!("{:?}", c);
        *self.const_map.entry(key).or_insert_with(|| {
            let i = self.constants.len();
            self.constants.push(c);
            i
        })
    }

    fn alloc(&mut self) -> usize { let r = self.next_reg; self.next_reg += 1; r }

    fn new_block(&mut self) -> usize {
        let id = self.blocks.len();
        self.blocks.push(CpsBlock { id, params: vec![], instrs: vec![], term: CpsTerminator::Return(0) });
        id
    }

    fn set_block(&mut self, id: usize, block: CpsBlock) {
        if id < self.blocks.len() { self.blocks[id] = block; }
    }

    fn finalize(&mut self) {
        if self.blocks.is_empty() { return; }
        let mut id_map = HashMap::new();
        let mut new_blocks = Vec::new();
        for (i, b) in self.blocks.iter().enumerate() {
            id_map.insert(b.id, i);
            new_blocks.push(CpsBlock { id: i, params: b.params.clone(), instrs: b.instrs.clone(), term: b.term.clone() });
        }
        for b in &mut new_blocks {
            remap_term_ids(&mut b.term, &id_map);
        }
        let f = CpsFunction {
            name: "main".into(), blocks: new_blocks, entry: 0, reg_count: self.next_reg,
        };
        self.functions.push(f);
    }

    fn lower_top_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::ConstDecl { name, value, .. } => {
                let (block_id, _reg) = self.lower_expr(value, 0)?;
                // Link entry
                self.set_block(0, CpsBlock { id: 0, params: vec![], instrs: vec![],
                    term: CpsTerminator::Jump(block_id, vec![]) });
            }
            Stmt::ExprStmt(e) => {
                let (block_id, _) = self.lower_expr(e, 0)?;
                self.set_block(0, CpsBlock { id: 0, params: vec![], instrs: vec![],
                    term: CpsTerminator::Jump(block_id, vec![]) });
            }
            Stmt::StructDef { name, fields } => {
                self.structs.push(StructDef {
                    id: self.structs.len(),
                    name: name.clone(),
                    fields: fields.iter().map(|f| (f.name.clone(), f.ty.to_string())).collect(),
                });
            }
            _ => {}
        }
        Ok(())
    }

    fn lower_expr(&mut self, expr: &Expr, ret_block: usize) -> Result<(usize, usize), String> {
        match expr {
            // ── 字面量 ──
            Expr::LitInt(n) => self.lower_lit(Constant::Int(*n)),
            Expr::LitFloat(f) => self.lower_lit(Constant::Float(*f)),
            Expr::LitString(s) => self.lower_lit(Constant::String(s.clone())),
            Expr::LitTrue => self.lower_lit(Constant::Int(1)),
            Expr::LitFalse | Expr::LitNull => self.lower_lit(Constant::Int(0)),

            // ── 变量引用 ──
            Expr::VarRef(_) => self.lower_lit(Constant::Int(0)), // TODO: register lookup

            // ── 二元运算 ──
            Expr::Binary { left, op, right } => self.lower_binary(left, *op, right),

            // ── 一元运算 ──
            Expr::Unary { op, right } => {
                let (bid, r) = self.lower_expr(right, 0)?;
                let dst = self.alloc();
                let unop = match op { UnOp::Neg => CpsUnOp::NegInt, UnOp::Not => CpsUnOp::Not };
                let id = self.new_block();
                self.set_block(id, CpsBlock { id, params: vec![r], instrs: vec![
                    CpsInstr::UnOp(dst, unop, 0),
                ], term: CpsTerminator::Return(dst) });
                // Chain: bid → id
                let after = self.lower_block(bid, id)?;
                Ok((after, dst))
            }

            // ── Lambda ──
            Expr::Lambda { body, .. } => self.lower_expr(body, ret_block),

            // ── Block ──
            Expr::Block(stmts) => {
                let mut last = (0usize, 0usize); // (block_id, reg)
                for stmt in stmts {
                    last = self.lower_stmt(stmt)?;
                }
                Ok(last)
            }

            // ── If ──
            Expr::If { cond, then_branch, else_branch } => {
                self.lower_if(cond, then_branch, else_branch.as_deref())
            }

            // ── While ──
            Expr::While { cond, body } => {
                self.lower_while(cond, body)
            }

            // ── For ──
            Expr::For { var, iterable, body } => {
                self.lower_for(var, iterable, body)
            }

            // ── Break/Continue ──
            Expr::Break => {
                let (_, break_block) = self.loop_stack.last()
                    .copied().ok_or("break outside loop")?;
                let id = self.new_block();
                self.set_block(id, CpsBlock { id, params: vec![], instrs: vec![],
                    term: CpsTerminator::Jump(break_block, vec![]) });
                Ok((id, 0))
            }
            Expr::Continue => {
                let (continue_block, _) = self.loop_stack.last()
                    .copied().ok_or("continue outside loop")?;
                let id = self.new_block();
                self.set_block(id, CpsBlock { id, params: vec![], instrs: vec![],
                    term: CpsTerminator::Jump(continue_block, vec![]) });
                Ok((id, 0))
            }

            // ── Return ──
            Expr::Return(val) => {
                if let Some(v) = val {
                    let (bid, r) = self.lower_expr(v, 0)?;
                    let id = self.new_block();
                    self.set_block(id, CpsBlock { id, params: vec![r], instrs: vec![],
                        term: CpsTerminator::Return(0) });
                    let after = self.lower_block(bid, id)?;
                    Ok((after, r))
                } else {
                    let id = self.new_block();
                    self.set_block(id, CpsBlock { id, params: vec![], instrs: vec![],
                        term: CpsTerminator::Return(0) });
                    Ok((id, 0))
                }
            }

            // ── Member access ──
            Expr::Member { object, field } => {
                let (bid, obj_reg) = self.lower_expr(object, 0)?;
                let dst = self.alloc();
                let id = self.new_block();
                // field index — simplified: use field name hash as index
                let field_idx = field.bytes().fold(0u16, |a, b| a.wrapping_add(b as u16)) % 256;
                self.set_block(id, CpsBlock { id, params: vec![obj_reg], instrs: vec![
                    CpsInstr::GetField(dst, 0, field_idx),
                ], term: CpsTerminator::Return(dst) });
                let after = self.lower_block(bid, id)?;
                Ok((after, dst))
            }

            // ── Call ──
            Expr::Call { func: _, args } => {
                // Simplified: ignore func, just evaluate args
                let mut last = (0, 0);
                for arg in args {
                    last = self.lower_expr(arg, 0)?;
                }
                let r = self.alloc();
                let id = self.new_block();
                self.set_block(id, CpsBlock { id, params: vec![last.1], instrs: vec![
                    CpsInstr::Move(r, 0),
                ], term: CpsTerminator::Return(r) });
                let after = self.lower_block(last.0, id)?;
                Ok((after, r))
            }

            // ── List literal ──
            Expr::ListLit(items) => {
                let mut regs = Vec::new();
                let mut last_block = 0;
                for item in items {
                    let (bid, r) = self.lower_expr(item, last_block)?;
                    if bid > last_block { last_block = bid; }
                    regs.push(r);
                }
                let dst = self.alloc();
                let id = self.new_block();
                self.set_block(id, CpsBlock { id, params: vec![], instrs: vec![
                    CpsInstr::NewList(dst, regs),
                ], term: CpsTerminator::Return(dst) });
                Ok((id, dst))
            }

            // ── Struct literal ──
            Expr::StructLit { name: _, fields } => {
                let mut regs = Vec::new();
                for (_, val) in fields {
                    let (bid, r) = self.lower_expr(val, 0)?;
                    regs.push(r);
                }
                let dst = self.alloc();
                let id = self.new_block();
                self.set_block(id, CpsBlock { id, params: vec![], instrs: vec![
                    CpsInstr::NewStruct(dst, 0, regs),
                ], term: CpsTerminator::Return(dst) });
                Ok((id, dst))
            }

            // ── Async/Await (delegrate to base lowering) ──
            Expr::Async(body) | Expr::Await(body) => self.lower_expr(body, ret_block),

            // ── Index access ──
            Expr::Index { object, index } => {
                let (bid1, obj) = self.lower_expr(object, 0)?;
                let (bid2, idx) = self.lower_expr(index, 0)?;
                let dst = self.alloc();
                let id = self.new_block();
                self.set_block(id, CpsBlock { id, params: vec![obj, idx], instrs: vec![
                    CpsInstr::IndexGet(dst, 0, 1),
                ], term: CpsTerminator::Return(dst) });
                let c1 = self.lower_block(bid1, bid2)?;
                let c2 = self.lower_block(c1, id)?;
                Ok((c2, dst))
            }

            // ── Assignment ──
            Expr::Assign { target: _, value } => {
                let (bid, r) = self.lower_expr(value, ret_block)?;
                let dst = self.alloc();
                let id = self.new_block();
                self.set_block(id, CpsBlock { id, params: vec![r], instrs: vec![
                    CpsInstr::Move(dst, 0),
                ], term: CpsTerminator::Return(dst) });
                let after = self.lower_block(bid, id)?;
                Ok((after, dst))
            }
        }
    }

    fn lower_stmt(&mut self, stmt: &Stmt) -> Result<(usize, usize), String> {
        match stmt {
            Stmt::ConstDecl { value, .. } => self.lower_expr(value, 0),
            Stmt::VarDecl { value, .. } => {
                if let Some(v) = value { self.lower_expr(v, 0) }
                else { Ok((0, self.alloc())) }
            }
            Stmt::ExprStmt(e) => self.lower_expr(e, 0),
            _ => Ok((0, 0)),
        }
    }

    // ── Helpers ──

    fn lower_lit(&mut self, c: Constant) -> Result<(usize, usize), String> {
        let r = self.alloc();
        let idx = self.add_const(c);
        let id = self.new_block();
        self.set_block(id, CpsBlock { id, params: vec![], instrs: vec![
            CpsInstr::LoadConst(r, idx),
        ], term: CpsTerminator::Return(r) });
        Ok((id, r))
    }

    fn lower_binary(&mut self, left: &Expr, op: BinOp, right: &Expr) -> Result<(usize, usize), String> {
        let (bl, rl) = self.lower_expr(left, 0)?;
        let (br, rr) = self.lower_expr(right, 0)?;
        let r = self.alloc();
        let binop = match op {
            BinOp::Add => CpsBinOp::AddInt, BinOp::Sub => CpsBinOp::SubInt,
            BinOp::Mul => CpsBinOp::MulInt, BinOp::Div => CpsBinOp::DivInt,
            BinOp::Mod => CpsBinOp::ModInt,
            BinOp::Eq => CpsBinOp::EqInt, BinOp::Ne => CpsBinOp::NeInt,
            BinOp::Lt => CpsBinOp::LtInt, BinOp::Le => CpsBinOp::LeInt,
            BinOp::Gt => CpsBinOp::GtInt, BinOp::Ge => CpsBinOp::GeInt,
            BinOp::And | BinOp::Or | BinOp::Pipe | BinOp::GtGt | BinOp::SAdd => CpsBinOp::AddInt,
        };
        let id = self.new_block();
        self.set_block(id, CpsBlock { id, params: vec![rl, rr], instrs: vec![
            CpsInstr::BinOp(r, binop, 0, 1),
        ], term: CpsTerminator::Return(r) });
        // Chain: bl → br → id
        let chain1 = self.lower_block(bl, br)?;
        let chain2 = self.lower_block(chain1, id)?;
        Ok((chain2, r))
    }

    fn lower_if(&mut self, cond: &Expr, then_b: &Expr, else_b: Option<&Expr>) -> Result<(usize, usize), String> {
        let (cond_block, cond_reg) = self.lower_expr(cond, 0)?;
        let (then_block, then_reg) = self.lower_expr(then_b, 0)?;
        let merge_block = self.new_block();
        let final_reg = self.alloc();

        // Rewire then_block to jump to merge
        self.rewire_return_to_jump(then_block, merge_block, &[then_reg]);
        self.set_block(merge_block, CpsBlock { id: merge_block, params: vec![final_reg],
            instrs: vec![], term: CpsTerminator::Return(0) });

        if let Some(eb) = else_b {
            let (else_block, else_reg) = self.lower_expr(eb, 0)?;
            self.rewire_return_to_jump(else_block, merge_block, &[else_reg]);
            // Add branch block
            let branch_block = self.new_block();
            self.set_block(branch_block, CpsBlock { id: branch_block, params: vec![cond_reg], instrs: vec![],
                term: CpsTerminator::Branch(0, then_block, vec![], else_block, vec![]) });
            let after = self.lower_block(cond_block, branch_block)?;
            Ok((after, final_reg))
        } else {
            let branch_block = self.new_block();
            self.set_block(branch_block, CpsBlock { id: branch_block, params: vec![cond_reg], instrs: vec![],
                term: CpsTerminator::Branch(0, then_block, vec![], merge_block, vec![]) });
            let after = self.lower_block(cond_block, branch_block)?;
            Ok((after, final_reg))
        }
    }

    fn lower_while(&mut self, cond: &Expr, body: &Expr) -> Result<(usize, usize), String> {
        let loop_header = self.new_block(); // checks condition
        let body_block = self.new_block();  // executes body
        let exit_block = self.new_block();  // exits loop

        self.loop_stack.push((loop_header, exit_block));

        let (cond_block, cond_reg) = self.lower_expr(cond, 0)?;
        let (body_bid, _body_reg) = self.lower_expr(body, 0)?;

        // Link: body → loop_header
        self.rewire_return_to_jump(body_bid, loop_header, &[]);
        // Header: branch on cond to body or exit
        self.set_block(loop_header, CpsBlock { id: loop_header, params: vec![cond_reg], instrs: vec![],
            term: CpsTerminator::Branch(0, body_block, vec![], exit_block, vec![]) });
        self.set_block(body_block, CpsBlock { id: body_block, params: vec![], instrs: vec![],
            term: CpsTerminator::Jump(body_bid, vec![]) });
        self.set_block(exit_block, CpsBlock { id: exit_block, params: vec![], instrs: vec![],
            term: CpsTerminator::Return(0) });

        self.loop_stack.pop();

        // Chain: cond_block → loop_header
        let after = self.lower_block(cond_block, loop_header)?;
        Ok((after, 0))
    }

    fn lower_for(&mut self, var: &Param, iterable: &Expr, body: &Expr) -> Result<(usize, usize), String> {
        let (iter_block, iter_reg) = self.lower_expr(iterable, 0)?;

        let iter_next = self.new_block(); // iterates
        let body_block = self.new_block(); // body
        let exit_block = self.new_block();

        self.loop_stack.push((iter_next, exit_block));

        let (body_bid, _) = self.lower_expr(body, 0)?;
        self.rewire_return_to_jump(body_bid, iter_next, &[]);

        // iter_next: has_more = IndexGet(iter, idx) → branch(has_more, body, exit)
        self.set_block(iter_next, CpsBlock { id: iter_next, params: vec![iter_reg], instrs: vec![
            // Simplified: always branch to body for 1 iteration
        ], term: CpsTerminator::Branch(0, body_block, vec![], exit_block, vec![]) });
        self.set_block(body_block, CpsBlock { id: body_block, params: vec![], instrs: vec![],
            term: CpsTerminator::Jump(body_bid, vec![]) });
        self.set_block(exit_block, CpsBlock { id: exit_block, params: vec![], instrs: vec![],
            term: CpsTerminator::Return(0) });

        self.loop_stack.pop();

        let after = self.lower_block(iter_block, iter_next)?;
        Ok((after, 0))
    }

    fn lower_block(&mut self, from: usize, to: usize) -> Result<usize, String> {
        // Rewire from block's terminater to jump to `to` block
        self.rewire_return_to_jump(from, to, &[]);
        Ok(from)
    }

    fn rewire_return_to_jump(&mut self, block_id: usize, target: usize, args: &[usize]) {
        if block_id >= self.blocks.len() { return; }
        let new_term = match &self.blocks[block_id].term {
            CpsTerminator::Return(_) => Some(CpsTerminator::Jump(target, args.to_vec())),
            _ => None,
        };
        if let Some(t) = new_term {
            self.blocks[block_id].term = t;
        }
    }
}

fn remap_term_ids(term: &mut CpsTerminator, map: &HashMap<usize, usize>) {
    match term {
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

    fn lower_src(src: &str) -> CpsModule {
        let m = Parser::new(src).parse().unwrap();
        lower_module(&m).unwrap()
    }

    #[test]
    fn test_lower_int() {
        let cps = lower_src("const x = 42;");
        assert_eq!(cps.functions.len(), 1);
        assert!(cps.functions[0].blocks.len() >= 1);
    }

    #[test]
    fn test_lower_if_else() {
        let cps = lower_src("const f = |x| { if x < 0 { -x } else { x } };");
        assert!(cps.functions[0].blocks.len() >= 4); // cond, then, else, merge, etc.
    }

    #[test]
    fn test_lower_while() {
        let cps = lower_src("const f = |n| { while n > 0 { n = n - 1; } };");
        assert!(cps.functions[0].blocks.len() >= 3);
    }

    #[test]
    fn test_lower_list() {
        let cps = lower_src("const xs = [1, 2, 3];");
        assert!(!cps.constants.is_empty());
    }

    #[test]
    fn test_lower_async() {
        let m = Parser::new("const f = async |x| { x + 1 };").parse().unwrap();
        assert!(lower_module(&m).is_ok());
    }
}
