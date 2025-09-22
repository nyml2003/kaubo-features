#pragma once
#include "Lexer/Type.h"

#include <memory>
#include <variant>
#include <vector>

namespace Parser::Expr {
class Expr;
}  // namespace Parser::Expr

namespace Parser {
using ExprPtr = std::shared_ptr<Expr::Expr>;
}

namespace Parser::Expr {
using Lexer::TokenType;

// 整数字面量表达式
using IntValue = int64_t;

// 二元运算符表达式
struct Binary {
  ExprPtr left;
  TokenType op;
  ExprPtr right;
};

// 一元运算符表达式
struct Unary {
  TokenType op;
  ExprPtr operand;
};

// 括号表达式
struct Grouping {
  ExprPtr expression;
};
// 变量引用表达式
struct VarRef {
  std::string name;
};

// 函数调用表达式
struct FunctionCall {
  std::string function_name;
  std::vector<ExprPtr> arguments;
};

// 赋值表达式
struct Assign {
  std::string name;
  ExprPtr value;
};

class Expr {
 public:
  using ValueType = std::variant<
    IntValue,
    std::shared_ptr<Binary>,
    std::shared_ptr<Unary>,
    std::shared_ptr<Grouping>,
    std::shared_ptr<VarRef>,
    std::shared_ptr<FunctionCall>,
    std::shared_ptr<Assign>>;

  explicit Expr() = delete;

  explicit Expr(IntValue n) : m_value(n) {}
  template <typename T>
  explicit Expr(std::shared_ptr<T> expr) : m_value(std::move(expr)) {}

  // 获取值类型的访问方法
  [[nodiscard]] auto get_value() const -> const ValueType& { return m_value; }

 private:
  ValueType m_value;
};

}  // namespace Parser::Expr