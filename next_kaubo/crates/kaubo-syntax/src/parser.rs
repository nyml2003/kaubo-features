//! Parser — Kaubo v2
//!
//! 表达式导向，递归下降 + Pratt 运算符解析
//! `;` 为分隔符，block 最后一个表达式即返回值

use crate::token::{Token, TokenKind};
use crate::ast::*;
use crate::lexer::Lexer;

pub type ParseResult<T> = Result<T, String>;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(source: &str) -> Self {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        Self { tokens, pos: 0 }
    }

    pub fn from_tokens(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
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
            TokenKind::Impl => self.parse_impl(),
            TokenKind::Export => { self.bump(); Ok(Stmt::ExportStmt(Box::new(self.parse_top()?))) }
            TokenKind::Import => self.parse_import(),
            TokenKind::Semicolon => { self.bump(); self.parse_top() }
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
        Ok(Stmt::ConstDecl { name, ty_ann: ty, value: val })
    }

    fn parse_var(&mut self) -> ParseResult<Stmt> {
        self.bump(); // var
        let name = self.expect_ident()?;
        let ty = self.opt_type()?;
        let val = if self.current_kind() == TokenKind::Eq {
            self.bump();
            Some(self.parse_expr()?)
        } else { None };
        self.expect_semi()?;
        Ok(Stmt::VarDecl { name, ty_ann: ty, value: val })
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
            fields.push(FieldDef { name: fname, ty: fty });
            if self.current_kind() == TokenKind::Comma { self.bump(); }
        }
        self.bump(); // }
        self.skip_semis();
        Ok(Stmt::StructDef { name, fields })
    }

    fn parse_impl(&mut self) -> ParseResult<Stmt> {
        self.bump(); // impl
        let struct_name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut methods = Vec::new();
        while self.current_kind() != TokenKind::RBrace {
            let mname = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let body = self.parse_expr()?;
            methods.push(MethodDef { name: mname, body });
            if self.current_kind() == TokenKind::Comma { self.bump(); }
        }
        self.bump(); // }
        self.skip_semis();
        Ok(Stmt::ImplBlock { struct_name, methods })
    }

    fn parse_import(&mut self) -> ParseResult<Stmt> {
        self.bump(); // import
        if self.current_kind() == TokenKind::LBrace {
            self.bump();
            let mut names = Vec::new();
            while self.current_kind() != TokenKind::RBrace {
                names.push(self.expect_ident()?);
                if self.current_kind() == TokenKind::Comma { self.bump(); }
            }
            self.bump(); // }
            self.expect_kw(TokenKind::From)?;
            let path = self.expect_string()?;
            self.expect_semi()?;
            Ok(Stmt::Import { path, alias: None, names })
        } else {
            let path = self.expect_string()?;
            let alias = if self.current_kind() == TokenKind::As {
                self.bump();
                Some(self.expect_ident()?)
            } else { None };
            self.expect_semi()?;
            Ok(Stmt::Import { path, alias, names: vec![] })
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
            let (lbp, op) = match self.current_kind() {
                TokenKind::Semicolon | TokenKind::RBrace | TokenKind::RParen
                    | TokenKind::RBracket | TokenKind::Comma | TokenKind::Eof
                    | TokenKind::In | TokenKind::Else | TokenKind::Colon => break,
                TokenKind::Eq => (1, None),                       // = 赋值
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
                TokenKind::Asterisk => (5, Some(BinOp::Mul)),
                TokenKind::Slash => (5, Some(BinOp::Div)),
                TokenKind::Percent => (5, Some(BinOp::Mod)),
                _ => break,
            };
            if lbp < min_bp { break; }
            self.bump();

            let right_bp = if self.current_kind() == TokenKind::Eq { lbp - 1 } else { lbp };
            let right = self.parse_pratt(right_bp)?;

            if let None = op {
                left = Expr::Assign { target: Box::new(left), value: Box::new(right) };
            } else if let Some(binop) = op {
                left = Expr::Binary { left: Box::new(left), op: binop, right: Box::new(right) };
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
            TokenKind::True => { self.bump(); Ok(Expr::LitTrue) }
            TokenKind::False => { self.bump(); Ok(Expr::LitFalse) }
            TokenKind::Null => { self.bump(); Ok(Expr::LitNull) }

            TokenKind::Minus => {
                self.bump();
                let val = self.parse_pratt(10)?;
                Ok(Expr::Unary { op: UnOp::Neg, right: Box::new(val) })
            }
            TokenKind::Not => {
                self.bump();
                let val = self.parse_pratt(10)?;
                Ok(Expr::Unary { op: UnOp::Not, right: Box::new(val) })
            }

            TokenKind::Bar => self.parse_lambda(),

            TokenKind::LParen => {
                self.bump();
                let e = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(e)
            }
            TokenKind::LBrace => self.parse_block(),
            TokenKind::LBracket => self.parse_list(),

            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Break => { self.bump(); Ok(Expr::Break) }
            TokenKind::Continue => { self.bump(); Ok(Expr::Continue) }
            TokenKind::Return => {
                self.bump();
                Ok(Expr::Return(Some(Box::new(self.parse_expr()?))))
            }
            TokenKind::Async_ => {
                self.bump();
                Ok(Expr::Async(Box::new(self.parse_expr()?)))
            }
            TokenKind::Await => {
                self.bump();
                Ok(Expr::Await(Box::new(self.parse_expr()?)))
            }

            TokenKind::Identifier | TokenKind::Self_ => {
                Ok(Expr::VarRef(self.consume_lexeme()))
            }

            _ => Err(format!("unexpected token {:?} at {}:{}",
                self.current_kind(), self.current().line, self.current().col)),
        }
    }

    // ── 后缀链 (call / dot / index / struct literal) ──

    fn chain_postfix(&mut self, mut expr: Expr) -> ParseResult<Expr> {
        loop {
            match self.current_kind() {
                TokenKind::LParen => {
                    self.bump(); // consume (
                    let args = self.parse_call_args()?;
                    expr = Expr::Call { func: Box::new(expr), args };
                }
                TokenKind::Dot => {
                    self.bump();
                    let field = self.expect_ident()?;
                    expr = Expr::Member { object: Box::new(expr), field };
                }
                TokenKind::LBracket => {
                    self.bump();
                    let index = self.parse_expr()?;
                    self.expect(TokenKind::RBracket)?;
                    expr = Expr::Index { object: Box::new(expr), index: Box::new(index) };
                }
                TokenKind::LBrace => {
                    if let Expr::VarRef(ref name) = expr {
                        if name.chars().next().map_or(false, |c| c.is_uppercase()) {
                            let struct_name = name.clone();
                            self.bump();
                            let mut fields = Vec::new();
                            while self.current_kind() != TokenKind::RBrace {
                                let fname = self.expect_ident()?;
                                self.expect(TokenKind::Colon)?;
                                let val = self.parse_expr()?;
                                fields.push((fname, val));
                                if self.current_kind() == TokenKind::Comma { self.bump(); }
                            }
                            self.bump();
                            expr = Expr::StructLit { name: struct_name, fields };
                        } else { break; }
                    } else { break; }
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
            if self.current_kind() == TokenKind::Comma { self.bump(); }
        }
        self.bump(); // RParen
        Ok(args)
    }

    // ── 表达式子解析 ──

    fn parse_int(&mut self) -> ParseResult<Expr> {
        let s = self.consume_lexeme();
        s.parse::<i64>().map(Expr::LitInt).map_err(|_| format!("invalid int: {}", s))
    }

    fn parse_float(&mut self) -> ParseResult<Expr> {
        let s = self.consume_lexeme();
        s.parse::<f64>().map(Expr::LitFloat).map_err(|_| format!("invalid float: {}", s))
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
            params.push(Param { name: pname, ty_ann: ty });
            if self.current_kind() == TokenKind::Comma { self.bump(); }
        }
        self.bump(); // closing |
        let ret_ty = if self.current_kind() == TokenKind::FatArrow {
            self.bump();
            Some(self.parse_type()?)
        } else { None };
        let body = self.parse_expr()?;
        Ok(Expr::Lambda { params, ret_ty, body: Box::new(body) })
    }

    fn parse_block(&mut self) -> ParseResult<Expr> {
        self.bump(); // {
        let mut stmts = Vec::new();
        while self.current_kind() != TokenKind::RBrace {
            match self.current_kind() {
                TokenKind::Const => stmts.push(self.parse_const()?),
                TokenKind::Var => stmts.push(self.parse_var()?),
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
            if self.current_kind() == TokenKind::Comma { self.bump(); }
        }
        self.bump(); // ]
        Ok(Expr::ListLit(items))
    }

    fn parse_if(&mut self) -> ParseResult<Expr> {
        self.bump();
        let cond = self.parse_expr()?;
        let then_b = self.parse_expr()?;
        let else_b = if self.current_kind() == TokenKind::Else {
            self.bump();
            Some(Box::new(self.parse_expr()?))
        } else { None };
        Ok(Expr::If { cond: Box::new(cond), then_branch: Box::new(then_b), else_branch: else_b })
    }

    fn parse_while(&mut self) -> ParseResult<Expr> {
        self.bump();
        let cond = self.parse_expr()?;
        let body = self.parse_expr()?;
        Ok(Expr::While { cond: Box::new(cond), body: Box::new(body) })
    }

    fn parse_for(&mut self) -> ParseResult<Expr> {
        self.bump();
        let varname = self.expect_ident()?;
        self.expect_kw(TokenKind::In).map_err(|_| "expected 'in' in for loop".to_string())?;
        let iterable = self.parse_expr()?;
        let body = self.parse_expr()?;
        Ok(Expr::For { var: Param { name: varname, ty_ann: None }, iterable: Box::new(iterable), body: Box::new(body) })
    }

    // ── 类型 ──

    fn parse_type(&mut self) -> ParseResult<TypeExpr> {
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
        } else { Ok(None) }
    }

    // ── 辅助 ──

    fn current(&self) -> &Token { &self.tokens[self.pos] }
    fn current_kind(&self) -> TokenKind { self.current().kind }

    fn is_eof(&self) -> bool { self.current_kind() == TokenKind::Eof }

    fn bump(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
        self.pos += 1;
        t
    }

    fn consume_lexeme(&mut self) -> String { self.bump().lexeme.clone() }

    fn expect_ident(&mut self) -> ParseResult<String> {
        if matches!(self.current_kind(), TokenKind::Identifier | TokenKind::Self_) {
            Ok(self.consume_lexeme())
        } else {
            Err(format!("expected ident at {}:{}, got {:?}", self.current().line, self.current().col, self.current_kind()))
        }
    }

    fn expect_string(&mut self) -> ParseResult<String> {
        if self.current_kind() == TokenKind::StringLiteral {
            Ok(self.consume_lexeme())
        } else {
            Err(format!("expected string at {}:{}", self.current().line, self.current().col))
        }
    }

    fn expect(&mut self, kind: TokenKind) -> ParseResult<()> {
        if self.current_kind() == kind { self.bump(); Ok(()) }
        else { Err(format!("expected {:?} at {}:{}, got {:?}", kind, self.current().line, self.current().col, self.current_kind())) }
    }

    fn expect_kw(&mut self, kind: TokenKind) -> ParseResult<()> { self.expect(kind) }

    fn expect_semi(&mut self) -> ParseResult<()> {
        if self.current_kind() == TokenKind::Semicolon { self.bump(); Ok(()) }
        else { Err(format!("expected ; at {}:{}, got {:?}", self.current().line, self.current().col, self.current_kind())) }
    }

    fn skip_semis(&mut self) {
        while self.current_kind() == TokenKind::Semicolon { self.bump(); }
    }
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
        let rtk = p.current().kind; assert!(p.is_eof(), "extra tokens");
        e
    }

    #[test]
    fn test_const() {
        let m = parse_mod("const x = 42;");
        match &m.stmts[0] { Stmt::ConstDecl { name, value, .. } => { assert_eq!(name, "x"); assert!(matches!(value, Expr::LitInt(42))); } _ => panic!() }
    }

    #[test]
    fn test_var_with_type() {
        let m = parse_mod("var counter: Int64 = 0;");
        match &m.stmts[0] { Stmt::VarDecl { name, ty_ann, .. } => { assert_eq!(name, "counter"); assert!(ty_ann.is_some()); } _ => panic!() }
    }

    #[test]
    fn test_lambda() {
        let m = parse_mod("const add = |a, b| { a + b };");
        match &m.stmts[0] { Stmt::ConstDecl { name, value, .. } => { assert_eq!(name, "add"); assert!(matches!(value, Expr::Lambda { .. })); } _ => panic!() }
    }

    #[test]
    fn test_lambda_with_return_type() {
        let m = parse_mod("const greet = |name| -> String { \"Hello\" };");
        match &m.stmts[0] { Stmt::ConstDecl { value, .. } => { if let Expr::Lambda { ret_ty, .. } = value { assert!(ret_ty.is_some()); } } _ => panic!() }
    }

    #[test]
    fn test_struct_def() {
        let m = parse_mod("struct Point { x: Float64, y: Float64 }");
        match &m.stmts[0] { Stmt::StructDef { name, fields } => { assert_eq!(name, "Point"); assert_eq!(fields.len(), 2); } _ => panic!() }
    }

    #[test]
    fn test_impl_block() {
        let m = parse_mod("impl Point { dist: |self, other| -> Float64 { return 0.0; } }");
        match &m.stmts[0] { Stmt::ImplBlock { struct_name, methods } => { assert_eq!(struct_name, "Point"); assert_eq!(methods.len(), 1); } _ => panic!() }
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
        let e = parse_expr_only("if x < 0 { -x } else { x }");
        assert!(matches!(e, Expr::If { .. }));
    }

    #[test]
    fn test_while_loop() {
        let e = parse_expr_only("while i < 10 { i = i + 1 }");
        assert!(matches!(e, Expr::While { .. }));
    }

    #[test]
    fn test_pipe() {
        let e = parse_expr_only("a |> f |> g");
        assert!(matches!(e, Expr::Binary { op: BinOp::Pipe, .. }));
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
        match &m.stmts[0] { Stmt::ConstDecl { name, value, .. } => { assert_eq!(name, "result"); assert!(matches!(value, Expr::Block(_))); } _ => panic!() }
    }

    #[test]
    #[test] fn test_for_loop() { // TODO: fix for-loop in struct context
        let e = parse_expr_only("for x in xs { print(x) }"); assert!(matches!(e, Expr::For { .. })); }

    #[test]
    fn test_struct_literal() {
        let e = parse_expr_only("Point { x: 100, y: 200 }");
        assert!(matches!(e, Expr::StructLit { .. }));
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
}
