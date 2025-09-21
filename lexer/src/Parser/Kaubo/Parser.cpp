#include "Parser.h"
#include "Expr.h"
#include "Stmt.h"
#include "Utils.h"
#include "Utils/Overloaded.h"

#include <iostream>
#include <vector>

namespace Parser::Kaubo {

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
  // 检查是否是block
  if (check(TokenType::LeftBrace)) {
    auto block_result = parse_block();
    if (block_result.is_err()) {
      return Err(block_result.unwrap_err());
    }
    return Ok(block_result.unwrap());
  }

  // 检查是否是变量声明
  if (check(TokenType::Var)) {
    auto expr_result = parse_var_declaration();
    if (expr_result.is_err()) {
      return Err(expr_result.unwrap_err());
    }
    auto expr = expr_result.unwrap();
    return Ok(Utils::create<Stmt::Stmt>(Utils::create<Stmt::Expr>(expr)));
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
  return Ok(
    Utils::create<Stmt::Stmt>(Utils::create<Stmt::Expr>(expr_result.unwrap()))
  );
}

auto Parser::parse_block()  // NOLINT(misc-no-recursion)
  -> Result<StmtPtr, Error> {
  // 期望左大括号
  auto err = expect(TokenType::LeftBrace);
  if (err.is_err()) {
    return Err(Error::UnexpectedToken);
  }

  std::vector<StmtPtr> statements;

  // 解析block内的所有语句直到遇到右大括号
  while (current_token.has_value() && !check(TokenType::RightBrace)) {
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
  auto right_brace_result = expect(TokenType::RightBrace);
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

    left = Utils::create<Expr::Expr>(Utils::create(
      Expr::Binary{
        .left = left,
        .op = op,
        .right = right,
      }
    ));
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

    return Ok(
      Utils::create<Expr::Expr>(Utils::create(
        Expr::Unary{
          .op = op,
          .operand = operand,
        }
      ))
    );
  }

  return parse_primary();
}

auto Parser::parse_primary()  // NOLINT(misc-no-recursion)
  -> Result<ExprPtr, Error> {
  if (!current_token.has_value()) {
    return Err(Error::UnexpectedEndOfInput);
  }

  switch (current_token->type) {
    case TokenType::Integer: {
      try {
        int64_t value = std::stoll(current_token->value);
        consume();
        return Ok(Utils::create<Expr::Expr>(value));
      } catch (const std::exception&) {
        return Err(Error::InvalidNumberFormat);
      }
    }

    case TokenType::LeftParen: {
      consume();  // 消费左括号

      auto expr_result = parse_expression();
      if (expr_result.is_err()) {
        return Err(expr_result.unwrap_err());
      }

      auto err = expect(TokenType::RightParen);
      if (err.is_err()) {
        return Err(Error::MissingRightParen);
      }
      return Ok(
        Utils::create<Expr::Expr>(Utils::create(
          Expr::Grouping{
            .expression = expr_result.unwrap(),
          }
        ))
      );
    }

    case TokenType::Identifier: {
      std::string identifier_name = current_token->value;
      consume();

      // 检查是否是函数调用
      if (check(TokenType::LeftParen)) {
        return parse_function_call(identifier_name);
      }
      return Ok(
        Utils::create<Expr::Expr>(
          Utils::create(Expr::VarRef{.name = identifier_name})
        )
      );
    }

    default:
      return Err(Error::UnexpectedToken);
  }
}

auto Parser::parse_function_call  // NOLINT(misc-no-recursion)
  (const std::string& function_name) -> Result<ExprPtr, Error> {
  // 消费左括号
  consume();

  std::vector<ExprPtr> arguments;

  // 解析参数列表（如果有）
  if (!check(TokenType::RightParen)) {
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
  auto err = expect(TokenType::RightParen);
  if (err.is_err()) {
    return Err(Error::MissingRightParen);
  }

  return Ok(
    Utils::create<Expr::Expr>(Utils::create(
      Expr::FunctionCall{
        .function_name = function_name,
        .arguments = arguments,
      }
    ))
  );
}

auto Parser::parse_var_declaration() -> Result<ExprPtr, Error> {
  // 消费 'var' 关键字
  consume();

  // 期望标识符
  if (!check(TokenType::Identifier)) {
    return Err(Error::UnexpectedToken);
  }
  std::string var_name = current_token->value;
  consume();

  // 期望等号
  auto equals_result = expect(TokenType::Equals);
  if (equals_result.is_err()) {
    return Err(Error::UnexpectedToken);
  }

  // 解析表达式
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
    Utils::create<Expr::Expr>(Utils::create(
      Expr::VarDecl{.name = var_name, .initializer = expr_result.unwrap()}
    ))
  );
}

auto Parser::print_ast(const ExprPtr& expr, size_t indent) -> void {
  // 缩进字符串
  std::string indent_str(indent * 2, ' ');

  // 使用访问者模式处理不同类型的表达式
  std::visit(
    overloaded{
      [&](Expr::IntValue n) {
        std::cout << indent_str << "IntValue: " << n << '\n';
      },
      [&](const std::shared_ptr<Expr::Binary>& binary_expr) {
        std::cout << indent_str << "BinaryExpr: " << to_string(binary_expr->op)
                  << '\n';
        std::cout << indent_str << "  left:" << '\n';
        print_ast(binary_expr->left, indent + 2);
        std::cout << indent_str << "  right:" << '\n';
        print_ast(binary_expr->right, indent + 2);
      },
      [&](const std::shared_ptr<Expr::Unary>& unary_expr) {
        std::cout << indent_str << "UnaryExpr: " << to_string(unary_expr->op)
                  << '\n';
        std::cout << indent_str << "  operand:" << '\n';
        print_ast(unary_expr->operand, indent + 2);
      },
      [&](const std::shared_ptr<Expr::Grouping>& grouping_expr) {
        std::cout << indent_str << "GroupingExpr: ()" << '\n';
        std::cout << indent_str << "  expression:" << '\n';
        print_ast(grouping_expr->expression, indent + 2);
      },
      [&](const std::shared_ptr<Expr::VarDecl>& var_decl_expr) {
        std::cout << indent_str << "VarDeclExpr: " << var_decl_expr->name
                  << '\n';
        if (var_decl_expr->initializer) {
          std::cout << indent_str << "  initializer:" << '\n';
          print_ast(var_decl_expr->initializer, indent + 2);
        }
      },
      [&](const std::shared_ptr<Expr::VarRef>& var_ref_expr) {
        std::cout << indent_str << "VarRefExpr: " << var_ref_expr->name << '\n';
      },
      [&](const std::shared_ptr<Expr::FunctionCall>& func_call_expr) {
        std::cout << indent_str
                  << "FunctionCallExpr: " << func_call_expr->function_name
                  << '\n';
        std::cout << indent_str << "  arguments:" << '\n';
        for (const auto& arg : func_call_expr->arguments) {
          print_ast(arg, indent + 2);
        }
      },
      [&](const std::shared_ptr<Expr::Assign>& assign_expr) {
        std::cout << indent_str << "AssignExpr: " << assign_expr->name << '\n';
      }

    },
    expr->get_value()
  );
}

auto print_ast(const StmtPtr& stmt, size_t indent) -> void {
  // 缩进字符串
  std::string indent_str(indent * 2, ' ');

  // 使用访问者模式处理不同类型的语句
  std::visit(
    overloaded{
      [&](const std::shared_ptr<Stmt::Expr>& expr_stmt) {
        std::cout << indent_str << "ExprStmt:" << '\n';
        if (expr_stmt->expression) {
          Parser::print_ast(expr_stmt->expression, indent + 1);
        }
      },
      [&](const std::shared_ptr<Stmt::Empty>& /*empty_stmt*/) {
        std::cout << indent_str << "EmptyStmt: ;" << '\n';
      },
      [&](const std::shared_ptr<Stmt::Block>& block_stmt) {
        std::cout << indent_str << "BlockStmt: {" << '\n';
        for (const auto& stmt : block_stmt->statements) {
          print_ast(stmt, indent + 1);
        }
        std::cout << indent_str << "}" << '\n';
      },
    },
    stmt->get_value()
  );
}

auto print_ast(const ModulePtr& module, size_t indent) -> void {
  // 缩进字符串
  std::string indent_str(indent * 2, ' ');

  std::cout << indent_str << "Module:" << '\n';
  for (const auto& stmt : module->statements) {
    print_ast(stmt, indent + 1);
  }
}

}  // namespace Parser::Kaubo