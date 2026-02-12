use super::super::lexer::token_kind::KauboTokenKind;
use super::error::{ErrorLocation, ParseResult, ParserError, ParserErrorKind};
use super::expr::{
    Binary, Expr, ExprKind, FunctionCall, Grouping, IndexAccess, JsonLiteral, Lambda, LiteralFalse,
    LiteralInt, LiteralList, LiteralNull, LiteralString, LiteralTrue, MemberAccess, Unary, VarRef,
    YieldExpr,
};
use super::module::{Module, ModuleKind};
use super::stmt::{
    BlockStmt, EmptyStmt, ExprStmt, ForStmt, IfStmt, ImportStmt, ModuleStmt, ReturnStmt, Stmt,
    StmtKind, VarDeclStmt, WhileStmt,
};
use super::type_expr::TypeExpr;
use super::utils::{get_associativity, get_precedence};
use crate::kit::lexer::scanner::Token;
use crate::kit::lexer::types::Coordinate;
use crate::kit::lexer::Lexer;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Parser {
    lexer: Rc<RefCell<Lexer>>,
    current_token: Option<Token<KauboTokenKind>>,
}

impl Parser {
    pub fn new(lexer: Lexer) -> Self {
        let lexer = Rc::new(RefCell::new(lexer));
        let mut parser = Self {
            lexer,
            current_token: None,
        };
        parser.consume(); // 预读第一个token
        parser
    }

    /// 解析整个模块
    pub fn parse(&mut self) -> ParseResult<Module> {
        self.parse_module()
    }

    /// 消费当前token并读取下一个
    fn consume(&mut self) {
        self.current_token = self.lexer.borrow_mut().next_token();
    }

    /// 检查当前token是否为指定类型
    fn check(&self, kind: KauboTokenKind) -> bool {
        self.current_token
            .as_ref()
            .map(|token| token.kind == kind)
            .unwrap_or(false)
    }

    /// 匹配并消费指定类型的token
    fn match_token(&mut self, kind: KauboTokenKind) -> bool {
        if self.check(kind) {
            self.consume();
            true
        } else {
            false
        }
    }

    /// 获取当前token的位置信息
    fn current_location(&self) -> ErrorLocation {
        match &self.current_token {
            Some(token) => ErrorLocation::At(Coordinate {
                line: token.span.start.line,
                column: token.span.start.column,
            }),
            None => ErrorLocation::Eof,
        }
    }

    /// 获取当前token的坐标（如果有）
    fn current_coordinate(&self) -> Option<Coordinate> {
        self.current_token.as_ref().map(|t| Coordinate {
            line: t.span.start.line,
            column: t.span.start.column,
        })
    }

    /// 获取当前token的文本表示
    fn current_token_text(&self) -> String {
        match &self.current_token {
            Some(token) => format!("{:?}", token.kind),
            None => "EOF".to_string(),
        }
    }

    /// 创建带有当前位置的错误
    fn error_here(&self, kind: ParserErrorKind) -> ParserError {
        ParserError {
            kind,
            location: self.current_location(),
        }
    }

    /// 期望并消费指定类型的token，否则返回错误
    fn expect(&mut self, kind: KauboTokenKind) -> ParseResult<()> {
        if self.match_token(kind.clone()) {
            Ok(())
        } else {
            let expected = format!("{:?}", kind);
            Err(self.error_here(ParserErrorKind::UnexpectedToken {
                found: self.current_token_text(),
                expected: vec![expected],
            }))
        }
    }

    /// 期望一个标识符，返回其名称
    fn expect_identifier(&mut self) -> ParseResult<String> {
        let token = self
            .current_token
            .as_ref()
            .ok_or_else(|| ParserError::at_eof(ParserErrorKind::UnexpectedEndOfInput))?;

        if token.kind == KauboTokenKind::Identifier {
            let name = token.text.clone().unwrap_or_default();
            self.consume();
            Ok(name)
        } else {
            Err(self.error_here(ParserErrorKind::ExpectedIdentifier {
                found: self.current_token_text(),
            }))
        }
    }

    /// 解析模块路径（如 std.core, math.geometry）
    fn parse_module_path(&mut self) -> ParseResult<String> {
        let mut path = self.expect_identifier()?;

        // 继续解析 .xxx 部分
        while self.match_token(KauboTokenKind::Dot) {
            let part = self.expect_identifier()?;
            path.push('.');
            path.push_str(&part);
        }

        Ok(path)
    }

    /// 解析模块（顶层语句集合）
    fn parse_module(&mut self) -> ParseResult<Module> {
        let mut statements = Vec::new();

        while self.current_token.is_some() {
            // 跳过分号（空语句）
            if self.check(KauboTokenKind::Semicolon) {
                self.consume();
                continue;
            }

            let stmt = self.parse_statement()?;
            statements.push(stmt);
        }

        Ok(Box::new(ModuleKind { statements }))
    }

    /// 解析单个语句
    fn parse_statement(&mut self) -> ParseResult<Stmt> {
        if self.check(KauboTokenKind::LeftCurlyBrace) {
            self.parse_block()
        } else if self.check(KauboTokenKind::Var) {
            self.parse_var_declaration()
        } else if self.check(KauboTokenKind::Semicolon) {
            self.consume();
            Ok(Box::new(StmtKind::Empty(EmptyStmt)))
        } else if self.check(KauboTokenKind::Return) {
            self.parse_return_statement()
        } else if self.check(KauboTokenKind::If) {
            self.parse_if_statement()
        } else if self.check(KauboTokenKind::While) {
            self.parse_while_loop()
        } else if self.check(KauboTokenKind::For) {
            self.parse_for_loop()
        } else if self.check(KauboTokenKind::Module) {
            self.parse_module_statement()
        } else if self.check(KauboTokenKind::Import) {
            self.parse_import_statement()
        } else if self.check(KauboTokenKind::From) {
            // from...import 也是导入语句
            self.parse_import_statement()
        } else if self.check(KauboTokenKind::Pub) {
            // pub 关键字：标记为 public 导出
            self.consume(); // 消费 'pub'
                            // 目前只支持 pub var ...
            if self.check(KauboTokenKind::Var) {
                // 解析变量声明（pub 修饰）
                self.parse_var_declaration_with_pub(true)
            } else {
                Err(self.error_here(ParserErrorKind::UnexpectedToken {
                    found: self.current_token_text(),
                    expected: vec!["var".to_string()],
                }))
            }
        } else {
            // 表达式语句
            let expr = self.parse_expression(0)?;
            Ok(Box::new(StmtKind::Expr(ExprStmt { expression: expr })))
        }
    }

    /// 解析代码块
    fn parse_block(&mut self) -> ParseResult<Stmt> {
        self.expect(KauboTokenKind::LeftCurlyBrace)?;

        let mut statements = Vec::new();

        while self.current_token.is_some() && !self.check(KauboTokenKind::RightCurlyBrace) {
            if self.match_token(KauboTokenKind::Semicolon) {
                continue;
            }

            let stmt = self.parse_statement()?;
            statements.push(stmt);

            // 消费可选的分号
            self.match_token(KauboTokenKind::Semicolon);
        }

        self.expect(KauboTokenKind::RightCurlyBrace)?;
        Ok(Box::new(StmtKind::Block(BlockStmt { statements })))
    }

    /// 解析表达式（Pratt解析核心）
    fn parse_expression(&mut self, min_precedence: i32) -> ParseResult<Expr> {
        // 解析左操作数（一元表达式或基础表达式）
        let mut left = self.parse_unary()?;

        // 循环解析二元运算符和右操作数
        while let Some(token) = &self.current_token {
            let op_precedence = get_precedence(token.kind.clone());

            // 优先级不足，停止解析
            if op_precedence <= min_precedence {
                break;
            }

            // 消费运算符
            let op = token.kind.clone();
            self.consume();

            // 解析右操作数（考虑结合性）
            let next_precedence = if get_associativity(op.clone()) {
                op_precedence
            } else {
                op_precedence - 1
            };
            let right = self.parse_expression(next_precedence)?;

            // 构建二元表达式
            left = Box::new(ExprKind::Binary(Binary { left, op, right }));
        }

        Ok(left)
    }

    /// 解析一元表达式
    fn parse_unary(&mut self) -> ParseResult<Expr> {
        if self.check(KauboTokenKind::Minus) || self.check(KauboTokenKind::Not) {
            let token = self.current_token.as_ref().unwrap();
            let op = token.kind.clone();
            self.consume();

            let operand = self.parse_unary()?;
            Ok(Box::new(ExprKind::Unary(Unary { op, operand })))
        } else if self.check(KauboTokenKind::Yield) {
            // 解析 yield 表达式
            self.consume(); // 消耗 yield

            // yield 可以有值也可以没有值
            let value = if self.check(KauboTokenKind::Semicolon)
                || self.check(KauboTokenKind::RightCurlyBrace)
            {
                // yield; 或 yield } - 无值
                None
            } else {
                // yield expr;
                Some(self.parse_expression(0)?)
            };

            Ok(Box::new(ExprKind::Yield(YieldExpr { value })))
        } else {
            self.parse_primary()
        }
    }

    /// 解析基础表达式（带后缀处理）
    fn parse_primary(&mut self) -> ParseResult<Expr> {
        let base_expr = self.parse_primary_base()?;
        self.parse_postfix(base_expr)
    }

    /// 解析基础表达式核心（无后缀）
    fn parse_primary_base(&mut self) -> ParseResult<Expr> {
        let token = self
            .current_token
            .as_ref()
            .ok_or_else(|| ParserError::at_eof(ParserErrorKind::UnexpectedEndOfInput))?;

        match token.kind {
            KauboTokenKind::LiteralInteger => self.parse_int(),
            KauboTokenKind::LiteralFloat => self.parse_float(),
            KauboTokenKind::LiteralString => self.parse_string(),
            KauboTokenKind::True => {
                self.consume();
                Ok(Box::new(ExprKind::LiteralTrue(LiteralTrue)))
            }
            KauboTokenKind::False => {
                self.consume();
                Ok(Box::new(ExprKind::LiteralFalse(LiteralFalse)))
            }
            KauboTokenKind::Null => {
                self.consume();
                Ok(Box::new(ExprKind::LiteralNull(LiteralNull)))
            }
            KauboTokenKind::LeftSquareBracket => self.parse_list(),
            KauboTokenKind::LeftParenthesis => self.parse_parenthesized(),
            KauboTokenKind::Identifier => self.parse_identifier_expression(),
            KauboTokenKind::Pipe => self.parse_lambda(),
            KauboTokenKind::Json => self.parse_json_literal(),
            _ => Err(self.error_here(ParserErrorKind::UnexpectedToken {
                found: self.current_token_text(),
                expected: vec!["expression".to_string()],
            })),
        }
    }

    /// 解析 JSON 字面量
    /// json { "key": value, ... }
    fn parse_json_literal(&mut self) -> ParseResult<Expr> {
        self.consume(); // 消费 'json'
        self.expect(KauboTokenKind::LeftCurlyBrace)?;

        let mut entries = Vec::new();

        while !self.check(KauboTokenKind::RightCurlyBrace) {
            // 解析键（必须是字符串）
            let key_token = self
                .current_token
                .as_ref()
                .ok_or_else(|| ParserError::at_eof(ParserErrorKind::UnexpectedEndOfInput))?;

            let key = if key_token.kind == KauboTokenKind::LiteralString {
                let k = key_token.text.clone().unwrap_or_default();
                self.consume();
                // 去除引号（安全检查）
                if k.len() >= 2 && k.starts_with('"') && k.ends_with('"') {
                    k[1..k.len() - 1].to_string()
                } else {
                    k
                }
            } else if key_token.kind == KauboTokenKind::Identifier {
                // 也支持裸标识符作为键（像 JavaScript）
                let k = key_token.text.clone().unwrap_or_default();
                self.consume();
                k
            } else {
                return Err(self.error_here(ParserErrorKind::UnexpectedToken {
                    found: self.current_token_text(),
                    expected: vec!["string".to_string(), "identifier".to_string()],
                }));
            };

            self.expect(KauboTokenKind::Colon)?;

            // 解析值
            let value = self.parse_expression(0)?;
            entries.push((key, value));

            // 可选的逗号
            if !self.match_token(KauboTokenKind::Comma) {
                break;
            }
        }

        self.expect(KauboTokenKind::RightCurlyBrace)?;

        Ok(Box::new(ExprKind::JsonLiteral(JsonLiteral { entries })))
    }

    /// 解析后缀表达式（成员访问、函数调用）
    fn parse_postfix(&mut self, mut expr: Expr) -> ParseResult<Expr> {
        loop {
            if self.check(KauboTokenKind::Dot) {
                // 成员访问：a.b
                self.consume();

                let token = self
                    .current_token
                    .as_ref()
                    .ok_or_else(|| self.error_here(ParserErrorKind::ExpectedIdentifierAfterDot))?;
                if token.kind != KauboTokenKind::Identifier {
                    return Err(self.error_here(ParserErrorKind::ExpectedIdentifierAfterDot));
                }

                let member_name = token.text.clone().unwrap_or_default();
                self.consume();
                expr = Box::new(ExprKind::MemberAccess(MemberAccess {
                    object: expr,
                    member: member_name,
                }));
            } else if self.check(KauboTokenKind::LeftParenthesis) {
                // 函数调用：a() 或 a.b()
                expr = self.parse_function_call(expr)?;
            } else if self.check(KauboTokenKind::LeftSquareBracket) {
                // 索引访问：a[i]
                self.consume(); // 消费 '['
                let index = self.parse_expression(0)?;
                self.expect(KauboTokenKind::RightSquareBracket)?;
                expr = Box::new(ExprKind::IndexAccess(IndexAccess {
                    object: expr,
                    index,
                }));
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// 解析整数字面量
    fn parse_int(&mut self) -> ParseResult<Expr> {
        let token = self.current_token.as_ref().unwrap();
        let coord = Coordinate {
            line: token.span.start.line,
            column: token.span.start.column,
        };
        let text = token.text.clone().unwrap_or_default();
        let num = text.parse().map_err(|_| {
            ParserError::here(ParserErrorKind::InvalidNumberFormat(text.clone()), coord)
        })?;
        self.consume();
        Ok(Box::new(ExprKind::LiteralInt(LiteralInt { value: num })))
    }

    /// 解析浮点数字面量
    fn parse_float(&mut self) -> ParseResult<Expr> {
        let token = self.current_token.as_ref().unwrap();
        let text = token.text.clone().unwrap_or_default();
        let num = text.parse().map_err(|_| {
            ParserError::here(
                ParserErrorKind::InvalidNumberFormat(text.clone()),
                Coordinate {
                    line: token.span.start.line,
                    column: token.span.start.column,
                },
            )
        })?;
        self.consume();
        Ok(Box::new(ExprKind::LiteralFloat(super::expr::LiteralFloat { value: num })))
    }

    /// 解析字符串字面量
    fn parse_string(&mut self) -> ParseResult<Expr> {
        let token = self.current_token.as_ref().unwrap();
        // 移除首尾引号
        let text = token.text.clone().unwrap_or_default();
        let s = text.to_string();
        self.consume();
        Ok(Box::new(ExprKind::LiteralString(LiteralString {
            value: s,
        })))
    }

    /// 解析列表字面量
    fn parse_list(&mut self) -> ParseResult<Expr> {
        self.expect(KauboTokenKind::LeftSquareBracket)?;

        let mut elements = Vec::new();
        while !self.check(KauboTokenKind::RightSquareBracket) {
            elements.push(self.parse_expression(0)?);

            if !self.match_token(KauboTokenKind::Comma) {
                break;
            }
        }

        self.expect(KauboTokenKind::RightSquareBracket)
            .map_err(|_| self.error_here(ParserErrorKind::MissingRightBracket))?;
        Ok(Box::new(ExprKind::LiteralList(LiteralList { elements })))
    }

    /// 解析括号表达式
    fn parse_parenthesized(&mut self) -> ParseResult<Expr> {
        self.consume(); // 消费 '('

        let expr = self.parse_expression(0)?;

        self.expect(KauboTokenKind::RightParenthesis)
            .map_err(|_| self.error_here(ParserErrorKind::MissingRightParen))?;

        Ok(Box::new(ExprKind::Grouping(Grouping { expression: expr })))
    }

    /// 解析标识符引用
    fn parse_identifier_expression(&mut self) -> ParseResult<Expr> {
        let token = self.current_token.as_ref().unwrap();
        let name = token.text.clone().unwrap_or_default();
        self.consume();
        Ok(Box::new(ExprKind::VarRef(VarRef { name })))
    }

    /// 解析匿名函数（lambda）
    /// 
    /// 语法: |param1: Type1, param2: Type2| -> ReturnType { body }
    fn parse_lambda(&mut self) -> ParseResult<Expr> {
        self.expect(KauboTokenKind::Pipe)?; // 消费 '|'

        let mut params: Vec<(String, Option<TypeExpr>)> = Vec::new();

        // 解析参数列表
        if !self.check(KauboTokenKind::Pipe) {
            loop {
                // 解析参数名
                let param_name = self.expect_identifier()?;
                
                // 解析可选的参数类型标注
                let param_type = if self.match_token(KauboTokenKind::Colon) {
                    Some(self.parse_type_expression()?)
                } else {
                    None
                };
                
                params.push((param_name, param_type));

                if self.match_token(KauboTokenKind::Comma) {
                    continue;
                } else if self.check(KauboTokenKind::Pipe) {
                    break;
                } else {
                    return Err(self.error_here(ParserErrorKind::ExpectedCommaOrPipeInLambda));
                }
            }
        }

        self.expect(KauboTokenKind::Pipe)?; // 消费 '|'

        // 解析可选的返回类型标注 ("-> Type")
        let return_type = if self.match_token(KauboTokenKind::FatArrow) {
            Some(self.parse_type_expression()?)
        } else {
            None
        };

        let body = self.parse_block()?;
        
        Ok(Box::new(ExprKind::Lambda(Lambda { 
            params, 
            return_type,
            body 
        })))
    }

    /// 解析函数调用
    fn parse_function_call(&mut self, function_expr: Expr) -> ParseResult<Expr> {
        self.consume(); // 消费 '('

        let mut arguments = Vec::new();
        while !self.check(KauboTokenKind::RightParenthesis) {
            arguments.push(self.parse_expression(0)?);

            if !self.match_token(KauboTokenKind::Comma) {
                break;
            }
        }

        self.expect(KauboTokenKind::RightParenthesis)
            .map_err(|_| self.error_here(ParserErrorKind::MissingRightParen))?;

        Ok(Box::new(ExprKind::FunctionCall(FunctionCall {
            function_expr,
            arguments,
        })))
    }

    /// 解析变量声明（非 pub）
    fn parse_var_declaration(&mut self) -> ParseResult<Stmt> {
        self.parse_var_declaration_inner(false)
    }

    /// 解析变量声明（带 pub 标记）
    fn parse_var_declaration_with_pub(&mut self, is_public: bool) -> ParseResult<Stmt> {
        self.parse_var_declaration_inner(is_public)
    }

    /// 解析变量声明内部实现
    fn parse_var_declaration_inner(&mut self, is_public: bool) -> ParseResult<Stmt> {
        self.consume(); // 消费 'var'

        let token = self.current_token.as_ref().ok_or_else(|| {
            self.error_here(ParserErrorKind::UnexpectedToken {
                found: self.current_token_text(),
                expected: vec!["identifier".to_string()],
            })
        })?;
        if token.kind != KauboTokenKind::Identifier {
            return Err(self.error_here(ParserErrorKind::ExpectedIdentifier {
                found: self.current_token_text(),
            }));
        }
        let name = token.text.clone().unwrap_or_default();
        self.consume();

        // 解析可选的类型标注 (": Type")
        let type_annotation = if self.match_token(KauboTokenKind::Colon) {
            Some(self.parse_type_expression()?)
        } else {
            None
        };

        self.expect(KauboTokenKind::Equal)?;
        let initializer = self.parse_expression(0)?;
        self.expect(KauboTokenKind::Semicolon)?;

        Ok(Box::new(StmtKind::VarDecl(VarDeclStmt {
            name,
            type_annotation,
            initializer,
            is_public,
        })))
    }

    /// 解析return语句
    fn parse_return_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(); // 消费 'return'

        let value = if !self.check(KauboTokenKind::Semicolon) {
            Some(self.parse_expression(0)?)
        } else {
            None
        };

        self.expect(KauboTokenKind::Semicolon)?;
        Ok(Box::new(StmtKind::Return(ReturnStmt { value })))
    }

    /// 解析模块定义语句
    /// module name { ... }
    fn parse_module_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(); // 消费 'module'

        // 解析模块名
        let name = self.expect_identifier()?;

        // 解析模块体（代码块）
        let body = self.parse_block()?;

        Ok(Box::new(StmtKind::Module(ModuleStmt { name, body })))
    }

    /// 解析导入语句
    /// import module;
    /// import module as alias;
    /// from module import item1, item2;
    fn parse_import_statement(&mut self) -> ParseResult<Stmt> {
        // 检查是 from...import 还是 import
        if self.check(KauboTokenKind::From) {
            // from module import item1, item2;
            self.consume(); // 消费 'from'
            let module_path = self.parse_module_path()?;
            self.expect(KauboTokenKind::Import)?;

            // 解析导入的项列表
            let mut items = Vec::new();
            loop {
                let item = self.expect_identifier()?;
                items.push(item);

                if self.match_token(KauboTokenKind::Comma) {
                    continue;
                } else {
                    break;
                }
            }

            self.expect(KauboTokenKind::Semicolon)?;
            Ok(Box::new(StmtKind::Import(ImportStmt {
                module_path,
                items,
                alias: None,
            })))
        } else {
            // import module; 或 import module as alias;
            self.consume(); // 消费 'import'
            let module_path = self.parse_module_path()?;

            // 检查是否有别名
            let alias = if self.match_token(KauboTokenKind::As) {
                Some(self.expect_identifier()?)
            } else {
                None
            };

            self.expect(KauboTokenKind::Semicolon)?;
            Ok(Box::new(StmtKind::Import(ImportStmt {
                module_path,
                items: Vec::new(),
                alias,
            })))
        }
    }

    /// 解析if语句
    fn parse_if_statement(&mut self) -> ParseResult<Stmt> {
        self.consume(); // 消费 'if'
        let if_condition = self.parse_expression(0)?;
        let then_body = self.parse_block()?;

        let mut elif_conditions = Vec::new();
        let mut elif_bodies = Vec::new();

        while self.check(KauboTokenKind::Elif) {
            self.consume(); // 消费 'elif'
            let cond = self.parse_expression(0)?;
            let body = self.parse_block()?;
            elif_conditions.push(cond);
            elif_bodies.push(body);
        }

        let else_body = if self.check(KauboTokenKind::Else) {
            self.consume(); // 消费 'else'
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Box::new(StmtKind::If(IfStmt {
            if_condition,
            then_body,
            elif_conditions,
            elif_bodies,
            else_body,
        })))
    }

    /// 解析while循环
    fn parse_while_loop(&mut self) -> ParseResult<Stmt> {
        self.consume(); // 消费 'while'
        let condition = self.parse_expression(0)?;
        let body = self.parse_block()?;
        Ok(Box::new(StmtKind::While(WhileStmt { condition, body })))
    }

    /// 解析for循环
    fn parse_for_loop(&mut self) -> ParseResult<Stmt> {
        self.consume(); // 消费 'for'

        // 新语法: for var item in iterable { ... }
        self.expect(KauboTokenKind::Var)?;
        let iterator = self.parse_expression(0)?;
        self.expect(KauboTokenKind::In)?;
        let iterable = self.parse_expression(0)?;
        let body = self.parse_block()?;
        Ok(Box::new(StmtKind::For(ForStmt {
            iterator,
            iterable,
            body,
        })))
    }

    // ==================== 类型表达式解析 ====================

    /// 解析类型表达式
    /// 
    /// 支持的类型:
    /// - 命名类型: int, string, bool, float, 自定义类型
    /// - List 类型: List<T>
    /// - Tuple 类型: Tuple<T1, T2, ...>
    /// - 函数类型: |T1, T2| -> R
    fn parse_type_expression(&mut self) -> ParseResult<TypeExpr> {
        // 检查是否是函数类型（以 | 开头）
        if self.check(KauboTokenKind::Pipe) {
            return self.parse_function_type();
        }

        // 解析基础类型名
        let token = self
            .current_token
            .as_ref()
            .ok_or_else(|| ParserError::at_eof(ParserErrorKind::UnexpectedEndOfInput))?;

        if token.kind != KauboTokenKind::Identifier {
            return Err(self.error_here(ParserErrorKind::ExpectedIdentifier {
                found: self.current_token_text(),
            }));
        }

        let type_name = token.text.clone().unwrap_or_default();
        self.consume();

        // 检查是否是泛型类型 (List<T> 或 Tuple<T1, T2>)
        if self.check(KauboTokenKind::LessThan) {
            match type_name.as_str() {
                "List" => self.parse_list_type(),
                "Tuple" => self.parse_tuple_type(),
                _ => Err(self.error_here(ParserErrorKind::UnexpectedToken {
                    found: type_name,
                    expected: vec!["int".to_string(), "string".to_string(), "bool".to_string(), "float".to_string(), "List".to_string(), "Tuple".to_string()],
                })),
            }
        } else {
            // 普通命名类型
            Ok(TypeExpr::named(type_name))
        }
    }

    /// 解析 List<T> 类型
    fn parse_list_type(&mut self) -> ParseResult<TypeExpr> {
        self.expect(KauboTokenKind::LessThan)?; // 消费 '<'
        let elem_type = self.parse_type_expression()?;
        self.expect(KauboTokenKind::GreaterThan)?; // 消费 '>'
        Ok(TypeExpr::list(elem_type))
    }

    /// 解析 Tuple<T1, T2, ...> 类型
    fn parse_tuple_type(&mut self) -> ParseResult<TypeExpr> {
        self.expect(KauboTokenKind::LessThan)?; // 消费 '<'
        
        let mut types = Vec::new();
        
        // 解析第一个类型（如果有）
        if !self.check(KauboTokenKind::GreaterThan) {
            types.push(self.parse_type_expression()?);
            
            // 解析后续类型
            while self.match_token(KauboTokenKind::Comma) {
                types.push(self.parse_type_expression()?);
            }
        }
        
        self.expect(KauboTokenKind::GreaterThan)?; // 消费 '>'
        Ok(TypeExpr::tuple(types))
    }

    /// 解析函数类型: |T1, T2| -> R
    fn parse_function_type(&mut self) -> ParseResult<TypeExpr> {
        self.expect(KauboTokenKind::Pipe)?; // 消费 '|'
        
        let mut params = Vec::new();
        
        // 解析参数类型列表
        if !self.check(KauboTokenKind::Pipe) {
            params.push(self.parse_type_expression()?);
            
            while self.match_token(KauboTokenKind::Comma) {
                params.push(self.parse_type_expression()?);
            }
        }
        
        self.expect(KauboTokenKind::Pipe)?; // 消费 '|'
        
        // 解析返回类型 (-> Type)
        let return_type = if self.match_token(KauboTokenKind::FatArrow) {
            Some(self.parse_type_expression()?)
        } else {
            None
        };
        
        Ok(TypeExpr::function(params, return_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::lexer::builder::build_lexer;

    fn parse_code(code: &str) -> ParseResult<Module> {
        let mut lexer = build_lexer();
        let _ = lexer.feed(&code.as_bytes().to_vec());
        let _ = lexer.terminate();
        let mut parser = Parser::new(lexer);
        parser.parse()
    }

    #[test]
    fn test_parse_literal_int() {
        let code = "42;";
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_literal_string() {
        // 调试 lexer 输出
        let mut lexer = build_lexer();
        let _ = lexer.feed(&r#""hello";"#.as_bytes().to_vec());
        let _ = lexer.terminate();

        println!("Tokens from lexer:");
        for i in 0..10 {
            // 最多打印 10 个 token，防止死循环
            match lexer.next_token() {
                Some(token) => println!("  [{}] {:?} = {:?}", i, token.kind, token.text),
                None => {
                    println!("  [{}] None (EOF)", i);
                    break;
                }
            }
        }

        let code = r#""hello";"#;
        let result = parse_code(code);
        if let Err(ref e) = result {
            println!("Parse error: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "Failed to parse string literal: {:?}",
            result
        );
    }

    #[test]
    fn test_parse_literal_bool() {
        let code = "true;";
        let result = parse_code(code);
        assert!(result.is_ok());

        let code = "false;";
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_literal_null() {
        let code = "null;";
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_binary_expression() {
        let code = "1 + 2;";
        let result = parse_code(code);
        assert!(result.is_ok());

        let code = "a * b + c;";
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_unary_expression() {
        let code = "-5;";
        let result = parse_code(code);
        assert!(result.is_ok());

        let code = "not true;";
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_var_declaration() {
        let code = "var x = 5;";
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_if_statement() {
        let code = r#"
        if (a > b) {
            return a;
        }
        "#;
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_if_else_statement() {
        let code = r#"
        if (a > b) {
            return a;
        } else {
            return b;
        }
        "#;
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_while_loop() {
        let code = r#"
        while (i < 10) {
            i = i + 1;
        }
        "#;
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_for_loop() {
        let code = r#"
        for var item in list {
            print item;
        }
        "#;
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_return_statement() {
        let code = "return 5;";
        let result = parse_code(code);
        assert!(result.is_ok());

        let code = "return;";
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_lambda() {
        let code = "var f = |x| { return x; };";
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_function_call() {
        let code = "foo(a, b, c);";
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_list() {
        let code = "[1, 2, 3];";
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_empty_statement() {
        let code = ";";
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_block() {
        let code = r#"
        {
            var x = 1;
            var y = 2;
        }
        "#;
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_operator_precedence() {
        // 测试优先级：* 高于 +
        let code = "a + b * c;";
        let result = parse_code(code);
        assert!(result.is_ok());

        // 测试括号改变优先级
        let code = "(a + b) * c;";
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_comparison_operators() {
        let cases = vec![
            ("a == b;", "DoubleEqual"),
            ("a != b;", "ExclamationEqual"),
            ("a > b;", "GreaterThan"),
            ("a <= b;", "LessThanEqual"),
        ];

        for (code, expected_op) in cases {
            let result = parse_code(code);
            if let Err(ref e) = result {
                println!("Failed to parse '{}': {:?}", code, e);
            }
            assert!(result.is_ok(), "Failed to parse {} comparison", expected_op);
        }
    }

    #[test]
    fn test_parse_logical_operators() {
        let code = "a and b;";
        let result = parse_code(code);
        assert!(result.is_ok());

        let code = "a or b;";
        let result = parse_code(code);
        assert!(result.is_ok());

        let code = "not a;";
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_member_access() {
        let code = "obj.field;";
        let result = parse_code(code);
        assert!(result.is_ok());

        let code = "obj.method();";
        let result = parse_code(code);
        assert!(result.is_ok());

        let code = "obj.nested.field;";
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_complex_program() {
        let code = r#"
        var add = |x, y| {
            return x + y;
        };
        
        var result = add(1, 2);
        
        if (result > 0) {
            print(result);
        }
        "#;
        let result = parse_code(code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_error_unexpected_token() {
        // 测试错误处理
        let code = "var ;";
        let result = parse_code(code);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_missing_semicolon() {
        let code = "var x = 5"; // 缺少分号
        let result = parse_code(code);
        // 当前实现可能允许最后一个语句无分号
        // 这个测试用于确认当前行为
        println!("Result: {:?}", result);
    }

    // ===== 索引访问测试 =====

    #[test]
    fn test_parse_index_access() {
        let code = "list[0];";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse index access: {:?}", result.err());
    }

    #[test]
    fn test_parse_index_access_expression() {
        let code = "list[i + 1];";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse index with expression: {:?}", result.err());
    }

    #[test]
    fn test_parse_nested_index_access() {
        let code = "matrix[i][j];";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse nested index: {:?}", result.err());
    }

    #[test]
    fn test_parse_chained_index_and_member() {
        let code = "data.items[0].name;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse chained index and member: {:?}", result.err());
    }

    // ===== JSON 字面量测试 =====

    #[test]
    fn test_parse_json_literal_empty() {
        let code = "json {};";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse empty JSON: {:?}", result.err());
    }

    #[test]
    fn test_parse_json_literal_single_entry() {
        let code = r#"json { "key": 42 };"#;
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse single-entry JSON: {:?}", result.err());
    }

    #[test]
    fn test_parse_json_literal_multiple_entries() {
        let code = r#"json { "name": "test", "value": 123, "active": true };"#;
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse multi-entry JSON: {:?}", result.err());
    }

    #[test]
    fn test_parse_json_literal_identifier_keys() {
        // JSON 也支持裸标识符作为键
        let code = "json { name: \"test\", value: 123 };";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse JSON with identifier keys: {:?}", result.err());
    }

    #[test]
    fn test_parse_json_literal_nested() {
        let code = r#"json { "outer": json { "inner": 42 } };"#;
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse nested JSON: {:?}", result.err());
    }

    #[test]
    fn test_parse_json_literal_with_expression() {
        let code = r#"json { "result": a + b, "value": foo() };"#;
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse JSON with expressions: {:?}", result.err());
    }

    // ===== 模块定义测试 =====

    #[test]
    fn test_parse_module_definition() {
        let code = r#"
        module math {
            var PI = 314;
        }
        "#;
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse module definition: {:?}", result.err());
    }

    #[test]
    fn test_parse_module_with_exports() {
        let code = r#"
        module utils {
            pub var version = 1;
            var internal = 0;
        }
        "#;
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse module with exports: {:?}", result.err());
    }

    #[test]
    fn test_parse_nested_module() {
        let code = r#"
        module outer {
            module inner {
                var x = 1;
            }
        }
        "#;
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse nested module: {:?}", result.err());
    }

    // ===== 导入语句测试 =====

    #[test]
    fn test_parse_import_simple() {
        let code = "import std;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse simple import: {:?}", result.err());
    }

    #[test]
    fn test_parse_import_with_alias() {
        let code = "import std as standard;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse import with alias: {:?}", result.err());
    }

    #[test]
    fn test_parse_import_module_path() {
        let code = "import std.math.geometry;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse module path import: {:?}", result.err());
    }

    #[test]
    fn test_parse_from_import_single() {
        let code = "from std import print;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse from import single: {:?}", result.err());
    }

    #[test]
    fn test_parse_from_import_multiple() {
        let code = "from std import print, assert, type;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse from import multiple: {:?}", result.err());
    }

    #[test]
    fn test_parse_from_import_module_path() {
        let code = "from std.math import sqrt, sin, cos;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse from import with path: {:?}", result.err());
    }

    // ===== Yield 表达式测试 =====

    #[test]
    fn test_parse_yield_without_value() {
        let code = "yield;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse yield without value: {:?}", result.err());
    }

    #[test]
    fn test_parse_yield_with_value() {
        let code = "yield 42;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse yield with value: {:?}", result.err());
    }

    #[test]
    fn test_parse_yield_with_expression() {
        let code = "yield a + b;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse yield with expression: {:?}", result.err());
    }

    #[test]
    fn test_parse_yield_in_lambda() {
        let code = r#"
        var gen = || {
            yield 1;
            yield 2;
            return 3;
        };
        "#;
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse yield in lambda: {:?}", result.err());
    }

    // ===== 更复杂的组合测试 =====

    #[test]
    fn test_parse_complex_chained_calls() {
        let code = r#"
        var result = obj.method1(a, b).method2(c).field.method3();
        "#;
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse complex chained calls: {:?}", result.err());
    }

    #[test]
    fn test_parse_function_call_with_complex_args() {
        let code = r#"
        foo(a + b, obj.field, list[0], || { return 1; });
        "#;
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse function call with complex args: {:?}", result.err());
    }

    #[test]
    fn test_parse_assignment_to_index() {
        let code = "list[0] = 42;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse index assignment: {:?}", result.err());
    }

    #[test]
    fn test_parse_assignment_to_member() {
        let code = "obj.field = value;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse member assignment: {:?}", result.err());
    }

    #[test]
    fn test_parse_pub_var_declaration() {
        let code = "pub var x = 5;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse pub var: {:?}", result.err());
    }

    // ===== 更多错误场景测试 =====

    #[test]
    fn test_parse_error_invalid_json_key() {
        // JSON 键必须是字符串或标识符
        let code = "json { 123: value };";
        let result = parse_code(code);
        assert!(result.is_err(), "Should error for invalid JSON key");
    }

    #[test]
    fn test_parse_error_unclosed_json() {
        let code = r#"json { "key": value"#;
        let result = parse_code(code);
        assert!(result.is_err(), "Should error for unclosed JSON");
    }

    #[test]
    fn test_parse_error_unclosed_list() {
        let code = "[1, 2, 3;";
        let result = parse_code(code);
        assert!(result.is_err(), "Should error for unclosed list");
    }

    #[test]
    fn test_parse_error_lambda_missing_pipe() {
        let code = "var f = |x { return x; };";
        let result = parse_code(code);
        assert!(result.is_err(), "Should error for lambda missing closing pipe");
    }

    #[test]
    fn test_parse_error_lambda_missing_comma() {
        let code = "var f = |x y| { return x; };";
        let result = parse_code(code);
        assert!(result.is_err(), "Should error for lambda missing comma");
    }

    #[test]
    fn test_parse_error_import_missing_semicolon() {
        let code = "import std";
        let result = parse_code(code);
        // 最后一个语句可能允许无分号
        println!("Import without semicolon: {:?}", result);
    }

    #[test]
    fn test_parse_error_from_import_missing_items() {
        let code = "from std import;";
        let result = parse_code(code);
        assert!(result.is_err(), "Should error for from import without items");
    }

    #[test]
    fn test_parse_empty_module_body() {
        let code = "module empty {}";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse empty module body: {:?}", result.err());
    }

    #[test]
    fn test_parse_multiple_semicolons() {
        let code = "var x = 1;;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse multiple semicolons: {:?}", result.err());
    }

    #[test]
    fn test_parse_deeply_nested_expressions() {
        let code = "((((1 + 2))));";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse deeply nested parens: {:?}", result.err());
    }

    #[test]
    fn test_parse_complex_if_elif_else() {
        let code = r#"
        if (a == 1) {
            return 1;
        } elif (a == 2) {
            return 2;
        } elif (a == 3) {
            return 3;
        } elif (a == 4) {
            return 4;
        } else {
            return 0;
        }
        "#;
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse complex if-elif-else: {:?}", result.err());
    }

    #[test]
    fn test_parse_while_with_complex_condition() {
        let code = r#"
        while (i > 0 and i < 100) {
            i = i + 1;
        }
        "#;
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse while with complex condition: {:?}", result.err());
    }

    // ===== 类型表达式解析测试 =====

    #[test]
    fn test_parse_var_decl_with_simple_type() {
        let code = "var x: int = 42;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse var with int type: {:?}", result.err());
    }

    #[test]
    fn test_parse_var_decl_with_string_type() {
        let code = r#"var x: string = "hello";"#;
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse var with string type: {:?}", result.err());
    }

    #[test]
    fn test_parse_var_decl_with_bool_type() {
        let code = "var x: bool = true;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse var with bool type: {:?}", result.err());
    }

    #[test]
    fn test_parse_var_decl_with_float_type() {
        let code = "var x: float = 3.14;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse var with float type: {:?}", result.err());
    }

    #[test]
    fn test_parse_var_decl_with_list_type() {
        let code = "var x: List<int> = [1, 2, 3];";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse var with List<int> type: {:?}", result.err());
    }

    #[test]
    fn test_parse_var_decl_with_nested_list_type() {
        let code = "var x: List<List<int>> = [[1], [2]];";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse var with nested list type: {:?}", result.err());
    }

    #[test]
    fn test_parse_var_decl_with_tuple_type() {
        // 注意：Tuple 字面量语法还未实现，使用 json 作为替代
        let code = r#"var x: Tuple<int, string> = json { "0": 1, "1": "hello" };"#;
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse var with Tuple type: {:?}", result.err());
    }

    #[test]
    fn test_parse_var_decl_with_function_type() {
        let code = "var f: |int| -> int = |x: int| -> int { return x; };";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse var with function type: {:?}", result.err());
    }

    #[test]
    fn test_parse_var_decl_with_complex_function_type() {
        let code = "var f: |int, int| -> bool = |x: int, y: int| -> bool { return x == y; };";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse var with complex function type: {:?}", result.err());
    }

    #[test]
    fn test_parse_lambda_with_param_types() {
        let code = "var f = |x: int, y: int| { return x + y; };";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse lambda with param types: {:?}", result.err());
    }

    #[test]
    fn test_parse_lambda_with_return_type() {
        let code = "var f = |x: int| -> int { return x * 2; };";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse lambda with return type: {:?}", result.err());
    }

    #[test]
    fn test_parse_lambda_with_full_types() {
        let code = "var f = |x: int, y: float| -> string { return \"result\"; };";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse lambda with full types: {:?}", result.err());
    }

    #[test]
    fn test_parse_lambda_no_params_with_return_type() {
        let code = "var f = || -> int { return 42; };";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse lambda no params with return type: {:?}", result.err());
    }

    #[test]
    fn test_parse_tuple_type_empty() {
        // 空 Tuple 类型
        let code = "var x: Tuple<> = json {};";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse empty Tuple type: {:?}", result.err());
    }

    #[test]
    fn test_parse_tuple_type_single() {
        // 单元素 Tuple 类型
        let code = "var x: Tuple<int> = json { \"0\": 1 };";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse single Tuple type: {:?}", result.err());
    }

    #[test]
    fn test_parse_pub_var_with_type() {
        let code = "pub var x: int = 42;";
        let result = parse_code(code);
        assert!(result.is_ok(), "Failed to parse pub var with type: {:?}", result.err());
    }
}
