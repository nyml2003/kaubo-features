#include <utility>

#include "Parser/Expr.h"
#include "Parser/Parser.h"
#include "Parser/Stmt.h"
#include "Parser/Utils.h"

namespace Parser {

auto Parser::parse() -> Result<ModulePtr, Error> {
  return parse_module();
}

auto Parser::parse_module() -> Result<ModulePtr, Error> {
  auto module = Utils::create<Module>();

  // 解析所有语句直到文件结束
  while (current_token.has_value()) {
    // 跳过分号（空语句）
    if (match(TokenType::Semicolon)) {
      continue;
    }

    auto stmt_result = parse_statement();
    if (stmt_result.is_err()) {
      return Err(stmt_result.unwrap_err());
    }

    module->statements.push_back(stmt_result.unwrap());

    // 消费分号（如果存在）
    match(TokenType::Semicolon);
  }

  return Ok(module);
}

auto Parser::parse_statement()  // NOLINT(misc-no-recursion)
  -> Result<StmtPtr, Error> {
  enter_statement();
  // 检查是否是block
  if (check(TokenType::LeftCurlyBrace)) {
    auto block_result = parse_block();
    if (block_result.is_err()) {
      return Err(block_result.unwrap_err());
    }
    auto block = block_result.unwrap();
    exit_statement(block);
    return Ok(block);
  }

  // 检查是否是变量声明
  if (check(TokenType::Var)) {
    auto expr_result = parse_var_declaration();
    if (expr_result.is_err()) {
      return Err(expr_result.unwrap_err());
    }
    auto var_decl = expr_result.unwrap();
    exit_statement(var_decl);
    return Ok(var_decl);
  }

  // 检查是否是空语句（只有分号）
  if (check(TokenType::Semicolon)) {
    consume();  // 消费分号
    return Ok(Utils::create<Stmt::Stmt>(Utils::create<Stmt::Empty>()));
  }

  // 否则是表达式语句
  auto expr_result = parse_expression();
  if (expr_result.is_err()) {
    return Err(expr_result.unwrap_err());
  }
  auto expr_stmt =
    Utils::create<Stmt::Stmt>(Utils::create<Stmt::Expr>(expr_result.unwrap()));
  return Ok(expr_stmt);
}

auto Parser::parse_block()  // NOLINT(misc-no-recursion)
  -> Result<StmtPtr, Error> {
  // 期望左大括号
  auto err = expect(TokenType::LeftCurlyBrace);
  if (err.is_err()) {
    return Err(Error::UnexpectedToken);
  }

  std::vector<StmtPtr> statements;

  // 解析block内的所有语句直到遇到右大括号
  while (current_token.has_value() && !check(TokenType::RightCurlyBrace)) {
    // 跳过分号（空语句）
    if (match(TokenType::Semicolon)) {
      continue;
    }

    auto stmt_result = parse_statement();
    if (stmt_result.is_err()) {
      return Err(stmt_result.unwrap_err());
    }

    statements.push_back(stmt_result.unwrap());

    // 消费分号（如果存在）
    match(TokenType::Semicolon);
  }

  // 期望右大括号
  auto right_brace_result = expect(TokenType::RightCurlyBrace);
  if (right_brace_result.is_err()) {
    return Err(Error::UnexpectedToken);
  }

  return Ok(
    Utils::create<Stmt::Stmt>(
      Utils::create(Stmt::Block{.statements = statements})
    )
  );
}

auto Parser::parse_expression(int32_t precedence)  // NOLINT(misc-no-recursion)
  -> Result<ExprPtr, Error> {
  // 解析左操作数（一元表达式或基本表达式）
  auto left_result = parse_unary();
  if (left_result.is_err()) {
    return Err(left_result.unwrap_err());
  }
  auto left = left_result.unwrap();

  // 解析二元运算符和右操作数
  while (true) {
    if (!current_token.has_value()) {
      break;
    }

    TokenType op = current_token->type;
    auto op_precedence = Utils::get_precedence(op);

    // 如果当前运算符优先级低于要求的最小优先级，停止解析
    if (op_precedence <= precedence) {
      break;
    }

    // 消费运算符
    consume();

    // 解析右操作数，考虑结合性
    auto next_precedence =
      Utils::get_associativity(op) ? op_precedence : op_precedence - 1;
    auto right_result = parse_expression(next_precedence);
    if (right_result.is_err()) {
      return Err(right_result.unwrap_err());
    }
    const auto& right = right_result.unwrap();
    enter_expr();
    left = Utils::create<Expr::Expr>(Utils::create(
      Expr::Binary{
        .left = left,
        .op = op,
        .right = right,
      }
    ));
    exit_expr(left);
  }
  return Ok(left);
}

auto Parser::parse_unary()     // NOLINT(misc-no-recursion)
  -> Result<ExprPtr, Error> {  // 检查一元运算符
  if (check(TokenType::Plus) || check(TokenType::Minus)) {
    TokenType op = current_token->type;
    consume();

    auto operand_result = parse_unary();  // 右结合
    if (operand_result.is_err()) {
      return Err(operand_result.unwrap_err());
    }
    const auto& operand = operand_result.unwrap();
    enter_expr();
    auto expr = Utils::create<Expr::Expr>(Utils::create(
      Expr::Unary{
        .op = op,
        .operand = operand,
      }
    ));
    exit_expr(expr);
    return Ok(expr);
  }

  return parse_primary();
}

auto Parser::parse_int() -> Result<ExprPtr, Error> {
  try {
    int64_t value = std::stoll(current_token->value);
    consume();
    enter_expr();
    auto expr =
      Utils::create<Expr::Expr>(Utils::create(Expr::LiteralInt{.value = value})
      );
    exit_expr(expr);
    return Ok(expr);
  } catch (const std::exception&) {
    return Err(Error::InvalidNumberFormat);
  }
}

auto Parser::parse_identifier_expression  // NOLINT(misc-no-recursion)
  () -> Result<ExprPtr, Error> {
  std::string identifier_name = current_token->value;
  consume();
  enter_expr();
  auto expr = Utils::create<Expr::Expr>(
    Utils::create(Expr::VarRef{.name = identifier_name})
  );
  exit_expr(expr);
  return Ok(expr);
}

auto Parser::parse_string() -> Result<ExprPtr, Error> {
  enter_expr();
  auto expr = Utils::create<Expr::Expr>(Utils::create(
    Expr::LiteralString{
      .value = current_token->value.substr(1, current_token->value.size() - 2)
    }
  ));
  consume();
  exit_expr(expr);
  return Ok(expr);
}

// 新增：解析匿名函数 |参数列表|{函数体}
auto Parser::parse_lambda()  // NOLINT(misc-no-recursion)
  -> Result<ExprPtr, Error> {
  // 消费左竖线 |
  if (!match(TokenType::Pipe)) {
    return Err(Error::ExpectedPipe);
  }

  std::vector<std::string> parameters;

  // 解析参数列表（|a, b| 形式）
  if (!check(TokenType::Pipe)) {  // 如果不是空参数列表
    while (true) {
      if (!check(TokenType::Identifier)) {
        return Err(Error::ExpectedIdentifierInLambdaParams);
      }

      // 收集参数名
      parameters.push_back(current_token->value);
      consume();

      if (match(TokenType::Comma)) {
        continue;  // 处理多个参数
      }
      if (check(TokenType::Pipe)) {
        break;  // 参数列表结束
      }
      return Err(Error::ExpectedCommaOrPipeInLambda);
    }
  }

  // 消费右竖线 |
  if (!match(TokenType::Pipe)) {
    return Err(Error::ExpectedPipe);
  }

  // 解析函数体（必须是代码块）
  if (!check(TokenType::LeftCurlyBrace)) {
    return Err(Error::ExpectedLeftBraceInLambdaBody);
  }
  auto body_result = parse_block();
  if (body_result.is_err()) {
    return Err(body_result.unwrap_err());
  }

  // 创建lambda表达式节点
  enter_expr();
  auto lambda_expr = Utils::create<Expr::Expr>(Utils::create(
    Expr::Lambda{
      .params = parameters,
      .body = body_result.unwrap(),
    }
  ));
  exit_expr(lambda_expr);
  return Ok(lambda_expr);
}

auto Parser::parse_primary_base  // NOLINT(misc-no-recursion)
  () -> Result<ExprPtr, Error> {
  if (!current_token.has_value()) {
    return Err(Error::UnexpectedEndOfInput);
  }

  switch (current_token->type) {
    case TokenType::Literal_Integer:
      return parse_int();
    case TokenType::Literal_String:
      return parse_string();
    case TokenType::LeftParenthesis:
      return parse_parenthesized();
    case TokenType::Identifier:
      return parse_identifier_expression();
    case TokenType::Pipe:  // 新增：遇到|时解析匿名函数
      return parse_lambda();
    default:
      return Err(Error::UnexpectedToken);
  }
}

auto Parser::parse_primary  // NOLINT(misc-no-recursion)
  () -> Result<ExprPtr, Error> {
  // 先解析基础表达式
  auto base_expr = parse_primary_base();
  if (base_expr.is_err()) {
    return Err(base_expr.unwrap_err());
  }

  // 再处理后缀运算符（. 成员访问、() 函数调用等）
  return parse_postfix(base_expr.unwrap());
}

auto Parser::parse_parenthesized  // NOLINT(misc-no-recursion)
  () -> Result<ExprPtr, Error> {
  consume();  // 消费 '('

  // 解析括号内的表达式（仅作为分组，不再处理函数定义）
  auto expr_result = parse_expression();
  if (expr_result.is_err()) {
    return Err(expr_result.unwrap_err());
  }

  // 期望右括号
  if (!match(TokenType::RightParenthesis)) {
    return Err(Error::MissingRightParen);
  }

  enter_expr();
  auto group_expr = Utils::create<Expr::Expr>(
    Utils::create(Expr::Grouping{.expression = expr_result.unwrap()})
  );
  exit_expr(group_expr);
  return Ok(group_expr);
}

auto Parser::parse_function_call  // NOLINT(misc-no-recursion)
  (ExprPtr function_expr) -> Result<ExprPtr, Error> {
  // 消费左括号
  consume();

  std::vector<ExprPtr> arguments;

  // 解析参数列表（如果有）
  if (!check(TokenType::RightParenthesis)) {
    while (true) {
      // 解析参数表达式
      auto arg_result = parse_expression();
      if (arg_result.is_err()) {
        return Err(arg_result.unwrap_err());
      }
      arguments.push_back(arg_result.unwrap());

      // 检查是否有逗号继续解析更多参数
      if (match(TokenType::Comma)) {
        continue;
      }
      break;
    }
  }

  // 期望右括号
  auto err = expect(TokenType::RightParenthesis);
  if (err.is_err()) {
    return Err(Error::MissingRightParen);
  }
  enter_expr();
  auto expr = Utils::create<Expr::Expr>(Utils::create(
    Expr::FunctionCall{
      .function_expr = std::move(function_expr),
      .arguments = arguments,
    }
  ));
  exit_expr(expr);
  return Ok(expr);
}

auto Parser::parse_postfix(ExprPtr expr)  // NOLINT(misc-no-recursion)
  -> Result<ExprPtr, Error> {
  while (true) {
    if (check(Lexer::TokenType::Dot)) {
      // 处理成员访问（a.b）
      consume();  // 消费 '.'

      if (!check(Lexer::TokenType::Identifier)) {
        return Err(Error::ExpectedIdentifierAfterDot);
      }

      std::string member_name = current_token->value;
      consume();  // 消费标识符

      enter_expr();
      expr = Utils::create<Expr::Expr>(
        Utils::create(Expr::MemberAccess{.object = expr, .member = member_name})
      );
      exit_expr(expr);

    } else if (check(Lexer::TokenType::LeftParenthesis)) {
      // 处理函数调用（a.b() 或 f()）
      expr = parse_function_call(expr).unwrap();

    } else {
      // 无后缀运算符，退出循环
      break;
    }
  }
  return Ok(expr);
}

auto Parser::parse_var_declaration()  // NOLINT(misc-no-recursion)
  -> Result<StmtPtr, Error> {
  // 消费 'var' 关键字
  consume();

  // 期望标识符
  if (!check(TokenType::Identifier)) {
    return Err(Error::UnexpectedToken);
  }
  std::string var_name = current_token->value;
  consume();

  // 期望等号
  auto equals_result = expect(TokenType::Equal);
  if (equals_result.is_err()) {
    return Err(Error::UnexpectedToken);
  }

  // 解析表达式（支持新的lambda语法）
  auto expr_result = parse_expression();
  if (expr_result.is_err()) {
    return Err(expr_result.unwrap_err());
  }

  // 消费分号
  auto semicolon_result = expect(TokenType::Semicolon);
  if (semicolon_result.is_err()) {
    return Err(semicolon_result.unwrap_err());
  }

  return Ok(
    Utils::create<Stmt::Stmt>(Utils::create(
      Stmt::VarDecl{.name = var_name, .initializer = expr_result.unwrap()}
    ))
  );
}

}  // namespace Parser