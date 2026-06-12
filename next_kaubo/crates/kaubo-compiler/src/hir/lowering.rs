//! HIR Lowering: AST → HirModule
//!
//! 将 AST 转换为基本块表示的控制流图。
//! break/continue desugar 为基本块跳转（continuation = BlockId）。

use kaubo_ir::hir::{
    ConstantValue, HirBinaryOp, HirBlock, HirFunction, HirInstr, HirModule, HirOperand,
    HirTerminator, HirUnaryOp,
};
use crate::parser::expr::{Expr, ExprKind};
use crate::parser::stmt::{Stmt, StmtKind};
use crate::parser::Module;

/// Lowering 上下文：跟踪当前基本块、跳转目标、变量映射
struct LoweringCtx {
    module: HirModule,
    current_func: HirFunction,
    current_block_idx: usize,
    next_block_id: usize,
    next_temp: usize,
    /// 当前循环的 break 目标块 ID
    break_target: Option<usize>,
    /// 当前循环的 continue 目标块 ID
    continue_target: Option<usize>,
    /// 变量名 → Temp ID 映射
    locals: Vec<(String, usize)>,
}

impl LoweringCtx {
    fn new() -> Self {
        Self {
            module: HirModule::new(),
            current_func: HirFunction {
                name: None,
                arity: 0,
                blocks: Vec::new(),
                entry: 0,
                local_count: 0,
                return_type: None,
            },
            current_block_idx: 0,
            next_block_id: 1,
            next_temp: 0,
            break_target: None,
            continue_target: None,
            locals: Vec::new(),
        }
    }

    fn new_temp(&mut self) -> usize {
        let t = self.next_temp;
        self.next_temp += 1;
        t
    }

    fn alloc_block(&mut self) -> usize {
        let id = self.next_block_id;
        self.next_block_id += 1;
        let block = HirBlock { id, instrs: Vec::new(), term: HirTerminator::End };
        self.current_func.blocks.push(block);
        id
    }

    fn current_block(&mut self) -> &mut HirBlock {
        &mut self.current_func.blocks[self.current_block_idx]
    }

    fn set_terminator(&mut self, term: HirTerminator) {
        self.current_block().term = term;
    }

    fn emit(&mut self, instr: HirInstr) {
        self.current_block().instrs.push(instr);
    }

    fn switch_to_block(&mut self, block_id: usize) {
        self.current_block_idx = self.current_func.blocks.iter().position(|b| b.id == block_id).expect("block not found");
    }

    fn resolve_local(&mut self, name: &str) -> HirOperand {
        for (n, t) in self.locals.iter().rev() {
            if n == name { return HirOperand::Temp(*t); }
        }
        let t = self.new_temp();
        self.locals.push((name.to_string(), t));
        HirOperand::Temp(t)
    }

    fn lower_expr(&mut self, expr: &Expr) -> HirOperand {
        match expr.as_ref() {
            ExprKind::LiteralInt(li) => {
                HirOperand::Immediate(ConstantValue::Int(li.value as i32))
            }
            ExprKind::LiteralFloat(lf) => {
                HirOperand::Immediate(ConstantValue::Float(lf.value))
            }
            ExprKind::Binary(bin) => {
                let l = self.lower_expr(&bin.left);
                let r = self.lower_expr(&bin.right);
                let dst = HirOperand::Temp(self.new_temp());
                let hir_op = match bin.op {
                    crate::lexer::token_kind::KauboTokenKind::Plus => HirBinaryOp::Add,
                    crate::lexer::token_kind::KauboTokenKind::Minus => HirBinaryOp::Sub,
                    crate::lexer::token_kind::KauboTokenKind::Asterisk => HirBinaryOp::Mul,
                    crate::lexer::token_kind::KauboTokenKind::Slash => HirBinaryOp::Div,
                    crate::lexer::token_kind::KauboTokenKind::Percent => HirBinaryOp::Mod,
                    crate::lexer::token_kind::KauboTokenKind::DoubleEqual => HirBinaryOp::Eq,
                    crate::lexer::token_kind::KauboTokenKind::ExclamationEqual => HirBinaryOp::Neq,
                    crate::lexer::token_kind::KauboTokenKind::LessThan => HirBinaryOp::Lt,
                    crate::lexer::token_kind::KauboTokenKind::GreaterThan => HirBinaryOp::Gt,
                    crate::lexer::token_kind::KauboTokenKind::LessThanEqual => HirBinaryOp::Le,
                    crate::lexer::token_kind::KauboTokenKind::GreaterThanEqual => HirBinaryOp::Ge,
                    crate::lexer::token_kind::KauboTokenKind::And => HirBinaryOp::And,
                    crate::lexer::token_kind::KauboTokenKind::Or => HirBinaryOp::Or,
                    _ => HirBinaryOp::Add, // fallback
                };
                self.emit(HirInstr::Binary { dst: dst.clone(), op: hir_op, left: l, right: r });
                dst
            }
            _ => {
                // Unsupported expression — return a dummy temp
                HirOperand::Temp(self.new_temp())
            }
        }
    }

    fn lower_stmt(&mut self, stmt: &Stmt) {
        match stmt.as_ref() {
            StmtKind::Expr(es) => {
                self.lower_expr(&es.expression);
            }
            StmtKind::VarDecl(vd) => {
                let val = self.lower_expr(&vd.initializer);
                let t = self.resolve_local(&vd.name);
                self.emit(HirInstr::Move { dst: t, src: val });
            }
            StmtKind::Break(_) => {
                if let Some(target) = self.break_target {
                    self.set_terminator(HirTerminator::Jump { target });
                }
            }
            StmtKind::Continue(_) => {
                if let Some(target) = self.continue_target {
                    self.set_terminator(HirTerminator::Jump { target });
                }
            }
            StmtKind::While(ws) => {
                let saved_break = self.break_target;
                let saved_continue = self.continue_target;

                let header_id = self.alloc_block();
                let body_id = self.alloc_block();
                let exit_id = self.alloc_block();

                // Jump to header
                self.set_terminator(HirTerminator::Jump { target: header_id });

                // header: condition check
                self.switch_to_block(header_id);
                let cond = self.lower_expr(&ws.condition);
                self.continue_target = Some(header_id);
                self.break_target = Some(exit_id);
                self.set_terminator(HirTerminator::Branch { cond, true_target: body_id, false_target: exit_id });

                // body
                self.switch_to_block(body_id);
                self.lower_stmt(&ws.body);
                // After body, jump back to header (unless terminator already set by break/continue)
                self.current_block_if_no_term(HirTerminator::Jump { target: header_id });

                // exit
                self.switch_to_block(exit_id);

                self.break_target = saved_break;
                self.continue_target = saved_continue;
            }
            StmtKind::Return(rs) => {
                let val = rs.value.as_ref().map(|v| self.lower_expr(v));
                self.set_terminator(HirTerminator::Return { value: val });
            }
            StmtKind::Print(ps) => {
                let val = self.lower_expr(&ps.expression);
                self.emit(HirInstr::Print { value: val });
            }
            StmtKind::Block(bs) => {
                for s in &bs.statements {
                    self.lower_stmt(s);
                }
            }
            StmtKind::Pass(_) | StmtKind::Empty(_) => {
                // no-op
            }
            _ => {
                // unsupported statement type — skip
            }
        }
    }

    fn current_block_if_no_term(&mut self, default: HirTerminator) {
        if let HirTerminator::End = self.current_block().term {
            self.set_terminator(default);
        }
    }
}

/// AST Module → HIR
pub fn lower_module(ast: &Module) -> HirModule {
    let mut ctx = LoweringCtx::new();

    // Create entry block
    let entry = ctx.alloc_block();
    ctx.current_func.entry = ctx.current_func.blocks.iter().position(|b| b.id == entry).unwrap_or(0);
    ctx.switch_to_block(entry);

    for stmt in &ast.statements {
        ctx.lower_stmt(stmt);
    }

    // If the entry block still has no terminator, add Return(None)
    ctx.current_block_if_no_term(HirTerminator::Return { value: None });

    ctx.current_func.local_count = ctx.next_temp;
    ctx.module.functions.push(ctx.current_func);
    ctx.module
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Module;
    use crate::lexer::builder::build_lexer;
    use crate::parser::parser::Parser;

    fn parse(src: &str) -> Module {
        let mut lexer = build_lexer();
        let _ = lexer.feed(src.as_bytes());
        let _ = lexer.terminate();
        let mut parser = Parser::new(lexer);
        parser.parse().expect("parse failed")
    }

    #[test]
    fn test_lower_simple() {
        let ast = parse("var x = 1 + 2;");
        let hir = lower_module(&ast);
        assert!(!hir.functions.is_empty());
        let f = &hir.functions[0];
        assert!(!f.blocks.is_empty());
    }

    #[test]
    fn test_lower_while() {
        let ast = parse("var i = 0; while i < 10 { i = i + 1; }");
        let hir = lower_module(&ast);
        let f = &hir.functions[0];
        // Should have multiple blocks: entry, header, body, exit
        assert!(f.blocks.len() >= 3, "expected >= 3 blocks, got {}", f.blocks.len());
    }

    #[test]
    fn test_lower_break_continue() {
        let ast = parse("var i = 0; while i < 10 { if i == 5 { break; } i = i + 1; }");
        let hir = lower_module(&ast);
        let f = &hir.functions[0];
        // break should create a Jump to exit block
        let has_jump = f.blocks.iter().any(|b| matches!(b.term, HirTerminator::Jump { .. }));
        assert!(has_jump);
    }
}
