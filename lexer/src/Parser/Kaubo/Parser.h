#pragma once

#include "Error.h"
#include "Lexer/Lexer.h"
#include "Stmt.h"
#include "Utils/Result.h"

#include <cstdint>

namespace Parser::Kaubo {
using Lexer::Kaubo::TokenType;
using Utils::Err;
using Utils::Ok;
using Utils::Result;

// Module（包含多个语句或block）
struct Module {
  std::vector<StmtPtr> statements;
};

using ModulePtr = std::shared_ptr<Module>;

// Pratt parser实现
class Parser {
 public:
  explicit Parser(Lexer::Instance<TokenType> lexer)
    : m_lexer(std::move(lexer)) {
    consume();  // 预读第一个token
  }

  auto parse() -> Result<ModulePtr, Error>;

  // AST打印函数
  static auto print_ast(const ExprPtr& expr, size_t indent = 0) -> void;

 private:
  Lexer::Instance<TokenType> m_lexer;
  std::optional<Lexer::Token<TokenType>> current_token;

  // 消费当前token并读取下一个
  void consume() { current_token = m_lexer->next_token(); }

  // 检查当前token是否为指定类型
  [[nodiscard]] auto check(TokenType type) const -> bool {
    return current_token.has_value() && current_token->type == type;
  }

  // 检查并消费指定类型的token
  auto match(TokenType type) -> bool {
    if (check(type)) {
      consume();
      return true;
    }
    return false;
  }

  // 期望并消费指定类型的token，否则返回错误
  auto expect(TokenType type) -> Result<void, Error> {
    if (check(type)) {
      consume();
      return Ok();
    }
    return Err(Error::UnexpectedToken);
  }

  auto parse_expression(int32_t precedence = 0) -> Result<ExprPtr, Error>;
  auto parse_primary() -> Result<ExprPtr, Error>;
  auto parse_unary() -> Result<ExprPtr, Error>;
  auto parse_statement() -> Result<StmtPtr, Error>;
  auto parse_block() -> Result<StmtPtr, Error>;
  auto parse_module() -> Result<ModulePtr, Error>;
  auto parse_function_call(const std::string& function_name)
    -> Result<ExprPtr, Error>;
  auto parse_var_declaration() -> Result<ExprPtr, Error>;
};

// AST打印函数
auto print_ast(const StmtPtr& stmt, size_t indent = 0) -> void;
auto print_ast(const ModulePtr& module, size_t indent = 0) -> void;

}  // namespace Parser::Kaubo
