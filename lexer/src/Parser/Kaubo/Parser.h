#pragma once

#include "Lexer/Kaubo/TokenType.h"
#include "Lexer/Lexer.h"
#include "Utils/Result.h"

#include <cstdint>
#include <memory>
#include <optional>
#include <variant>
#include <vector>

namespace Parser::Kaubo {

using Lexer::Kaubo::TokenType;
using Utils::Result;

// 表达式AST节点类型
class Expr;
class Stmt;
using IntValue = int64_t;

// 二元运算符表达式
struct BinaryExpr {
  std::unique_ptr<Expr> left;
  TokenType op;
  std::unique_ptr<Expr> right;
};

// 一元运算符表达式
struct UnaryExpr {
  TokenType op;
  std::unique_ptr<Expr> operand;
};

// 括号表达式
struct GroupingExpr {
  std::unique_ptr<Expr> expression;
};

// 变量声明表达式
struct VarDeclExpr {
  std::string name;
  std::unique_ptr<Expr> initializer;
};

// 变量引用表达式
struct VarRefExpr {
  std::string name;
};

// 函数调用表达式
struct FunctionCallExpr {
  std::string function_name;
  std::vector<std::unique_ptr<Expr>> arguments;
};

// 赋值表达式
struct AssignExpr {
  std::string name;
  std::unique_ptr<Expr> value;
};

// 表达式语句
struct ExprStmt {
  std::unique_ptr<Expr> expression;
};

// 空语句
struct EmptyStmt {};

// Block语句（由{}包裹的语句列表）
struct BlockStmt {
  std::vector<std::unique_ptr<Stmt>> statements;
};

// Module（包含多个语句或block）
struct Module {
  std::vector<std::unique_ptr<Stmt>> statements;
};

// 语句变体类型
class Stmt {
 public:
  using ValueType = std::variant<
    std::unique_ptr<ExprStmt>,
    std::unique_ptr<EmptyStmt>,
    std::unique_ptr<BlockStmt>,
    std::unique_ptr<Expr>

    >;  // 兼容现有的表达式

  Stmt() = default;

  // 各种类型的构造函数
  // NOLINTNEXTLINE(google-explicit-constructor)
  Stmt(std::unique_ptr<ExprStmt> stmt) : m_value(std::move(stmt)) {}
  // NOLINTNEXTLINE(google-explicit-constructor)
  Stmt(std::unique_ptr<EmptyStmt> stmt) : m_value(std::move(stmt)) {}
  // NOLINTNEXTLINE(google-explicit-constructor)
  Stmt(std::unique_ptr<BlockStmt> stmt) : m_value(std::move(stmt)) {}
  // NOLINTNEXTLINE(google-explicit-constructor)
  Stmt(std::unique_ptr<Expr> expr) : m_value(std::move(expr)) {}

  // 获取值类型的访问方法
  [[nodiscard]] auto get() const -> const ValueType& { return m_value; }

 private:
  ValueType m_value;
};

// 表达式变体类型
class Expr {
 public:
  using ValueType = std::variant<
    IntValue,
    std::unique_ptr<BinaryExpr>,
    std::unique_ptr<UnaryExpr>,
    std::unique_ptr<GroupingExpr>,
    std::unique_ptr<VarDeclExpr>,
    std::unique_ptr<VarRefExpr>,
    std::unique_ptr<FunctionCallExpr>,
    std::unique_ptr<AssignExpr>>;

  Expr() = default;

  // 各种类型的构造函数
  // NOLINTNEXTLINE(google-explicit-constructor)
  Expr(IntValue n) : m_value(n) {}
  // NOLINTNEXTLINE(google-explicit-constructor)
  Expr(std::unique_ptr<BinaryExpr> expr) : m_value(std::move(expr)) {}
  // NOLINTNEXTLINE(google-explicit-constructor)
  Expr(std::unique_ptr<UnaryExpr> expr) : m_value(std::move(expr)) {}
  // NOLINTNEXTLINE(google-explicit-constructor)
  Expr(std::unique_ptr<GroupingExpr> expr) : m_value(std::move(expr)) {}
  // NOLINTNEXTLINE(google-explicit-constructor)
  Expr(std::unique_ptr<VarDeclExpr> expr) : m_value(std::move(expr)) {}
  // NOLINTNEXTLINE(google-explicit-constructor)
  Expr(std::unique_ptr<VarRefExpr> expr) : m_value(std::move(expr)) {}
  // NOLINTNEXTLINE(google-explicit-constructor)
  Expr(std::unique_ptr<FunctionCallExpr> expr) : m_value(std::move(expr)) {}
  // NOLINTNEXTLINE(google-explicit-constructor)
  Expr(std::unique_ptr<AssignExpr> expr) : m_value(std::move(expr)) {}

  // 获取值类型的访问方法
  [[nodiscard]] auto get() const -> const ValueType& { return m_value; }

 private:
  ValueType m_value;
};

// 解析错误类型
enum class ParseError : uint8_t {
  UnexpectedToken,
  UnexpectedEndOfInput,
  InvalidNumberFormat,
  MissingRightParen,
  DivisionByZero
};

// Pratt parser实现
class Parser {
 public:
  explicit Parser(Lexer::Instance<TokenType> lexer)
    : m_lexer(std::move(lexer)) {
    consume();  // 预读第一个token
  }

  auto parse() -> Result<Module, ParseError>;

  // AST打印函数
  static auto print_ast(const Expr& expr, size_t indent = 0) -> void;

 private:
  Lexer::Instance<TokenType> m_lexer;
  std::optional<Lexer::Token<TokenType>> current_token;

  // 消费当前token并读取下一个
  void consume();

  // 检查当前token是否为指定类型
  [[nodiscard]] auto check(TokenType type) const -> bool;

  // 检查并消费指定类型的token
  auto match(TokenType type) -> bool;

  // 期望并消费指定类型的token，否则返回错误
  auto expect(TokenType type) -> Result<void, ParseError>;

  // Pratt解析方法
  auto parse_expression(int32_t precedence = 0) -> Result<Expr, ParseError>;
  auto parse_primary() -> Result<Expr, ParseError>;
  auto parse_unary() -> Result<Expr, ParseError>;
  auto parse_statement() -> Result<std::unique_ptr<Stmt>, ParseError>;
  auto parse_block() -> Result<std::unique_ptr<BlockStmt>, ParseError>;
  auto parse_module() -> Result<Module, ParseError>;
  auto parse_function_call(const std::string& function_name)
    -> Result<Expr, ParseError>;
  auto parse_var_declaration() -> Result<Expr, ParseError>;

  // 获取运算符的优先级和结合性
  [[nodiscard]] static auto get_precedence(TokenType op) -> int32_t;
  [[nodiscard]] static auto get_associativity(TokenType op)
    -> bool;  // true for left, false for right
};

// AST打印函数
auto print_ast(const Stmt& stmt, size_t indent = 0) -> void;
auto print_ast(const Module& module, size_t indent = 0) -> void;

}  // namespace Parser::Kaubo

namespace std {
using Parser::Kaubo::ParseError;
inline auto to_string(ParseError error) -> const char* {
  switch (error) {
    case ParseError::UnexpectedToken:
      return "Unexpected token";
    case ParseError::UnexpectedEndOfInput:
      return "Unexpected end of input";
    case ParseError::InvalidNumberFormat:
      return "Invalid number format";
    case ParseError::MissingRightParen:
      return "Missing right parenthesis";
    case ParseError::DivisionByZero:
      return "Division by zero";
  }
}
}  // namespace std