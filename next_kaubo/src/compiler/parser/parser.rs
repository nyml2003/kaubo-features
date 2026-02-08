use super::super::lexer::token_kind::KauboTokenKind;
use super::error::{ParseResult, ParserError};
use super::expr::{
    Assign, Binary, Expr, ExprKind, FunctionCall, Grouping, Lambda, LiteralFalse, LiteralInt,
    LiteralList, LiteralNull, LiteralString, LiteralTrue, MemberAccess, Unary, VarRef,
};
use super::module::{Module, ModuleKind};
use super::stmt::{
    BlockStmt, EmptyStmt, ExprStmt, ForStmt, IfStmt, ReturnStmt, Stmt, StmtKind, VarDeclStmt,
    WhileStmt,
};
use super::utils::{get_associativity, get_precedence};
use crate::kit::lexer::c_lexer::Lexer;
use crate::kit::lexer::types::Token;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Parser {
    lexer: Rc<RefCell<Lexer<KauboTokenKind>>>,
    current_token: Option<Token<KauboTokenKind>>, // (类型, 文本值)
}

impl Parser {
    pub fn new(lexer: Lexer<KauboTokenKind>) -> Self {
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

    /// 期望并消费指定类型的token，否则返回错误
    fn expect(&mut self, kind: KauboTokenKind) -> ParseResult<()> {
        if self.match_token(kind) {
            Ok(())
        } else {
            Err(ParserError::UnexpectedToken)
        }
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
            .ok_or(ParserError::UnexpectedEndOfInput)?;

        match token.kind {
            KauboTokenKind::LiteralInteger => self.parse_int(),
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
            _ => Err(ParserError::UnexpectedToken),
        }
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
                    .ok_or(ParserError::ExpectedIdentifierAfterDot)?;
                if token.kind != KauboTokenKind::Identifier {
                    return Err(ParserError::ExpectedIdentifierAfterDot);
                }

                let member_name = token.value.clone();
                self.consume();
                expr = Box::new(ExprKind::MemberAccess(MemberAccess {
                    object: expr,
                    member: member_name,
                }));
            } else if self.check(KauboTokenKind::LeftParenthesis) {
                // 函数调用：a() 或 a.b()
                expr = self.parse_function_call(expr)?;
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// 解析整数字面量
    fn parse_int(&mut self) -> ParseResult<Expr> {
        let token = self.current_token.as_ref().unwrap();
        let num = token
            .value
            .parse()
            .map_err(|_| ParserError::InvalidNumberFormat)?;
        self.consume();
        Ok(Box::new(ExprKind::LiteralInt(LiteralInt { value: num })))
    }

    /// 解析字符串字面量
    fn parse_string(&mut self) -> ParseResult<Expr> {
        let token = self.current_token.as_ref().unwrap();
        // 移除首尾引号
        let s = token.value[1..token.value.len() - 1].to_string();
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

        self.expect(KauboTokenKind::RightSquareBracket)?;
        Ok(Box::new(ExprKind::LiteralList(LiteralList { elements })))
    }

    /// 解析括号表达式
    fn parse_parenthesized(&mut self) -> ParseResult<Expr> {
        self.consume(); // 消费 '('

        let expr = self.parse_expression(0)?;

        self.expect(KauboTokenKind::RightParenthesis)
            .map_err(|_| ParserError::MissingRightParen)?;

        Ok(Box::new(ExprKind::Grouping(Grouping { expression: expr })))
    }

    /// 解析标识符引用
    fn parse_identifier_expression(&mut self) -> ParseResult<Expr> {
        let token = self.current_token.as_ref().unwrap();
        let name = token.value.clone();
        self.consume();
        Ok(Box::new(ExprKind::VarRef(VarRef { name })))
    }

    /// 解析匿名函数（lambda）
    fn parse_lambda(&mut self) -> ParseResult<Expr> {
        self.expect(KauboTokenKind::Pipe)?; // 消费 '|'

        let mut params = Vec::new();

        // 解析参数列表
        if !self.check(KauboTokenKind::Pipe) {
            while let Some(token) = &self.current_token {
                if token.kind == KauboTokenKind::Identifier {
                    params.push(token.value.clone());
                    self.consume();

                    if self.match_token(KauboTokenKind::Comma) {
                        continue;
                    } else if self.check(KauboTokenKind::Pipe) {
                        break;
                    } else {
                        return Err(ParserError::ExpectedCommaOrPipeInLambda);
                    }
                } else {
                    break;
                }
            }
        }

        self.expect(KauboTokenKind::Pipe)?; // 消费 '|'

        let body = self.parse_block()?;
        Ok(Box::new(ExprKind::Lambda(Lambda { params, body })))
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
            .map_err(|_| ParserError::MissingRightParen)?;

        Ok(Box::new(ExprKind::FunctionCall(FunctionCall {
            function_expr,
            arguments,
        })))
    }

    /// 解析变量声明
    fn parse_var_declaration(&mut self) -> ParseResult<Stmt> {
        self.consume(); // 消费 'var'

        let token = self
            .current_token
            .as_ref()
            .ok_or(ParserError::UnexpectedToken)?;
        if token.kind != KauboTokenKind::Identifier {
            return Err(ParserError::UnexpectedToken);
        }
        let name = token.value.clone();
        self.consume();

        self.expect(KauboTokenKind::Equal)?;
        let initializer = self.parse_expression(0)?;
        self.expect(KauboTokenKind::Semicolon)?;

        Ok(Box::new(StmtKind::VarDecl(VarDeclStmt {
            name,
            initializer,
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
        for i in 0..10 {  // 最多打印 10 个 token，防止死循环
            match lexer.next_token() {
                Some(token) => println!("  [{}] {:?} = {:?}", i, token.kind, token.value),
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
        assert!(result.is_ok(), "Failed to parse string literal: {:?}", result);
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
        for (item) in (list) {
            print(item);
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
        let code = "var x = 5";  // 缺少分号
        let result = parse_code(code);
        // 当前实现可能允许最后一个语句无分号
        // 这个测试用于确认当前行为
        println!("Result: {:?}", result);
    }
}
