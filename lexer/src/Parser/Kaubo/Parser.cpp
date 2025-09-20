#include "Parser.h"
#include <iostream>
#include <string>
#include "Utils/Overloaded.h"

namespace Parser::Kaubo {

using Utils::Err;
using Utils::Ok;

auto Parser::parse() -> Result<Expr, ParseError> {
  // 解析第一个语句
  auto first_stmt = parse_statement();
  if (first_stmt.is_err()) {
    return Err(first_stmt.unwrap_err());
  }
  
  // 如果有分号，继续解析后续语句
  // 注意：这里暂时只返回第一个语句，后续需要修改为支持多个语句
  // TODO: 修改为支持多个语句的AST结构
  return Ok(std::move(first_stmt).unwrap());
}

auto Parser::parse_statement() -> Result<Expr, ParseError> {
  // 检查是否是变量声明
  if (check(TokenType::Var)) {
    return parse_var_declaration();
  }
  return parse_expression();
}

void Parser::consume() {
  current_token = m_lexer->next_token();
}

auto Parser::check(TokenType type) const -> bool {
  return current_token.has_value() && current_token->type == type;
}

auto Parser::match(TokenType type) -> bool {
  if (check(type)) {
    consume();
    return true;
  }
  return false;
}

auto Parser::expect(TokenType type) -> Result<void, ParseError> {
  if (check(type)) {
    consume();
    return Ok();
  }
  return Err(ParseError::UnexpectedToken);
}

auto Parser::get_precedence(TokenType op) const -> int {
  switch (op) {
    case TokenType::Plus:
    case TokenType::Minus:
      return 10;
    case TokenType::Multiply:
    case TokenType::Divide:
      return 20;
    default:
      return 0;
  }
}

auto Parser::get_associativity(TokenType op) const -> bool {
  // 所有运算符都是左结合的
  return true;
}

auto Parser::parse_expression(int precedence) -> Result<Expr, ParseError> {
  // 解析左操作数（一元表达式或基本表达式）
  auto left_result = parse_unary();
  if (left_result.is_err()) {
    return Err(left_result.unwrap_err());
  }
  auto left = std::move(left_result).unwrap();

  // 解析二元运算符和右操作数
  while (true) {
    if (!current_token.has_value()) {
      break;
    }

    TokenType op = current_token->type;
    int op_precedence = get_precedence(op);

    // 如果当前运算符优先级低于要求的最小优先级，停止解析
    if (op_precedence <= precedence) {
      break;
    }

    // 消费运算符
    consume();

    // 解析右操作数，考虑结合性
    int next_precedence =
      get_associativity(op) ? op_precedence : op_precedence - 1;
    auto right_result = parse_expression(next_precedence);
    if (right_result.is_err()) {
      return Err(right_result.unwrap_err());
    }
    auto right = std::move(right_result).unwrap();

    // 创建二元表达式
    auto binary_expr = std::make_unique<BinaryExpr>();
    binary_expr->left = std::make_unique<Expr>(std::move(left));
    binary_expr->op = op;
    binary_expr->right = std::make_unique<Expr>(std::move(right));

    left = Expr(std::move(binary_expr));
  }

  return Ok(std::move(left));
}

auto Parser::parse_unary() -> Result<Expr, ParseError> {
  // 检查一元运算符
  if (check(TokenType::Plus) || check(TokenType::Minus)) {
    TokenType op = current_token->type;
    consume();

    auto operand_result = parse_unary();  // 右结合
    if (operand_result.is_err()) {
      return Err(operand_result.unwrap_err());
    }
    auto operand = std::move(operand_result).unwrap();

    auto unary_expr = std::make_unique<UnaryExpr>();
    unary_expr->op = op;
    unary_expr->operand = std::make_unique<Expr>(std::move(operand));

    return Ok(Expr(std::move(unary_expr)));
  }

  return parse_primary();
}

auto Parser::parse_primary() -> Result<Expr, ParseError> {
  if (!current_token.has_value()) {
    return Err(ParseError::UnexpectedEndOfInput);
  }

  switch (current_token->type) {
    case TokenType::Integer: {
      try {
        int64_t value = std::stoll(current_token->value);
        consume();
        return Ok(Expr(value));
      } catch (const std::exception&) {
        return Err(ParseError::InvalidNumberFormat);
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
        return Err(ParseError::MissingRightParen);
      }

      auto grouping_expr = std::make_unique<GroupingExpr>();
      grouping_expr->expression =
        std::make_unique<Expr>(std::move(expr_result).unwrap());
      return Ok(Expr(std::move(grouping_expr)));
    }

    case TokenType::Identifier: {
      std::string identifier_name = current_token->value;
      consume();

      // 检查是否是函数调用
      if (check(TokenType::LeftParen)) {
        return parse_function_call(identifier_name);
      }

      // 否则是变量引用
      auto var_ref = std::make_unique<VarRefExpr>();
      var_ref->name = std::move(identifier_name);
      return Ok(Expr(std::move(var_ref)));
    }

    default:
      return Err(ParseError::UnexpectedToken);
  }
}

auto Parser::parse_function_call(const std::string& function_name) -> Result<Expr, ParseError> {
  // 消费左括号
  consume();

  std::vector<std::unique_ptr<Expr>> arguments;

  // 解析参数列表（如果有）
  if (!check(TokenType::RightParen)) {
    while (true) {
      // 解析参数表达式
      auto arg_result = parse_expression();
      if (arg_result.is_err()) {
        return Err(arg_result.unwrap_err());
      }
      arguments.push_back(std::make_unique<Expr>(std::move(arg_result).unwrap()));

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
    return Err(ParseError::MissingRightParen);
  }

  // 创建函数调用表达式
  auto func_call = std::make_unique<FunctionCallExpr>();
  func_call->function_name = function_name;
  func_call->arguments = std::move(arguments);

  return Ok(Expr(std::move(func_call)));
}

auto Parser::parse_var_declaration() -> Result<Expr, ParseError> {
  // 消费 'var' 关键字
  consume();

  // 期望标识符
  if (!check(TokenType::Identifier)) {
    return Err(ParseError::UnexpectedToken);
  }
  std::string var_name = current_token->value;
  consume();

  // 期望冒号
  auto colon_result = expect(TokenType::Colon);
  if (colon_result.is_err()) {
    return Err(ParseError::UnexpectedToken);
  }

  // 期望类型关键字 'int'
  if (!check(TokenType::IntType)) {
    return Err(ParseError::UnexpectedToken);
  }
  consume();

  // 期望等号
  auto equals_result = expect(TokenType::Equals);
  if (equals_result.is_err()) {
    return Err(ParseError::UnexpectedToken);
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

  // 创建变量声明表达式
  auto var_decl = std::make_unique<VarDeclExpr>();
  var_decl->name = std::move(var_name);
  var_decl->initializer =
    std::make_unique<Expr>(std::move(expr_result).unwrap());

  return Ok(Expr(std::move(var_decl)));
}

auto Parser::print_ast(const Expr& expr, int indent) -> void {
  // 缩进字符串
  std::string indent_str(indent * 2, ' ');

  // 使用访问者模式处理不同类型的表达式
  std::visit(
    overloaded{
      [&](IntValue n) {
        std::cout << indent_str << "IntValue: " << n << std::endl;
      },
      [&](const std::unique_ptr<BinaryExpr>& binary_expr) {
        std::cout << indent_str << "BinaryExpr: " << to_string(binary_expr->op)
                  << std::endl;
        std::cout << indent_str << "  left:" << std::endl;
        print_ast(*binary_expr->left, indent + 2);
        std::cout << indent_str << "  right:" << std::endl;
        print_ast(*binary_expr->right, indent + 2);
      },
      [&](const std::unique_ptr<UnaryExpr>& unary_expr) {
        std::cout << indent_str << "UnaryExpr: " << to_string(unary_expr->op)
                  << std::endl;
        std::cout << indent_str << "  operand:" << std::endl;
        print_ast(*unary_expr->operand, indent + 2);
      },
      [&](const std::unique_ptr<GroupingExpr>& grouping_expr) {
        std::cout << indent_str << "GroupingExpr: ()" << std::endl;
        std::cout << indent_str << "  expression:" << std::endl;
        print_ast(*grouping_expr->expression, indent + 2);
      },
      [&](const std::unique_ptr<VarDeclExpr>& var_decl_expr) {
        std::cout << indent_str << "VarDeclExpr: " << var_decl_expr->name
                  << std::endl;
        if (var_decl_expr->initializer) {
          std::cout << indent_str << "  initializer:" << std::endl;
          print_ast(*var_decl_expr->initializer, indent + 2);
        }
      },
      [&](const std::unique_ptr<VarRefExpr>& var_ref_expr) {
        std::cout << indent_str << "VarRefExpr: " << var_ref_expr->name
                  << std::endl;
      },
      [&](const std::unique_ptr<FunctionCallExpr>& func_call_expr) {
        std::cout << indent_str << "FunctionCallExpr: " << func_call_expr->function_name
                  << std::endl;
        std::cout << indent_str << "  arguments:" << std::endl;
        for (const auto& arg : func_call_expr->arguments) {
          print_ast(*arg, indent + 2);
        }
      }
    },
    expr.get()
  );
}

}  // namespace Parser::Kaubo