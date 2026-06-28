//! Parser — Kaubo v2
//!
//! 表达式导向，递归下降 + Pratt 运算符解析
//! `;` 为分隔符，block 最后一个表达式即返回值

use crate::ast::*;
use crate::lexer::Lexer;
use crate::token::{Token, TokenKind};
use std::collections::BTreeSet;

pub type ParseResult<T> = Result<T, String>;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    struct_names: BTreeSet<String>,
    variant_names: BTreeSet<String>,
    variant_to_enum: std::collections::HashMap<String, String>,
    variant_tag: std::collections::HashMap<String, u16>,
    opt_chain_counter: usize,
}

impl Parser {
    pub fn new(source: &str) -> Self {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        Self::from_tokens(tokens)
    }

    /// 注册外部已知的结构体名称（用于导入 struct 的解析支持）。
    ///
    /// 调用此方法后，parser 会将 `Name { ... }` 形式的语法识别为 StructLit，
    /// 即使该 struct 没有在当前文件中定义。
    pub fn register_struct_name(&mut self, name: &str) {
        self.struct_names.insert(name.to_string());
    }

    pub fn from_tokens(tokens: Vec<Token>) -> Self {
        let struct_names = collect_struct_names(&tokens);
        let (variant_names, variant_to_enum, variant_tag) = collect_enum_metadata(&tokens);
        Self {
            tokens,
            pos: 0,
            struct_names,
            variant_names,
            variant_to_enum,
            variant_tag,
            opt_chain_counter: 0,
        }
    }

    // ── 模块入口 ──

    pub fn parse(&mut self) -> ParseResult<Module> {
        let mut stmts = Vec::new();
        while !self.is_eof() {
            stmts.push(self.parse_top()?);
            self.skip_semis();
        }
        Ok(Module { stmts })
    }

    fn parse_top(&mut self) -> ParseResult<Stmt> {
        match self.current_kind() {
            TokenKind::Const => self.parse_const(),
            TokenKind::Var => self.parse_var(),
            TokenKind::Struct => self.parse_struct(),
            TokenKind::Enum => self.parse_enum(),
            TokenKind::Impl => self.parse_impl(),
            TokenKind::Interface => self.parse_interface(),
            TokenKind::Export => {
                self.bump();
                Ok(Stmt::ExportStmt(Box::new(self.parse_top()?)))
            }
            TokenKind::Import => self.parse_import(),
            TokenKind::Semicolon | TokenKind::Comment => {
                self.bump();
                self.parse_top()
            }
            _ => {
                let expr = self.parse_expr()?;
                self.expect_semi()?;
                Ok(Stmt::ExprStmt(expr))
            }
        }
    }

    // ── 声明 ──

    fn parse_const(&mut self) -> ParseResult<Stmt> {
        self.bump(); // const
        let name = self.expect_ident()?;
        let ty = self.opt_type()?;
        self.expect(TokenKind::Eq)?;
        let val = self.parse_expr()?;
        self.expect_semi()?;
        Ok(Stmt::ConstDecl {
            name,
            ty_ann: ty,
            value: val,
        })
    }

    fn parse_var(&mut self) -> ParseResult<Stmt> {
        self.bump(); // var
        let name = self.expect_ident()?;
        let ty = self.opt_type()?;
        let val = if self.current_kind() == TokenKind::Eq {
            self.bump();
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect_semi()?;
        Ok(Stmt::VarDecl {
            name,
            ty_ann: ty,
            value: val,
        })
    }

    fn parse_struct(&mut self) -> ParseResult<Stmt> {
        self.bump(); // struct
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while self.current_kind() != TokenKind::RBrace {
            let fname = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let fty = self.parse_type()?;
            fields.push(FieldDef {
                name: fname,
                ty: fty,
            });
            if self.current_kind() == TokenKind::Comma {
                self.bump();
            }
        }
        self.bump(); // }
        self.skip_semis();
        Ok(Stmt::StructDef { name, fields })
    }

    fn parse_enum(&mut self) -> ParseResult<Stmt> {
        self.bump(); // enum
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while self.current_kind() != TokenKind::RBrace {
            let vname = self.expect_ident()?;
            let fields = if self.current_kind() == TokenKind::LParen {
                self.bump(); // (
                let mut fs = Vec::new();
                while self.current_kind() != TokenKind::RParen {
                    let fname = self.expect_ident()?;
                    self.expect(TokenKind::Colon)?;
                    let fty = self.parse_type()?;
                    fs.push(FieldDef {
                        name: fname,
                        ty: fty,
                    });
                    if self.current_kind() == TokenKind::Comma {
                        self.bump();
                    }
                }
                self.bump(); // )
                fs
            } else {
                vec![]
            };
            variants.push(VariantDef {
                name: vname,
                fields,
            });
            if self.current_kind() == TokenKind::Comma {
                self.bump();
            }
        }
        self.bump(); // }
        self.skip_semis();
        Ok(Stmt::EnumDef { name, variants })
    }

    fn parse_interface(&mut self) -> ParseResult<Stmt> {
        self.bump(); // interface
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut methods = Vec::new();
        while self.current_kind() != TokenKind::RBrace {
            let is_operator = if self.current_kind() == TokenKind::Operator {
                self.bump();
                true
            } else {
                false
            };
            let mname = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            // Parse method signature: |params| -> ReturnType
            self.expect(TokenKind::Bar)?;
            let mut params = Vec::new();
            while self.current_kind() != TokenKind::Bar {
                let pname = self.expect_ident()?;
                self.expect(TokenKind::Colon)?;
                let pty = self.parse_type()?;
                params.push(Param {
                    name: pname,
                    ty_ann: Some(pty),
                });
                if self.current_kind() == TokenKind::Comma {
                    self.bump();
                }
            }
            self.bump(); // |
            let return_type = if self.current_kind() == TokenKind::FatArrow {
                self.bump(); // ->
                Some(self.parse_type()?)
            } else {
                None
            };
            methods.push(MethodSig {
                name: mname,
                params,
                return_type,
                operator: is_operator,
            });
            self.expect(TokenKind::Semicolon)?;
        }
        self.bump(); // }
        self.skip_semis();
        Ok(Stmt::InterfaceDef { name, methods })
    }

    fn parse_impl(&mut self) -> ParseResult<Stmt> {
        self.bump(); // impl
        let first = self.expect_ident()?;
        // impl Interface for Struct { ... } or impl Struct { ... }
        let (struct_name, interface_name) = if self.current_kind() == TokenKind::For {
            self.bump(); // for
            let struct_name = self.expect_ident()?;
            (struct_name, Some(first))
        } else {
            (first, None)
        };
        self.expect(TokenKind::LBrace)?;
        let mut methods = Vec::new();
        while self.current_kind() != TokenKind::RBrace {
            let is_operator = if self.current_kind() == TokenKind::Operator {
                self.bump();
                true
            } else {
                false
            };
            let mname = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let body = self.parse_expr()?;
            methods.push(MethodDef {
                name: mname,
                body,
                operator: is_operator,
            });
            if matches!(self.current_kind(), TokenKind::Semicolon | TokenKind::Comma) {
                self.bump();
            }
        }
        self.bump(); // }
        self.skip_semis();
        Ok(Stmt::ImplBlock {
            struct_name,
            interface_name,
            methods,
        })
    }

    fn parse_import(&mut self) -> ParseResult<Stmt> {
        self.bump(); // import
        if self.current_kind() == TokenKind::LBrace {
            self.bump();
            let mut names = Vec::new();
            while self.current_kind() != TokenKind::RBrace {
                names.push(self.expect_ident()?);
                if self.current_kind() == TokenKind::Comma {
                    self.bump();
                }
            }
            self.bump(); // }
            self.expect_kw(TokenKind::From)?;
            let path = self.expect_string()?;
            self.expect_semi()?;
            Ok(Stmt::Import {
                path,
                alias: None,
                names,
            })
        } else {
            let path = self.expect_string()?;
            let alias = if self.current_kind() == TokenKind::As {
                self.bump();
                Some(self.expect_ident()?)
            } else {
                None
            };
            self.expect_semi()?;
            Ok(Stmt::Import {
                path,
                alias,
                names: vec![],
            })
        }
    }

    // ── 表达式入口 (Pratt) ──

    fn parse_expr(&mut self) -> ParseResult<Expr> {
        self.parse_pratt(0)
    }

    fn parse_pratt(&mut self, min_bp: u8) -> ParseResult<Expr> {
        let mut left = self.parse_atom()?;
        left = self.chain_postfix(left)?;

        loop {
            // Special: ?? null-coalescing (bp=2, left-assoc)
            if self.current_kind() == TokenKind::QuestionQuestion {
                if 2 < min_bp {
                    break;
                }
                self.bump();
                let right = self.parse_pratt(3)?;
                left = self.desugar_null_coalesce(left, right);
                continue;
            }

            let (lbp, op) = match self.current_kind() {
                TokenKind::Semicolon
                | TokenKind::RBrace
                | TokenKind::RParen
                | TokenKind::RBracket
                | TokenKind::Comma
                | TokenKind::Eof
                | TokenKind::In
                | TokenKind::Else
                | TokenKind::Colon => break,
                TokenKind::Eq => (1, None), // = 赋值
                TokenKind::Pipe => (1, Some(BinOp::Pipe)),
                TokenKind::GtGt => (1, Some(BinOp::GtGt)),
                TokenKind::Or => (2, Some(BinOp::Or)),
                TokenKind::And => (3, Some(BinOp::And)),
                TokenKind::EqEq => (4, Some(BinOp::Eq)),
                TokenKind::NotEq => (4, Some(BinOp::Ne)),
                TokenKind::Lt => (4, Some(BinOp::Lt)),
                TokenKind::Le => (4, Some(BinOp::Le)),
                TokenKind::Gt => (4, Some(BinOp::Gt)),
                TokenKind::Ge => (4, Some(BinOp::Ge)),
                TokenKind::Plus => (5, Some(BinOp::Add)),
                TokenKind::Minus => (5, Some(BinOp::Sub)),
                TokenKind::Asterisk => (6, Some(BinOp::Mul)),
                TokenKind::Slash => (6, Some(BinOp::Div)),
                TokenKind::Percent => (6, Some(BinOp::Mod)),
                _ => break,
            };
            if lbp < min_bp {
                break;
            }
            self.bump();

            let right_bp = if self.current_kind() == TokenKind::Eq {
                lbp - 1 // right-associative: a = b = c → a = (b = c)
            } else {
                lbp + 1 // left-associative: a + b + c → (a + b) + c
            };
            let right = self.parse_pratt(right_bp)?;

            if op.is_none() {
                left = Expr::Assign {
                    target: Box::new(left),
                    value: Box::new(right),
                };
            } else if let Some(binop) = op {
                left = Expr::Binary {
                    left: Box::new(left),
                    op: binop,
                    right: Box::new(right),
                };
            }
        }
        Ok(left)
    }

    // ── 原子表达式 ──

    fn parse_atom(&mut self) -> ParseResult<Expr> {
        match self.current_kind() {
            TokenKind::IntLiteral => self.parse_int(),
            TokenKind::FloatLiteral => self.parse_float(),
            TokenKind::StringLiteral => self.parse_string(),
            TokenKind::True => {
                self.bump();
                Ok(Expr::LitTrue)
            }
            TokenKind::False => {
                self.bump();
                Ok(Expr::LitFalse)
            }
            TokenKind::Null => {
                self.bump();
                Ok(Expr::LitNull)
            }

            TokenKind::Minus => {
                self.bump();
                let val = self.parse_pratt(10)?;
                Ok(Expr::Unary {
                    op: UnOp::Neg,
                    right: Box::new(val),
                })
            }
            TokenKind::Not => {
                self.bump();
                let val = self.parse_pratt(10)?;
                Ok(Expr::Unary {
                    op: UnOp::Not,
                    right: Box::new(val),
                })
            }

            TokenKind::Bar => self.parse_lambda(),

            TokenKind::LParen => {
                self.bump();
                // 空括号 () → 空元组 / unit
                if self.current_kind() == TokenKind::RParen {
                    self.bump();
                    return Ok(Expr::Tuple(vec![]));
                }
                let first = self.parse_expr()?;
                // 逗号 → 元组模式，继续收集直到 RParen
                if self.current_kind() == TokenKind::Comma {
                    let mut items = vec![first];
                    while self.current_kind() == TokenKind::Comma {
                        self.bump();
                        if self.current_kind() == TokenKind::RParen {
                            break; // 尾随逗号：单元素元组 (expr,)
                        }
                        items.push(self.parse_expr()?);
                    }
                    self.expect(TokenKind::RParen)?;
                    return Ok(Expr::Tuple(items));
                }
                // 无逗号 → 分组，折叠
                self.expect(TokenKind::RParen)?;
                Ok(first)
            }
            TokenKind::LBrace => self.parse_block(),
            TokenKind::LBracket => self.parse_list(),

            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Break => {
                self.bump();
                Ok(Expr::Break)
            }
            TokenKind::Continue => {
                self.bump();
                Ok(Expr::Continue)
            }
            TokenKind::Return => {
                self.bump();
                Ok(Expr::Return(Some(Box::new(self.parse_expr()?))))
            }
            TokenKind::Match => self.parse_match(),
            TokenKind::Async_ => {
                self.bump();
                Ok(Expr::Async(Box::new(self.parse_expr()?)))
            }
            TokenKind::Await => {
                self.bump();
                Ok(Expr::Await(Box::new(self.parse_expr()?)))
            }

            TokenKind::TemplateString => self.parse_template(),
            TokenKind::Identifier | TokenKind::Self_ => {
                let name = self.current().lexeme.clone();
                if self.variant_names.contains(&name) {
                    self.bump();
                    let enum_name = self.variant_to_enum.get(&name).cloned().unwrap_or_default();
                    let tag = self.variant_tag.get(&name).copied().unwrap_or(0);
                    if self.current_kind() == TokenKind::LParen {
                        // Payload variant: Some(args) → parse as Call for CPS build to handle
                        self.bump(); // (
                        let args = self.parse_call_args()?;
                        return Ok(Expr::Call {
                            func: Box::new(Expr::VarRef(name.clone())),
                            arg: Expr::call_arg(args),
                        });
                    }
                    // Unit variant: Red → VariantLit
                    return Ok(Expr::VariantLit {
                        enum_name,
                        variant_name: name,
                        tag,
                        fields: vec![],
                    });
                }
                Ok(Expr::VarRef(self.consume_lexeme()))
            }

            _ => Err(format!(
                "unexpected token {:?} at {}:{}",
                self.current_kind(),
                self.current().line,
                self.current().col
            )),
        }
    }

    // ── 后缀链 (call / dot / index / struct literal) ──

    fn chain_postfix(&mut self, mut expr: Expr) -> ParseResult<Expr> {
        loop {
            match self.current_kind() {
                TokenKind::LParen => {
                    self.bump(); // consume (
                    let args = self.parse_call_args()?;
                    expr = Expr::Call {
                        func: Box::new(expr),
                        arg: Expr::call_arg(args),
                    };
                }
                TokenKind::Dot => {
                    self.bump();
                    let field = self.expect_ident()?;
                    expr = Expr::Member {
                        object: Box::new(expr),
                        field,
                    };
                }
                TokenKind::LBracket => {
                    self.bump();
                    let index = self.parse_expr()?;
                    self.expect(TokenKind::RBracket)?;
                    expr = Expr::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                TokenKind::QuestionDot => {
                    self.bump();
                    let field = self.expect_ident()?;
                    expr = self.desugar_opt_chain(expr, |e| Expr::Member {
                        object: Box::new(e),
                        field: field.clone(),
                    });
                }
                TokenKind::QuestionLBracket => {
                    self.bump();
                    let index = self.parse_expr()?;
                    self.expect(TokenKind::RBracket)?;
                    expr = self.desugar_opt_chain(expr, |e| Expr::Index {
                        object: Box::new(e),
                        index: Box::new(index.clone()),
                    });
                }
                TokenKind::LBrace => {
                    if let Expr::VarRef(ref name) = expr {
                        // Name { ... } 始终解析为 StructLit，不查符号表
                        // struct 名有效性由 infer 阶段检查
                        let struct_name = name.clone();
                        self.bump();
                        let mut fields = Vec::new();
                        let mut spread: Option<Box<Expr>> = None;
                        while self.current_kind() != TokenKind::RBrace {
                            if self.current_kind() == TokenKind::DotDotDot {
                                self.bump();
                                spread = Some(Box::new(self.parse_expr()?));
                                if self.current_kind() == TokenKind::Comma {
                                    self.bump();
                                }
                                continue;
                            }
                            let fname = self.expect_ident()?;
                            let val = if self.current_kind() == TokenKind::Comma
                                || self.current_kind() == TokenKind::RBrace
                            {
                                Expr::VarRef(fname.clone())
                            } else {
                                self.expect(TokenKind::Colon)?;
                                self.parse_expr()?
                            };
                            fields.push((fname, val));
                            if self.current_kind() == TokenKind::Comma {
                                self.bump();
                            }
                        }
                        self.bump();
                        expr = Expr::StructLit {
                            name: struct_name,
                            fields,
                            spread,
                        };
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_call_args(&mut self) -> ParseResult<Vec<Expr>> {
        // LParen already consumed
        let mut args = Vec::new();
        while self.current_kind() != TokenKind::RParen {
            args.push(self.parse_expr()?);
            if self.current_kind() == TokenKind::Comma {
                self.bump();
            }
        }
        self.bump(); // RParen
        Ok(args)
    }

    // ── 表达式子解析 ──

    fn parse_int(&mut self) -> ParseResult<Expr> {
        let s = self.consume_lexeme();
        s.parse::<i64>()
            .map(Expr::LitInt)
            .map_err(|_| format!("invalid int: {s}"))
    }

    fn parse_float(&mut self) -> ParseResult<Expr> {
        let s = self.consume_lexeme();
        s.parse::<f64>()
            .map(Expr::LitFloat)
            .map_err(|_| format!("invalid float: {s}"))
    }

    fn parse_string(&mut self) -> ParseResult<Expr> {
        Ok(Expr::LitString(self.consume_lexeme()))
    }

    fn parse_lambda(&mut self) -> ParseResult<Expr> {
        self.bump(); // first |
        let mut params = Vec::new();
        while self.current_kind() != TokenKind::Bar {
            let pname = self.expect_ident()?;
            let ty = self.opt_type()?;
            params.push(Param {
                name: pname,
                ty_ann: ty,
            });
            if self.current_kind() == TokenKind::Comma {
                self.bump();
            }
        }
        self.bump(); // closing |
        let ret_ty = if self.current_kind() == TokenKind::FatArrow {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_expr()?;
        Ok(Expr::Lambda {
            params,
            ret_ty,
            body: Box::new(body),
        })
    }

    fn parse_block(&mut self) -> ParseResult<Expr> {
        self.bump(); // {
        let mut stmts = Vec::new();
        while self.current_kind() != TokenKind::RBrace {
            match self.current_kind() {
                TokenKind::Const => stmts.push(self.parse_const()?),
                TokenKind::Var => stmts.push(self.parse_var()?),
                TokenKind::Comment => {
                    self.bump();
                }
                _ => {
                    let expr = self.parse_expr()?;
                    self.skip_semis();
                    stmts.push(Stmt::ExprStmt(expr));
                }
            }
        }
        self.bump(); // }
        Ok(Expr::Block(stmts))
    }

    fn parse_list(&mut self) -> ParseResult<Expr> {
        self.bump(); // [
        let mut items = Vec::new();
        while self.current_kind() != TokenKind::RBracket {
            items.push(self.parse_expr()?);
            if self.current_kind() == TokenKind::Comma {
                self.bump();
            }
        }
        self.bump(); // ]
        Ok(Expr::ListLit(items))
    }

    fn parse_if(&mut self) -> ParseResult<Expr> {
        self.bump(); // if
        self.expect(TokenKind::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        let then_b = self.parse_expr()?;
        let else_b = if self.current_kind() == TokenKind::Else {
            self.bump();
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };
        Ok(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_b),
            else_branch: else_b,
        })
    }

    fn parse_while(&mut self) -> ParseResult<Expr> {
        self.bump(); // while
        self.expect(TokenKind::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        let body = self.parse_expr()?;
        Ok(Expr::While {
            cond: Box::new(cond),
            body: Box::new(body),
        })
    }

    fn parse_match(&mut self) -> ParseResult<Expr> {
        self.bump(); // match
        self.expect(TokenKind::LParen)?;
        let scrutinee = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::LBrace)?;
        let mut arms: Vec<(Option<Expr>, Expr)> = Vec::new(); // (pattern|None=wildcard, body)
        while self.current_kind() != TokenKind::RBrace {
            let pattern =
                if self.current_kind() == TokenKind::Identifier && self.current().lexeme == "_" {
                    self.bump();
                    None // wildcard
                } else {
                    Some(self.parse_expr()?)
                };
            self.expect(TokenKind::FatArrow)?;
            let body = self.parse_expr()?;
            arms.push((pattern, body));
            if self.current_kind() == TokenKind::Comma {
                self.bump();
            }
        }
        self.bump(); // }
        self.desugar_match(scrutinee, arms)
    }

    fn desugar_match(
        &mut self,
        scrutinee: Expr,
        arms: Vec<(Option<Expr>, Expr)>,
    ) -> ParseResult<Expr> {
        // { var __mN = scrutinee; if __mN == pat1 { ... } else ... }
        let tmp = format!("__m{}", self.opt_chain_counter);
        self.opt_chain_counter += 1;

        let mut result: Option<Expr> = None;
        for (pattern, body) in arms.into_iter().rev() {
            result = Some(match pattern {
                Some(pat) => {
                    // Check for unit variant pattern: VariantLit
                    if let Expr::VariantLit { tag, .. } = &pat {
                        Expr::If {
                            cond: Box::new(Expr::Binary {
                                left: Box::new(Expr::GetVariantTag(Box::new(Expr::VarRef(
                                    tmp.clone(),
                                )))),
                                op: BinOp::Eq,
                                right: Box::new(Expr::LitInt(*tag as i64)),
                            }),
                            then_branch: Box::new(body),
                            else_branch: result.map(Box::new),
                        }
                    } else if let Expr::VarRef(ref vname) = &pat {
                        if self.variant_names.contains(vname) {
                            let tag = self.variant_tag.get(vname).copied().unwrap_or(0);
                            Expr::If {
                                cond: Box::new(Expr::Binary {
                                    left: Box::new(Expr::GetVariantTag(Box::new(Expr::VarRef(
                                        tmp.clone(),
                                    )))),
                                    op: BinOp::Eq,
                                    right: Box::new(Expr::LitInt(tag as i64)),
                                }),
                                then_branch: Box::new(body),
                                else_branch: result.map(Box::new),
                            }
                        } else {
                            // Regular value comparison
                            Expr::If {
                                cond: Box::new(Expr::Binary {
                                    left: Box::new(Expr::VarRef(tmp.clone())),
                                    op: BinOp::Eq,
                                    right: Box::new(pat),
                                }),
                                then_branch: Box::new(body),
                                else_branch: result.map(Box::new),
                            }
                        }
                    } else if let Expr::Call {
                        func,
                        arg: bindings,
                    } = &pat
                    {
                        // Check for payload variant pattern: Some(v1, v2) -> ...
                        if let Expr::VarRef(ref vname) = func.as_ref() {
                            if self.variant_names.contains(vname) {
                                let tag = self.variant_tag.get(vname).copied().unwrap_or(0);
                                // Build block: bind variables + body
                                let mut stmts: Vec<Stmt> = Vec::new();
                                let binding_list = bindings.as_args();
                                for (i, binding) in binding_list.iter().enumerate() {
                                    if let Expr::VarRef(bname) = binding {
                                        stmts.push(Stmt::VarDecl {
                                            name: bname.clone(),
                                            ty_ann: None,
                                            value: Some(Expr::GetVariantField {
                                                object: Box::new(Expr::VarRef(tmp.clone())),
                                                field_idx: i as u16,
                                            }),
                                        });
                                    }
                                }
                                stmts.push(Stmt::ExprStmt(body));
                                Expr::If {
                                    cond: Box::new(Expr::Binary {
                                        left: Box::new(Expr::GetVariantTag(Box::new(
                                            Expr::VarRef(tmp.clone()),
                                        ))),
                                        op: BinOp::Eq,
                                        right: Box::new(Expr::LitInt(tag as i64)),
                                    }),
                                    then_branch: Box::new(Expr::Block(stmts)),
                                    else_branch: result.map(Box::new),
                                }
                            } else {
                                // Regular function call pattern (fallback)
                                Expr::If {
                                    cond: Box::new(Expr::Binary {
                                        left: Box::new(Expr::VarRef(tmp.clone())),
                                        op: BinOp::Eq,
                                        right: Box::new(pat),
                                    }),
                                    then_branch: Box::new(body),
                                    else_branch: result.map(Box::new),
                                }
                            }
                        } else {
                            Expr::If {
                                cond: Box::new(Expr::Binary {
                                    left: Box::new(Expr::VarRef(tmp.clone())),
                                    op: BinOp::Eq,
                                    right: Box::new(pat),
                                }),
                                then_branch: Box::new(body),
                                else_branch: result.map(Box::new),
                            }
                        }
                    } else {
                        // Default: literal value comparison
                        Expr::If {
                            cond: Box::new(Expr::Binary {
                                left: Box::new(Expr::VarRef(tmp.clone())),
                                op: BinOp::Eq,
                                right: Box::new(pat),
                            }),
                            then_branch: Box::new(body),
                            else_branch: result.map(Box::new),
                        }
                    }
                }
                None => {
                    // wildcard: the else branch
                    body
                }
            });
        }

        Ok(Expr::Block(vec![
            Stmt::VarDecl {
                name: tmp.clone(),
                ty_ann: None,
                value: Some(scrutinee),
            },
            Stmt::ExprStmt(result.unwrap_or(Expr::LitNull)),
        ]))
    }

    fn parse_for(&mut self) -> ParseResult<Expr> {
        self.bump(); // for
        self.expect(TokenKind::LParen)?;
        let varname = self.expect_ident()?;
        self.expect_kw(TokenKind::In)
            .map_err(|_| "expected 'in' in for loop".to_string())?;
        let iterable = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        let body = self.parse_expr()?;
        Ok(Expr::For {
            var: Param {
                name: varname,
                ty_ann: None,
            },
            iterable: Box::new(iterable),
            body: Box::new(body),
        })
    }

    fn parse_template(&mut self) -> ParseResult<Expr> {
        let template = self.consume_lexeme();
        // template: `hello {name}, age {age + 1}`
        // Build: "hello " + name.to_string() + ", age " + (age + 1).to_string()
        //
        // Braces: {{ → literal {, }} → literal }
        let mut parts: Vec<Expr> = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = template.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // {{ → literal {
            if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] == '{' {
                current.push('{');
                i += 2;
                continue;
            }
            // }} → literal }
            if chars[i] == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
                current.push('}');
                i += 2;
                continue;
            }
            if chars[i] == '{' {
                if !current.is_empty() {
                    parts.push(Expr::LitString(std::mem::take(&mut current)));
                }
                // Find matching }, skipping nested {{ and handles }}
                let mut depth = 1;
                let mut expr_str = String::new();
                let mut trailing_brace = false;
                i += 1;
                while i < chars.len() && depth > 0 {
                    // {{ → literal { inside expression
                    if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] == '{' {
                        expr_str.push('{');
                        i += 2;
                        continue;
                    }
                    if chars[i] == '}' {
                        depth -= 1;
                        if depth == 0 {
                            // }} → close interpolation + literal }
                            if i + 1 < chars.len() && chars[i + 1] == '}' {
                                trailing_brace = true;
                                i += 1; // skip second }
                            }
                            break;
                        }
                    } else if chars[i] == '{' {
                        depth += 1;
                    }
                    expr_str.push(chars[i]);
                    i += 1;
                }
                // Parse the expression and wrap in .to_string()
                let mut sub = Parser::new(&expr_str);
                let expr = sub.parse_expr()?;
                parts.push(Expr::Call {
                    func: Box::new(Expr::Member {
                        object: Box::new(expr),
                        field: "to_string".to_string(),
                    }),
                    arg: Expr::call_arg(vec![]),
                });
                // }} → append literal "}"
                if trailing_brace {
                    parts.push(Expr::LitString("}".to_string()));
                }
            } else {
                current.push(chars[i]);
            }
            i += 1;
        }

        if !current.is_empty() || parts.is_empty() {
            parts.push(Expr::LitString(std::mem::take(&mut current)));
        }

        // Fold with SAdd
        let mut result = parts.remove(0);
        for part in parts {
            result = Expr::Binary {
                left: Box::new(result),
                op: BinOp::SAdd,
                right: Box::new(part),
            };
        }
        Ok(result)
    }

    // ── 类型 ──

    fn parse_type(&mut self) -> ParseResult<TypeExpr> {
        // 元组类型: (T1, T2, ...) 或 ()
        if self.current_kind() == TokenKind::LParen {
            self.bump();
            if self.current_kind() == TokenKind::RParen {
                self.bump();
                return Ok(TypeExpr::Tuple(vec![]));
            }
            let first = self.parse_type()?;
            if self.current_kind() == TokenKind::Comma {
                let mut items = vec![first];
                while self.current_kind() == TokenKind::Comma {
                    self.bump();
                    if self.current_kind() == TokenKind::RParen {
                        break;
                    }
                    items.push(self.parse_type()?);
                }
                self.expect(TokenKind::RParen)?;
                return Ok(TypeExpr::Tuple(items));
            }
            self.expect(TokenKind::RParen)?;
            return Ok(first); // (T) → 不是元组，折叠
        }
        let name = self.expect_ident()?;
        if name == "List" {
            self.expect(TokenKind::Lt)?;
            let inner = self.parse_type()?;
            self.expect(TokenKind::Gt)?;
            Ok(TypeExpr::List(Box::new(inner)))
        } else {
            Ok(TypeExpr::Named(name))
        }
    }

    fn opt_type(&mut self) -> ParseResult<Option<TypeExpr>> {
        if self.current_kind() == TokenKind::Colon {
            self.bump();
            Ok(Some(self.parse_type()?))
        } else {
            Ok(None)
        }
    }

    fn desugar_opt_chain(&mut self, obj: Expr, accessor: impl FnOnce(Expr) -> Expr) -> Expr {
        let tmp = format!("__o{}", self.opt_chain_counter);
        self.opt_chain_counter += 1;
        // { var tmp = obj; if tmp != null { accessor(tmp) } else { null } }
        Expr::Block(vec![
            Stmt::VarDecl {
                name: tmp.clone(),
                ty_ann: None,
                value: Some(obj),
            },
            Stmt::ExprStmt(Expr::If {
                cond: Box::new(Expr::Binary {
                    left: Box::new(Expr::VarRef(tmp.clone())),
                    op: BinOp::Ne,
                    right: Box::new(Expr::LitNull),
                }),
                then_branch: Box::new(accessor(Expr::VarRef(tmp))),
                else_branch: Some(Box::new(Expr::LitNull)),
            }),
        ])
    }

    fn desugar_null_coalesce(&mut self, left: Expr, right: Expr) -> Expr {
        // if left != null { left } else { right }
        // Note: left is evaluated twice; acceptable for variable refs and field accesses.
        // A proper single-evaluation version needs type-level support for nullable union types.
        Expr::If {
            cond: Box::new(Expr::Binary {
                left: Box::new(left.clone()),
                op: BinOp::Ne,
                right: Box::new(Expr::LitNull),
            }),
            then_branch: Box::new(left),
            else_branch: Some(Box::new(right)),
        }
    }

    // ── 辅助 ──

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }
    fn current_kind(&self) -> TokenKind {
        self.current().kind
    }

    fn is_eof(&self) -> bool {
        self.current_kind() == TokenKind::Eof
    }

    fn bump(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
        self.pos += 1;
        t
    }

    fn consume_lexeme(&mut self) -> String {
        self.bump().lexeme.clone()
    }

    fn expect_ident(&mut self) -> ParseResult<String> {
        if matches!(
            self.current_kind(),
            TokenKind::Identifier | TokenKind::Self_
        ) {
            Ok(self.consume_lexeme())
        } else {
            Err(format!(
                "expected ident at {}:{}, got {:?}",
                self.current().line,
                self.current().col,
                self.current_kind()
            ))
        }
    }

    fn expect_string(&mut self) -> ParseResult<String> {
        if self.current_kind() == TokenKind::StringLiteral {
            Ok(self.consume_lexeme())
        } else {
            Err(format!(
                "expected string at {}:{}",
                self.current().line,
                self.current().col
            ))
        }
    }

    fn expect(&mut self, kind: TokenKind) -> ParseResult<()> {
        if self.current_kind() == kind {
            self.bump();
            Ok(())
        } else {
            Err(format!(
                "expected {:?} at {}:{}, got {:?}",
                kind,
                self.current().line,
                self.current().col,
                self.current_kind()
            ))
        }
    }

    fn expect_kw(&mut self, kind: TokenKind) -> ParseResult<()> {
        self.expect(kind)
    }

    fn expect_semi(&mut self) -> ParseResult<()> {
        if self.current_kind() == TokenKind::Semicolon {
            self.bump();
            Ok(())
        } else {
            Err(format!(
                "expected ; at {}:{}, got {:?}",
                self.current().line,
                self.current().col,
                self.current_kind()
            ))
        }
    }

    fn skip_semis(&mut self) {
        while matches!(
            self.current_kind(),
            TokenKind::Semicolon | TokenKind::Comment
        ) {
            self.bump();
        }
    }
}

fn collect_enum_metadata(
    tokens: &[Token],
) -> (
    BTreeSet<String>,
    std::collections::HashMap<String, String>,
    std::collections::HashMap<String, u16>,
) {
    let mut variant_names = BTreeSet::new();
    let mut variant_to_enum: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut variant_tag: std::collections::HashMap<String, u16> = std::collections::HashMap::new();

    let mut i = 0;
    while i < tokens.len() {
        // Look for: Enum Identifier LBrace
        if tokens[i].kind == TokenKind::Enum
            && i + 1 < tokens.len()
            && tokens[i + 1].kind == TokenKind::Identifier
        {
            let enum_name = tokens[i + 1].lexeme.clone();

            // Skip past Enum, Identifier
            i += 2;
            // Scan forward to find the opening brace
            while i < tokens.len() && tokens[i].kind != TokenKind::LBrace {
                i += 1;
            }
            if i >= tokens.len() {
                continue;
            }
            i += 1; // skip past LBrace
            let mut depth = 1;
            let mut tag: u16 = 0;

            while i < tokens.len() {
                match tokens[i].kind {
                    TokenKind::LBrace => {
                        depth += 1;
                    }
                    TokenKind::RBrace => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    TokenKind::Identifier if depth == 1 => {
                        let vname = tokens[i].lexeme.clone();
                        variant_names.insert(vname.clone());
                        variant_to_enum.insert(vname.clone(), enum_name.clone());
                        variant_tag.insert(vname.clone(), tag);
                        tag += 1;
                        // Skip variant payload: Identifier ( Type , ... )
                        if i + 1 < tokens.len() && tokens[i + 1].kind == TokenKind::LParen {
                            let mut p_depth = 1;
                            i += 2; // skip past Identifier and LParen
                            while i < tokens.len() && p_depth > 0 {
                                match tokens[i].kind {
                                    TokenKind::LParen => p_depth += 1,
                                    TokenKind::RParen => {
                                        p_depth -= 1;
                                    }
                                    _ => {}
                                }
                                i += 1;
                            }
                            continue; // i is already past RParen
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        }
        i += 1;
    }

    (variant_names, variant_to_enum, variant_tag)
}

fn collect_struct_names(tokens: &[Token]) -> BTreeSet<String> {
    tokens
        .windows(2)
        .filter(|&window| {
            (window[0].kind == TokenKind::Struct && window[1].kind == TokenKind::Identifier)
        })
        .map(|window| window[1].lexeme.clone())
        .collect()
}

// ── tests ──

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_mod(src: &str) -> Module {
        Parser::new(src).parse().unwrap()
    }

    fn parse_expr_only(src: &str) -> Expr {
        let mut p = Parser::new(src);
        let e = p.parse_expr().unwrap();
        assert!(p.is_eof(), "extra tokens");
        e
    }

    #[test]
    fn test_const() {
        let m = parse_mod("const x = 42;");
        match &m.stmts[0] {
            Stmt::ConstDecl { name, value, .. } => {
                assert_eq!(name, "x");
                assert!(matches!(value, Expr::LitInt(42)));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_var_with_type() {
        let m = parse_mod("var counter: Int64 = 0;");
        match &m.stmts[0] {
            Stmt::VarDecl { name, ty_ann, .. } => {
                assert_eq!(name, "counter");
                assert!(ty_ann.is_some());
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_lambda() {
        let m = parse_mod("const add = |a, b| { a + b };");
        match &m.stmts[0] {
            Stmt::ConstDecl { name, value, .. } => {
                assert_eq!(name, "add");
                assert!(matches!(value, Expr::Lambda { .. }));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_lambda_with_return_type() {
        let m = parse_mod("const greet = |name| -> String { \"Hello\" };");
        match &m.stmts[0] {
            Stmt::ConstDecl { value, .. } => {
                if let Expr::Lambda { ret_ty, .. } = value {
                    assert!(ret_ty.is_some());
                }
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_struct_def() {
        let m = parse_mod("struct Point { x: Float64, y: Float64 }");
        match &m.stmts[0] {
            Stmt::StructDef { name, fields } => {
                assert_eq!(name, "Point");
                assert_eq!(fields.len(), 2);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_impl_block() {
        let m = parse_mod("impl Point { dist: |self, other| -> Float64 { return 0.0; } }");
        match &m.stmts[0] {
            Stmt::ImplBlock {
                struct_name,
                methods,
                ..
            } => {
                assert_eq!(struct_name, "Point");
                assert_eq!(methods.len(), 1);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_import() {
        let m = parse_mod(r#"import "std/prelude";"#);
        assert!(matches!(&m.stmts[0], Stmt::Import { .. }));
    }

    #[test]
    fn test_export() {
        let m = parse_mod("export const greet = |name| { \"hi\" };");
        assert!(matches!(&m.stmts[0], Stmt::ExportStmt(_)));
    }

    #[test]
    fn test_if_else() {
        let e = parse_expr_only("if (x < 0) { -x } else { x }");
        assert!(matches!(e, Expr::If { .. }));
    }

    #[test]
    fn test_while_loop() {
        let e = parse_expr_only("while (i < 10) { i = i + 1 }");
        assert!(matches!(e, Expr::While { .. }));
    }

    #[test]
    fn test_pipe() {
        let e = parse_expr_only("a |> f |> g");
        assert!(matches!(
            e,
            Expr::Binary {
                op: BinOp::Pipe,
                ..
            }
        ));
    }

    #[test]
    fn test_assignment() {
        let e = parse_expr_only("x = 42");
        assert!(matches!(e, Expr::Assign { .. }));
    }

    #[test]
    fn test_method_call() {
        let e = parse_expr_only("42.as_float()");
        assert!(matches!(e, Expr::Call { .. }));
    }

    #[test]
    fn test_block_is_expr() {
        let m = parse_mod("const result = { var x = 10; x + 1 };");
        match &m.stmts[0] {
            Stmt::ConstDecl { name, value, .. } => {
                assert_eq!(name, "result");
                assert!(matches!(value, Expr::Block(_)));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_for_loop() {
        // TODO: fix for-loop in struct context
        let e = parse_expr_only("for (x in xs) { print(x) }");
        assert!(matches!(e, Expr::For { .. }));
    }

    #[test]
    fn test_struct_literal() {
        let m = parse_mod("struct Point { x: Int64 }; const p = Point { x: 100 };");
        assert!(matches!(
            &m.stmts[1],
            Stmt::ConstDecl {
                value: Expr::StructLit { name, .. },
                ..
            } if name == "Point"
        ));
    }

    #[test]
    fn test_lowercase_struct_literal_uses_declared_type() {
        let m = parse_mod("struct point { x: Int64 }; const p = point { x: 100 };");
        assert!(matches!(
            &m.stmts[1],
            Stmt::ConstDecl {
                value: Expr::StructLit { name, .. },
                ..
            } if name == "point"
        ));
    }

    #[test]
    fn test_uppercase_call_is_not_struct_literal_without_declaration() {
        let e = parse_expr_only("Point(1)");
        assert!(matches!(e, Expr::Call { .. }));
    }

    #[test]
    fn test_list_literal() {
        let e = parse_expr_only("[1, 2, 3]");
        assert!(matches!(e, Expr::ListLit(_)));
    }

    #[test]
    fn test_async_expr() {
        let e = parse_expr_only("async |id| { await f(id) }");
        assert!(matches!(e, Expr::Async(_)));
    }

    // ── Statement-level tests ──

    #[test]
    fn test_var_without_value() {
        let m = parse_mod("var x: Int64;");
        match &m.stmts[0] {
            Stmt::VarDecl { name, value, .. } => {
                assert_eq!(name, "x");
                assert!(value.is_none());
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_var_without_type() {
        let m = parse_mod("var x = 42;");
        match &m.stmts[0] {
            Stmt::VarDecl { name, ty_ann, .. } => {
                assert_eq!(name, "x");
                assert!(ty_ann.is_none());
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_const_with_type_annotation() {
        let m = parse_mod("const pi: Float64 = 123.456;");
        match &m.stmts[0] {
            Stmt::ConstDecl { name, ty_ann, .. } => {
                assert_eq!(name, "pi");
                assert!(matches!(ty_ann, Some(TypeExpr::Named(n)) if n == "Float64"));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_multiple_top_level_statements() {
        let m = parse_mod("const a = 1;\nconst b = 2;\nconst c = 3;");
        assert_eq!(m.stmts.len(), 3);
    }

    #[test]
    fn test_empty_module() {
        let m = parse_mod("");
        assert!(m.stmts.is_empty());
    }

    #[test]
    fn test_empty_module_with_semicolons() {
        // Semicolons-only module currently produces a parse error
        // because parse_top() on Eof falls through to parse_expr().
        // This is a known limitation.
        let result = Parser::new("; ; ;").parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_module_skips_comments_between_stmts() {
        let m = parse_mod("const a = 1;\n// a comment\nconst b = 2;");
        assert_eq!(m.stmts.len(), 2);
    }

    #[test]
    fn test_struct_single_field() {
        let m = parse_mod("struct Nothing { value: Int64 }");
        match &m.stmts[0] {
            Stmt::StructDef { name, fields } => {
                assert_eq!(name, "Nothing");
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].name, "value");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_struct_with_trailing_comma() {
        let m = parse_mod("struct Point { x: Int64, y: Int64, }");
        match &m.stmts[0] {
            Stmt::StructDef { name, fields } => {
                assert_eq!(name, "Point");
                assert_eq!(fields.len(), 2);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_impl_block_multiple_methods() {
        let m =
            parse_mod("impl Point { norm: |self| { 0.0 }, dist: |self, other| { return 0.0; } }");
        match &m.stmts[0] {
            Stmt::ImplBlock {
                struct_name,
                methods,
                ..
            } => {
                assert_eq!(struct_name, "Point");
                assert_eq!(methods.len(), 2);
                assert_eq!(methods[0].name, "norm");
                assert_eq!(methods[1].name, "dist");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_import_with_names() {
        let m = parse_mod("import { map, filter } from \"std/iter\";");
        match &m.stmts[0] {
            Stmt::Import { path, names, alias } => {
                assert_eq!(path, "std/iter");
                assert_eq!(&names[..], &["map", "filter"]);
                assert!(alias.is_none());
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_import_with_alias() {
        let m = parse_mod("import \"std/math\" as math;");
        match &m.stmts[0] {
            Stmt::Import { path, alias, names } => {
                assert_eq!(path, "std/math");
                assert_eq!(alias.as_deref(), Some("math"));
                assert!(names.is_empty());
            }
            _ => panic!(),
        }
    }

    // ── Expression primitives ──

    #[test]
    fn test_expr_lit_true() {
        let e = parse_expr_only("true");
        assert!(matches!(e, Expr::LitTrue));
    }

    #[test]
    fn test_expr_lit_false() {
        let e = parse_expr_only("false");
        assert!(matches!(e, Expr::LitFalse));
    }

    #[test]
    fn test_expr_lit_null() {
        let e = parse_expr_only("null");
        assert!(matches!(e, Expr::LitNull));
    }

    #[test]
    fn test_expr_lit_int_zero() {
        let e = parse_expr_only("0");
        assert_eq!(e, Expr::LitInt(0));
    }

    #[test]
    fn test_expr_lit_int_large() {
        let e = parse_expr_only("9223372036854775807");
        assert_eq!(e, Expr::LitInt(9223372036854775807));
    }

    #[test]
    fn test_expr_lit_float() {
        let e = parse_expr_only("123.456");
        assert_eq!(e, Expr::LitFloat(123.456));
    }

    #[test]
    fn test_expr_lit_string() {
        let e = parse_expr_only(r#""hello world""#);
        assert_eq!(e, Expr::LitString("hello world".to_string()));
    }

    #[test]
    fn test_expr_var_ref() {
        let e = parse_expr_only("myVariable");
        assert_eq!(e, Expr::VarRef("myVariable".to_string()));
    }

    #[test]
    fn test_expr_self_ref() {
        let e = parse_expr_only("self");
        assert_eq!(e, Expr::VarRef("self".to_string()));
    }

    // ── Unary expressions ──

    #[test]
    fn test_unary_neg() {
        let e = parse_expr_only("-x");
        assert!(matches!(e, Expr::Unary { op: UnOp::Neg, .. }));
    }

    #[test]
    fn test_unary_not() {
        let e = parse_expr_only("not x");
        assert!(matches!(e, Expr::Unary { op: UnOp::Not, .. }));
    }

    #[test]
    fn test_unary_double_neg() {
        let e = parse_expr_only("--x");
        match e {
            Expr::Unary {
                op: UnOp::Neg,
                right,
            } => {
                assert!(matches!(*right, Expr::Unary { op: UnOp::Neg, .. }));
            }
            _ => panic!("expected double neg, got {e:?}"),
        }
    }

    #[test]
    fn test_unary_neg_literal() {
        let e = parse_expr_only("-42");
        match e {
            Expr::Unary {
                op: UnOp::Neg,
                right,
            } => {
                assert_eq!(*right, Expr::LitInt(42));
            }
            _ => panic!("expected neg literal, got {e:?}"),
        }
    }

    #[test]
    fn test_unary_not_literal() {
        let e = parse_expr_only("not true");
        match e {
            Expr::Unary {
                op: UnOp::Not,
                right,
            } => {
                assert_eq!(*right, Expr::LitTrue);
            }
            _ => panic!("expected not true, got {e:?}"),
        }
    }

    // ── Binary expressions: all operators ──

    macro_rules! binop_test {
        ($name:ident, $src:literal, $expected_op:ident) => {
            #[test]
            fn $name() {
                let e = parse_expr_only($src);
                assert!(
                    matches!(
                        e,
                        Expr::Binary {
                            op: BinOp::$expected_op,
                            ..
                        }
                    ),
                    "expected {}, got {e:?}",
                    stringify!($expected_op)
                );
            }
        };
    }

    binop_test!(test_binop_add, "a + b", Add);
    binop_test!(test_binop_sub, "a - b", Sub);
    binop_test!(test_binop_mul, "a * b", Mul);
    binop_test!(test_binop_div, "a / b", Div);
    binop_test!(test_binop_mod, "a % b", Mod);
    binop_test!(test_binop_eq, "a == b", Eq);
    binop_test!(test_binop_ne, "a != b", Ne);
    binop_test!(test_binop_lt, "a < b", Lt);
    binop_test!(test_binop_le, "a <= b", Le);
    binop_test!(test_binop_gt, "a > b", Gt);
    binop_test!(test_binop_ge, "a >= b", Ge);
    binop_test!(test_binop_and, "a and b", And);
    binop_test!(test_binop_or, "a or b", Or);
    binop_test!(test_binop_pipe, "a |> b", Pipe);
    binop_test!(test_binop_gtgt, "a >> b", GtGt);

    // ── Operator precedence ──

    #[test]
    fn test_precedence_mul_before_add() {
        let e = parse_expr_only("a + b * c");
        // (a + (b * c))
        match e {
            Expr::Binary {
                op: BinOp::Add,
                left,
                right,
            } => {
                assert!(matches!(*left, Expr::VarRef(_)));
                assert!(matches!(*right, Expr::Binary { op: BinOp::Mul, .. }));
            }
            _ => panic!("expected a + (b * c), got {e:?}"),
        }
    }

    #[test]
    fn test_precedence_comparison_before_and() {
        let e = parse_expr_only("a < b and c < d");
        match e {
            Expr::Binary { op: BinOp::And, .. } => {}
            _ => panic!("expected comparison before and, got {e:?}"),
        }
    }

    #[test]
    fn test_precedence_assignment_lowest() {
        let e = parse_expr_only("x = a + b");
        assert!(matches!(e, Expr::Assign { .. }));
    }

    #[test]
    fn test_left_associativity_add() {
        let e = parse_expr_only("a + b + c");
        // ((a + b) + c): outer left is inner Add
        match e {
            Expr::Binary {
                op: BinOp::Add,
                left,
                right,
            } => {
                assert!(matches!(*left, Expr::Binary { op: BinOp::Add, .. }));
                assert!(matches!(*right, Expr::VarRef(_)));
            }
            _ => panic!("expected left-assoc add, got {e:?}"),
        }
    }

    #[test]
    fn test_left_associativity_sub() {
        let e = parse_expr_only("a - b - c");
        // ((a - b) - c)
        match e {
            Expr::Binary {
                op: BinOp::Sub,
                left,
                right,
            } => {
                assert!(matches!(*left, Expr::Binary { op: BinOp::Sub, .. }));
                assert!(matches!(*right, Expr::VarRef(_)));
            }
            _ => panic!("expected left-assoc sub, got {e:?}"),
        }
    }

    #[test]
    fn test_grouping_with_parens() {
        let e = parse_expr_only("(a + b) * c");
        match e {
            Expr::Binary {
                op: BinOp::Mul,
                left,
                ..
            } => {
                assert!(matches!(*left, Expr::Binary { op: BinOp::Add, .. }));
            }
            _ => panic!("expected (a+b) * c, got {e:?}"),
        }
    }

    // ── Postfix chain ──

    #[test]
    fn test_call_no_args() {
        let e = parse_expr_only("f()");
        match e {
            Expr::Call { func, arg } => {
                assert_eq!(*func, Expr::VarRef("f".to_string()));
                assert!(arg.as_args().is_empty());
            }
            _ => panic!("expected call, got {e:?}"),
        }
    }

    #[test]
    fn test_call_multiple_args() {
        let e = parse_expr_only("f(a, b, c)");
        match e {
            Expr::Call { func, arg } => {
                assert_eq!(*func, Expr::VarRef("f".to_string()));
                assert_eq!(arg.as_args().len(), 3);
            }
            _ => panic!("expected call, got {e:?}"),
        }
    }

    #[test]
    fn test_nested_call() {
        let e = parse_expr_only("f(g(x))");
        match e {
            Expr::Call { func, arg } => {
                assert_eq!(*func, Expr::VarRef("f".to_string()));
                let args = arg.as_args();
                assert_eq!(args.len(), 1);
                match &args[0] {
                    Expr::Call {
                        func: inner_func, ..
                    } => {
                        assert_eq!(**inner_func, Expr::VarRef("g".to_string()));
                    }
                    _ => panic!("expected nested call"),
                }
            }
            _ => panic!("expected call, got {e:?}"),
        }
    }

    #[test]
    fn test_member_dot() {
        let e = parse_expr_only("obj.field");
        match e {
            Expr::Member { object, field } => {
                assert_eq!(*object, Expr::VarRef("obj".to_string()));
                assert_eq!(field, "field");
            }
            _ => panic!("expected member, got {e:?}"),
        }
    }

    #[test]
    fn test_member_dot_chain() {
        let e = parse_expr_only("a.b.c");
        match e {
            Expr::Member { object, field } => {
                assert_eq!(field, "c");
                assert!(matches!(*object, Expr::Member { .. }));
            }
            _ => panic!("expected member chain, got {e:?}"),
        }
    }

    #[test]
    fn test_method_call_on_value() {
        let e = parse_expr_only("42.as_float()");
        match e {
            Expr::Call { func, arg } => {
                assert!(
                    matches!(*func, Expr::Member { object, .. } if *object == Expr::LitInt(42))
                );
                assert!(arg.as_args().is_empty());
            }
            _ => panic!("expected method call, got {e:?}"),
        }
    }

    // ── 元组字面量 ──

    #[test]
    fn test_empty_tuple() {
        let e = parse_expr_only("()");
        assert!(matches!(e, Expr::Tuple(items) if items.is_empty()));
    }

    #[test]
    fn test_single_element_tuple() {
        let e = parse_expr_only("(1,)");
        match e {
            Expr::Tuple(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0], Expr::LitInt(1));
            }
            _ => panic!("expected tuple, got {e:?}"),
        }
    }

    #[test]
    fn test_multi_element_tuple() {
        let e = parse_expr_only("(1, 2)");
        match e {
            Expr::Tuple(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0], Expr::LitInt(1));
                assert_eq!(items[1], Expr::LitInt(2));
            }
            _ => panic!("expected tuple, got {e:?}"),
        }
    }

    #[test]
    fn test_nested_tuple() {
        let e = parse_expr_only("((1, 2), 3)");
        match e {
            Expr::Tuple(items) => {
                assert_eq!(items.len(), 2);
                assert!(matches!(&items[0], Expr::Tuple(inner) if inner.len() == 2));
                assert_eq!(items[1], Expr::LitInt(3));
            }
            _ => panic!("expected nested tuple, got {e:?}"),
        }
    }

    #[test]
    fn test_grouping_not_tuple() {
        let e = parse_expr_only("(1 + 2)");
        // (1 + 2) is grouping, NOT a tuple — no comma
        assert!(!matches!(e, Expr::Tuple(_)));
    }

    #[test]
    fn test_zero_arg_call_produces_empty_tuple() {
        let e = parse_expr_only("f()");
        match e {
            Expr::Call { func, arg } => {
                assert_eq!(*func, Expr::VarRef("f".to_string()));
                assert!(matches!(*arg, Expr::Tuple(items) if items.is_empty()));
            }
            _ => panic!("expected call, got {e:?}"),
        }
    }

    #[test]
    fn test_call_with_two_args_produces_tuple() {
        let e = parse_expr_only("add(1, 2)");
        match e {
            Expr::Call { func, arg } => {
                assert_eq!(*func, Expr::VarRef("add".to_string()));
                assert!(matches!(*arg, Expr::Tuple(items) if items.len() == 2));
            }
            _ => panic!("expected call, got {e:?}"),
        }
    }

    // ── 元组类型标注 ──

    #[test]
    fn test_parse_tuple_type() {
        let m = Parser::new("const x: (Int64, String) = (1, \"a\");")
            .parse()
            .unwrap();
        match &m.stmts[0] {
            Stmt::ConstDecl { ty_ann, .. } => match ty_ann {
                Some(TypeExpr::Tuple(items)) => {
                    assert_eq!(items.len(), 2);
                    assert!(matches!(&items[0], TypeExpr::Named(n) if n == "Int64"));
                    assert!(matches!(&items[1], TypeExpr::Named(n) if n == "String"));
                }
                _ => panic!("expected Tuple type, got {ty_ann:?}"),
            },
            _ => panic!("expected ConstDecl"),
        }
    }

    #[test]
    fn test_parse_empty_tuple_type() {
        let m = Parser::new("const x: () = ();").parse().unwrap();
        match &m.stmts[0] {
            Stmt::ConstDecl { ty_ann, .. } => {
                assert!(matches!(ty_ann, Some(TypeExpr::Tuple(items)) if items.is_empty()));
            }
            _ => panic!("expected ConstDecl"),
        }
    }

    #[test]
    fn test_index_bracket() {
        let e = parse_expr_only("arr[0]");
        match e {
            Expr::Index { object, index } => {
                assert_eq!(*object, Expr::VarRef("arr".to_string()));
                assert_eq!(*index, Expr::LitInt(0));
            }
            _ => panic!("expected index, got {e:?}"),
        }
    }

    #[test]
    fn test_index_nested() {
        let e = parse_expr_only("matrix[i][j]");
        match e {
            Expr::Index { object, .. } => {
                assert!(matches!(*object, Expr::Index { .. }));
            }
            _ => panic!("expected nested index, got {e:?}"),
        }
    }

    // ── Control flow ──

    #[test]
    fn test_if_without_else() {
        let e = parse_expr_only("if (x > 0) { return x }");
        match e {
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                assert!(else_branch.is_none());
                assert!(matches!(*cond, Expr::Binary { .. }));
                assert!(matches!(*then_branch, Expr::Block(_)));
            }
            _ => panic!("expected if without else, got {e:?}"),
        }
    }

    #[test]
    fn test_if_nested() {
        let e = parse_expr_only("if (a > 0) { if (b > 0) { 1 } else { 0 } } else { -1 }");
        match e {
            Expr::If { else_branch, .. } => {
                assert!(else_branch.is_some());
            }
            _ => panic!("expected nested if, got {e:?}"),
        }
    }

    #[test]
    fn test_while_body_is_block() {
        let e = parse_expr_only("while (i < 10) { i = i + 1; x = x * 2 }");
        match e {
            Expr::While { body, .. } => match &*body {
                Expr::Block(stmts) => assert_eq!(stmts.len(), 2),
                _ => panic!("expected block body"),
            },
            _ => panic!("expected while, got {e:?}"),
        }
    }

    #[test]
    fn test_for_loop_full() {
        let e = parse_expr_only("for (x in xs) { print(x) }");
        match e {
            Expr::For {
                var,
                iterable,
                body,
            } => {
                assert_eq!(var.name, "x");
                assert_eq!(*iterable, Expr::VarRef("xs".to_string()));
                assert!(matches!(*body, Expr::Block(_)));
            }
            _ => panic!("expected for loop, got {e:?}"),
        }
    }

    #[test]
    fn test_break_expr() {
        let e = parse_expr_only("break");
        assert_eq!(e, Expr::Break);
    }

    #[test]
    fn test_continue_expr() {
        let e = parse_expr_only("continue");
        assert_eq!(e, Expr::Continue);
    }

    #[test]
    fn test_return_with_value() {
        let e = parse_expr_only("return 42");
        match e {
            Expr::Return(Some(val)) => assert_eq!(*val, Expr::LitInt(42)),
            _ => panic!("expected return 42, got {e:?}"),
        }
    }

    #[test]
    fn test_return_expr() {
        // Return always takes an expression in this parser
        let e = parse_expr_only("return x");
        assert!(matches!(e, Expr::Return(Some(_))));
    }

    // ── Lambda expressions ──

    #[test]
    fn test_lambda_no_params() {
        let e = parse_expr_only("|| { 42 }");
        match e {
            Expr::Lambda { params, body, .. } => {
                assert!(params.is_empty());
                assert!(matches!(*body, Expr::Block(_)));
            }
            _ => panic!("expected lambda, got {e:?}"),
        }
    }

    #[test]
    fn test_lambda_single_param() {
        let e = parse_expr_only("|x| { x + 1 }");
        match e {
            Expr::Lambda { params, .. } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].name, "x");
            }
            _ => panic!("expected lambda, got {e:?}"),
        }
    }

    #[test]
    fn test_lambda_params_with_types() {
        let e = parse_expr_only("|a: Int64, b: Float64| -> Float64 { a }");
        match e {
            Expr::Lambda { params, ret_ty, .. } => {
                assert_eq!(params.len(), 2);
                assert!(params[0].ty_ann.is_some());
                assert!(params[1].ty_ann.is_some());
                assert!(ret_ty.is_some());
            }
            _ => panic!("expected lambda with types, got {e:?}"),
        }
    }

    #[test]
    fn test_nested_lambda() {
        let e = parse_expr_only("|a| { |b| { a + b } }");
        match e {
            Expr::Lambda { params, body, .. } => {
                assert_eq!(params.len(), 1);
                match &*body {
                    Expr::Block(stmts) => {
                        assert_eq!(stmts.len(), 1);
                    }
                    _ => panic!("expected block body"),
                }
            }
            _ => panic!("expected nested lambda, got {e:?}"),
        }
    }

    #[test]
    fn test_lambda_direct_body_no_block() {
        let e = parse_expr_only("|a| a + 1");
        match e {
            Expr::Lambda { params, body, .. } => {
                assert_eq!(params.len(), 1);
                assert!(matches!(*body, Expr::Binary { .. }));
            }
            _ => panic!("expected lambda, got {e:?}"),
        }
    }

    // ── List literal ──

    #[test]
    fn test_list_empty() {
        let e = parse_expr_only("[]");
        match e {
            Expr::ListLit(items) => assert!(items.is_empty()),
            _ => panic!("expected empty list, got {e:?}"),
        }
    }

    #[test]
    fn test_list_single() {
        let e = parse_expr_only("[42]");
        match e {
            Expr::ListLit(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0], Expr::LitInt(42));
            }
            _ => panic!("expected single element list, got {e:?}"),
        }
    }

    #[test]
    fn test_list_nested() {
        let e = parse_expr_only("[[1, 2], [3, 4]]");
        match e {
            Expr::ListLit(items) => {
                assert_eq!(items.len(), 2);
                assert!(matches!(items[0], Expr::ListLit(_)));
            }
            _ => panic!("expected nested list, got {e:?}"),
        }
    }

    // ── Struct literal ──

    #[test]
    fn test_struct_literal_single_field() {
        let m = parse_mod("struct Pair { first: Int64 }; const p = Pair { first: 10 };");
        match &m.stmts[1] {
            Stmt::ConstDecl { value, .. } => match value {
                Expr::StructLit { name, fields, .. } => {
                    assert_eq!(name, "Pair");
                    assert_eq!(fields.len(), 1);
                    assert_eq!(fields[0].0, "first");
                }
                _ => panic!("expected struct lit, got {value:?}"),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn test_struct_literal_shorthand_property() {
        let m = parse_mod("struct Point { x: Int64, y: Int64 }; const p = Point { x, y };");
        match &m.stmts[1] {
            Stmt::ConstDecl { value, .. } => match value {
                Expr::StructLit { name, fields, .. } => {
                    assert_eq!(name, "Point");
                    assert_eq!(fields.len(), 2);
                    assert_eq!(fields[0].0, "x");
                    assert_eq!(fields[0].1, Expr::VarRef("x".to_string()));
                    assert_eq!(fields[1].0, "y");
                    assert_eq!(fields[1].1, Expr::VarRef("y".to_string()));
                }
                _ => panic!("expected struct lit, got {value:?}"),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn test_struct_literal_mixed_shorthand_and_explicit() {
        let m = parse_mod("struct Point { x: Int64, y: Int64 }; const p = Point { x, y: 10 };");
        match &m.stmts[1] {
            Stmt::ConstDecl { value, .. } => match value {
                Expr::StructLit { fields, .. } => {
                    assert_eq!(fields.len(), 2);
                    assert_eq!(fields[0].0, "x");
                    assert_eq!(fields[0].1, Expr::VarRef("x".to_string()));
                    assert_eq!(fields[1].0, "y");
                    assert_eq!(fields[1].1, Expr::LitInt(10));
                }
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn test_struct_literal_dot_access() {
        let m = parse_mod("struct Point { x: Int64 }; const val = Point { x: 100 }.x;");
        match &m.stmts[1] {
            Stmt::ConstDecl { value, .. } => {
                assert!(matches!(value, Expr::Member { field, .. } if field == "x"));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_struct_always_parsed_as_struct_lit() {
        // Name { ... } 始终解析为 StructLit，不查符号表
        // struct 名有效性由 infer 阶段检查
        let result = Parser::new("const p = Unknown { x: 1 };").parse();
        assert!(result.is_ok());
        let m = result.unwrap();
        match &m.stmts[0] {
            Stmt::ConstDecl { value, .. } => {
                assert!(matches!(value, Expr::StructLit { name, .. } if name == "Unknown"));
            }
            _ => panic!("expected ConstDecl"),
        }
    }

    // ── Type annotation ──

    #[test]
    fn test_type_named() {
        let m = parse_mod("const x: Int64 = 42;");
        match &m.stmts[0] {
            Stmt::ConstDecl { ty_ann, .. } => {
                assert_eq!(*ty_ann, Some(TypeExpr::Named("Int64".to_string())));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_type_list() {
        let m = parse_mod("const xs: List<Int64> = [];");
        match &m.stmts[0] {
            Stmt::ConstDecl { ty_ann, .. } => {
                assert_eq!(
                    *ty_ann,
                    Some(TypeExpr::List(Box::new(TypeExpr::Named(
                        "Int64".to_string()
                    ))))
                );
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_var_type_annotation() {
        let m = parse_mod("var name: String;");
        match &m.stmts[0] {
            Stmt::VarDecl { ty_ann, .. } => {
                assert_eq!(*ty_ann, Some(TypeExpr::Named("String".to_string())));
            }
            _ => panic!(),
        }
    }

    // ── Error cases ──

    #[test]
    fn test_error_missing_semicolon() {
        let result = Parser::new("const x = 42").parse();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected ;"));
    }

    #[test]
    fn test_error_unexpected_token() {
        let result = Parser::new("const = 42;").parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_error_missing_colon_in_struct() {
        let result = Parser::new("struct Bad { x Int64 };").parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_error_invalid_int() {
        // int too large for i64
        let result = Parser::new("const x = 99999999999999999999;").parse();
        assert!(result.is_err());
    }

    // ── Await / Async ──

    #[test]
    fn test_null_coalesce_basic() {
        let e = parse_expr_only("a ?? b");
        // Should desugar to: if a != null { a } else { b }
        match e {
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                assert!(matches!(*cond, Expr::Binary { op: BinOp::Ne, .. }));
                assert_eq!(*then_branch, Expr::VarRef("a".to_string()));
                assert!(else_branch.is_some());
            }
            _ => panic!("expected if from ?? desugar, got {e:?}"),
        }
    }

    #[test]
    fn test_null_coalesce_chained() {
        // a ?? b ?? c → (a ?? b) ?? c → if (a!=null) { a } else { if (b!=null) { b } else { c } }
        let e = parse_expr_only("a ?? b ?? c");
        match e {
            Expr::If { .. } => {}
            _ => panic!("expected if, got {e:?}"),
        }
    }

    #[test]
    fn test_null_coalesce_with_assignment() {
        // x = a ?? "default" → x = if a != null { a } else { "default" }
        let e = parse_expr_only("x = a ?? \"default\"");
        match e {
            Expr::Assign { target, value } => {
                assert_eq!(*target, Expr::VarRef("x".to_string()));
                assert!(matches!(*value, Expr::If { .. }));
            }
            _ => panic!("expected assign with ??, got {e:?}"),
        }
    }

    // ── Optional chaining ──

    #[test]
    fn test_opt_chain_basic() {
        // a?.b → { var __o0=a; if __o0!=null { __o0.b } else { null } }
        let e = parse_expr_only("a?.b");
        match e {
            Expr::Block(stmts) => {
                assert_eq!(stmts.len(), 2);
                assert!(matches!(stmts[0], Stmt::VarDecl { .. }));
            }
            _ => panic!("expected block from ?., got {e:?}"),
        }
    }

    #[test]
    fn test_opt_chain_multi() {
        // a?.b?.c → two nested blocks
        let e = parse_expr_only("a?.b?.c");
        match e {
            Expr::Block(stmts) => {
                // outer block: var __o1 = (inner block); if ...
                match &stmts[0] {
                    Stmt::VarDecl { value, .. } => {
                        assert!(
                            matches!(value, Some(Expr::Block(_))),
                            "inner should be block from first ?."
                        );
                    }
                    _ => panic!("expected var decl"),
                }
            }
            _ => panic!("expected block from ?., got {e:?}"),
        }
    }

    // ── Struct spread ──

    #[test]
    fn test_struct_literal_with_spread() {
        let m = parse_mod("struct Point { x: Int64, y: Int64 }; const p2 = Point { ...p1, y: 3 };");
        match &m.stmts[1] {
            Stmt::ConstDecl { value, .. } => match value {
                Expr::StructLit {
                    name,
                    fields,
                    spread,
                } => {
                    assert_eq!(name, "Point");
                    assert_eq!(fields.len(), 1);
                    assert_eq!(fields[0].0, "y");
                    assert!(spread.is_some());
                }
                _ => panic!("expected StructLit, got {value:?}"),
            },
            _ => panic!(),
        }
    }

    // ── Match expressions ──

    #[test]
    fn test_match_basic() {
        let e = parse_expr_only("match (x) { 0 -> \"zero\", 1 -> \"one\", _ -> \"many\" }");
        // Should desugar to block with var + if/else chain
        match e {
            Expr::Block(stmts) => {
                assert_eq!(stmts.len(), 2);
                assert!(matches!(stmts[0], Stmt::VarDecl { .. }));
            }
            _ => panic!("expected block from match, got {e:?}"),
        }
    }

    // ── Template strings ──

    #[test]
    fn test_template_string_basic() {
        let e = parse_expr_only("`hello {name}`");
        // Should desugar to: "hello " + name.to_string()
        match e {
            Expr::Binary {
                op: BinOp::SAdd, ..
            } => {}
            _ => panic!("expected SAdd chain from template, got {e:?}"),
        }
    }

    #[test]
    fn test_template_string_multiple_exprs() {
        let e = parse_expr_only("`{a} + {b} = {a + b}`");
        match e {
            Expr::Binary {
                op: BinOp::SAdd, ..
            } => {}
            _ => panic!("expected SAdd chain, got {e:?}"),
        }
    }

    #[test]
    fn test_template_string_no_exprs() {
        let e = parse_expr_only("`hello world`");
        // Pure string, no interpolation
        match e {
            Expr::LitString(s) => assert_eq!(s, "hello world"),
            _ => panic!("expected LitString, got {e:?}"),
        }
    }

    #[test]
    fn test_opt_chain_index() {
        let e = parse_expr_only("a?[0]");
        match e {
            Expr::Block(_) => {}
            _ => panic!("expected block from ?[, got {e:?}"),
        }
    }

    // ── Await / Async ──

    #[test]
    fn test_await_expr() {
        let e = parse_expr_only("await f()");
        assert!(matches!(e, Expr::Await(_)));
    }

    #[test]
    fn test_async_await_chain() {
        let e = parse_expr_only("await async f()");
        match e {
            Expr::Await(inner) => {
                assert!(matches!(*inner, Expr::Async(_)));
            }
            _ => panic!("expected await async, got {e:?}"),
        }
    }

    // ── Print expression statement ──

    #[test]
    fn test_print_call_as_stmt() {
        let m = parse_mod("print(42);");
        match &m.stmts[0] {
            Stmt::ExprStmt(e) => {
                assert!(matches!(e, Expr::Call { .. }));
            }
            _ => panic!("expected expr stmt, got {:?}", m.stmts[0]),
        }
    }

    // ── Export expressions ──

    #[test]
    fn test_export_const() {
        let m = parse_mod("export const VERSION = 1;");
        assert!(matches!(&m.stmts[0], Stmt::ExportStmt(_)));
    }

    #[test]
    fn test_export_var() {
        let m = parse_mod("export var state = 0;");
        assert!(matches!(&m.stmts[0], Stmt::ExportStmt(_)));
    }
}
