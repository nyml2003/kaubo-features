//! 优化 Pass 框架
//!
//! 每个优化 pass 是纯函数 `HirModule → HirModule`。
//! Pass 之间通过 pipeline 链式执行。

use super::super::models::hir::HirModule;

// ============================================================
// OptimizationPass trait
// ============================================================

/// 优化 Pass 接口：纯函数，输入 HIR 模块，输出优化后的 HIR 模块。
pub trait OptimizationPass: Send + Sync {
    /// Pass 名称（用于调试和日志）
    fn name(&self) -> &'static str;

    /// 执行优化
    fn run(&self, module: HirModule) -> Result<HirModule, String>;
}

// ============================================================
// Pass Pipeline
// ============================================================

/// 优化管道：按顺序执行多个 pass
pub struct OptimizationPipeline {
    passes: Vec<Box<dyn OptimizationPass>>,
}

impl OptimizationPipeline {
    pub fn new() -> Self {
        Self {
            passes: Vec::new(),
        }
    }

    /// 添加一个 pass
    pub fn add_pass(mut self, pass: Box<dyn OptimizationPass>) -> Self {
        self.passes.push(pass);
        self
    }

    /// 运行全部 pass
    pub fn run(&self, mut module: HirModule) -> Result<HirModule, String> {
        for pass in &self.passes {
            module = pass.run(module)?;
        }
        Ok(module)
    }

    /// 返回 pass 数量
    pub fn len(&self) -> usize {
        self.passes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.passes.is_empty()
    }
}

impl Default for OptimizationPipeline {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 工厂函数：为 HIR 模块构建默认优化管道
// ============================================================

/// 构建默认优化管道（包含所有通用优化 pass）
pub fn create_default_pipeline() -> OptimizationPipeline {
    OptimizationPipeline::new()
        .add_pass(Box::new(ConstantFolding))
        .add_pass(Box::new(PeepholeOptimizer))
}

// ============================================================
// Constant Folding Pass
// ============================================================

/// 常量折叠：编译时计算常量表达式。
///
/// 例如：`1 + 2` → `3`，`true and false` → `false`
pub struct ConstantFolding;

impl OptimizationPass for ConstantFolding {
    fn name(&self) -> &'static str {
        "constant-folding"
    }

    fn run(&self, mut module: HirModule) -> Result<HirModule, String> {
        for func in &mut module.functions {
            for block in &mut func.blocks {
                for instr in &mut block.instrs {
                    *instr = fold_instruction(instr, &module.constants);
                }
            }
        }
        Ok(module)
    }
}

fn fold_instruction(
    instr: &HirInstr,
    _constants: &[super::super::models::hir::ConstantValue],
) -> HirInstr {
    use super::super::models::hir::{ConstantValue as CV, HirBinaryOp, HirInstr as HI, HirOperand as HO};

    match instr {
        HI::Binary { dst: _, op, left, right } => {
            let result = match (left, right) {
                (HO::Immediate(CV::Int(a)), HO::Immediate(CV::Int(b))) => {
                    match op {
                        HirBinaryOp::Add => a.checked_add(*b).map(|v| CV::Int(v)),
                        HirBinaryOp::Sub => a.checked_sub(*b).map(|v| CV::Int(v)),
                        HirBinaryOp::Mul => a.checked_mul(*b).map(|v| CV::Int(v)),
                        HirBinaryOp::Div if *b != 0 => Some(CV::Int(a / b)),
                        HirBinaryOp::Mod if *b != 0 => Some(CV::Int(a % b)),
                        HirBinaryOp::Eq => Some(CV::Bool(a == b)),
                        HirBinaryOp::Neq => Some(CV::Bool(a != b)),
                        HirBinaryOp::Lt => Some(CV::Bool(a < b)),
                        HirBinaryOp::Gt => Some(CV::Bool(a > b)),
                        HirBinaryOp::Le => Some(CV::Bool(a <= b)),
                        HirBinaryOp::Ge => Some(CV::Bool(a >= b)),
                        _ => None,
                    }
                }
                (HO::Immediate(CV::Float(a)), HO::Immediate(CV::Float(b))) => {
                    match op {
                        HirBinaryOp::Add => Some(CV::Float(a + b)),
                        HirBinaryOp::Sub => Some(CV::Float(a - b)),
                        HirBinaryOp::Mul => Some(CV::Float(a * b)),
                        HirBinaryOp::Div if *b != 0.0 => Some(CV::Float(a / b)),
                        HirBinaryOp::Eq => Some(CV::Bool(a == b)),
                        HirBinaryOp::Neq => Some(CV::Bool(a != b)),
                        HirBinaryOp::Lt => Some(CV::Bool(a < b)),
                        HirBinaryOp::Gt => Some(CV::Bool(a > b)),
                        HirBinaryOp::Le => Some(CV::Bool(a <= b)),
                        HirBinaryOp::Ge => Some(CV::Bool(a >= b)),
                        _ => None,
                    }
                }
                _ => None,
            };

            match result {
                Some(val) => HI::LoadConst {
                    dst: HO::None,
                    value: val,
                },
                None => instr.clone(),
            }
        }
        _ => instr.clone(),
    }
}

// ============================================================
// Peephole Optimizer
// ============================================================

/// Peephole 优化：在基本块内识别并简化局部指令模式。
pub struct PeepholeOptimizer;

impl OptimizationPass for PeepholeOptimizer {
    fn name(&self) -> &'static str {
        "peephole"
    }

    fn run(&self, mut module: HirModule) -> Result<HirModule, String> {
        for func in &mut module.functions {
            for block in &mut func.blocks {
                // 简化：删除连续的冗余 Move
                let mut new_instrs = Vec::new();
                for instr in &block.instrs {
                    match instr {
                        HirInstr::Nop => continue, // 删除 nop
                        _ => new_instrs.push(instr.clone()),
                    }
                }

                // 合并相邻的 LoadConst + LoadConst + Binary
                new_instrs = merge_const_binary(new_instrs);

                block.instrs = new_instrs;
            }
        }
        Ok(module)
    }
}

/// 合并模式：LoadConst a; LoadConst b; Binary op → LoadConst result
fn merge_const_binary(instrs: Vec<HirInstr>) -> Vec<HirInstr> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < instrs.len() {
        if i + 2 < instrs.len() {
            // 简化检查：连续两个 LoadConst 后面跟着 Binary
            if let (
                HirInstr::LoadConst { dst: _, value: a },
                HirInstr::LoadConst { dst: _, value: b },
                HirInstr::Binary { dst: _, op, left: _, right: _ },
            ) = (&instrs[i], &instrs[i + 1], &instrs[i + 2])
            {
                // 对常量做折叠（复用 constant folding 逻辑）
                let folded = match (a, b, op) {
                    (CV::Int(a), CV::Int(b), HirBinaryOp::Add) => {
                        a.checked_add(*b).map(|v| CV::Int(v))
                    }
                    (CV::Int(a), CV::Int(b), HirBinaryOp::Sub) => {
                        a.checked_sub(*b).map(|v| CV::Int(v))
                    }
                    (CV::Int(a), CV::Int(b), HirBinaryOp::Mul) => {
                        a.checked_mul(*b).map(|v| CV::Int(v))
                    }
                    _ => None,
                };

                if let Some(val) = folded {
                    result.push(HirInstr::LoadConst {
                        dst: HirOperand::None,
                        value: val,
                    });
                    i += 3;
                    continue;
                }
            }
        }

        result.push(instrs[i].clone());
        i += 1;
    }
    result
}

use super::super::models::hir::{ConstantValue as CV, HirBinaryOp, HirInstr, HirOperand};

#[cfg(test)]
mod tests {
    use crate::domain::models::hir::{
        ConstantValue, HirBinaryOp, HirBlock, HirFunction, HirInstr, HirModule, HirOperand,
        HirTerminator,
    };
    use crate::domain::optimization::pass::{ConstantFolding, PeepholeOptimizer, OptimizationPass, create_default_pipeline};

    fn make_simple_module() -> HirModule {
        let mut module = HirModule::new();
        module.functions.push(HirFunction {
            name: Some("main".into()),
            arity: 0,
            local_count: 1,
            return_type: None,
            entry: 0,
            blocks: vec![HirBlock {
                id: 0,
                instrs: vec![
                    HirInstr::LoadConst {
                        dst: HirOperand::Temp(0),
                        value: ConstantValue::Int(1),
                    },
                    HirInstr::LoadConst {
                        dst: HirOperand::Temp(1),
                        value: ConstantValue::Int(2),
                    },
                    HirInstr::Binary {
                        dst: HirOperand::Temp(2),
                        op: HirBinaryOp::Add,
                        left: HirOperand::Temp(0),
                        right: HirOperand::Temp(1),
                    },
                ],
                term: HirTerminator::Return {
                    value: Some(HirOperand::Temp(2)),
                },
            }],
        });
        module
    }

    #[test]
    fn test_constant_folding_immediate() {
        // Only Immediates get folded currently
        let mut module = HirModule::new();
        module.functions.push(HirFunction {
            name: Some("main".into()),
            arity: 0, local_count: 1, return_type: None, entry: 0,
            blocks: vec![HirBlock {
                id: 0,
                instrs: vec![
                    HirInstr::Binary {
                        dst: HirOperand::Temp(0),
                        op: HirBinaryOp::Add,
                        left: HirOperand::Immediate(ConstantValue::Int(1)),
                        right: HirOperand::Immediate(ConstantValue::Int(2)),
                    },
                ],
                term: HirTerminator::Return { value: Some(HirOperand::Temp(0)) },
            }],
        });
        let result = ConstantFolding.run(module).unwrap();
        let block = &result.functions[0].blocks[0];
        let has_binary = block.instrs.iter().any(|i| matches!(i, HirInstr::Binary { .. }));
        assert!(!has_binary, "Constant folding should fold 1+2 into LoadConst(3)");
    }

    #[test]
    fn test_default_pipeline_runs() {
        let module = make_simple_module();
        let pipeline = create_default_pipeline();
        let result = pipeline.run(module).unwrap();
        assert!(!result.functions.is_empty());
    }

    #[test]
    fn test_peephole_nop_removal() {
        let mut module = HirModule::new();
        module.functions.push(HirFunction {
            name: Some("main".into()),
            arity: 0, local_count: 1, return_type: None, entry: 0,
            blocks: vec![HirBlock {
                id: 0,
                instrs: vec![
                    HirInstr::Nop,
                    HirInstr::LoadConst { dst: HirOperand::Temp(0), value: ConstantValue::Int(42) },
                    HirInstr::Nop,
                ],
                term: HirTerminator::End,
            }],
        });
        let result = PeepholeOptimizer.run(module).unwrap();
        assert_eq!(result.functions[0].blocks[0].instrs.len(), 1);
    }
}
