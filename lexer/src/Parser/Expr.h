#pragma once
#include "Common.h"
#include "Lexer/Type.h"

#include <variant>
#include <vector>

namespace Parser::Expr {
using Lexer::TokenType;

// 整数字面量表达式
struct LiteralInt {
  int64_t value;
};

struct LiteralString {
  std::string value;
};

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
  ExprPtr function_expr;
  std::vector<ExprPtr> arguments;
};

// 赋值表达式
struct Assign {
  std::string name;
  ExprPtr value;
};

// 匿名函数表达式
struct Lambda {
  std::vector<std::string> params;
  Parser::StmtPtr body;
};

// 成员访问表达式
struct MemberAccess {
  ExprPtr object;      // 成员所属的对象（如 a，类型为 ExprPtr）
  std::string member;  // 成员名（如 b，字符串）
};

class Expr {
 public:
  using ValueType = std::variant<
    std::shared_ptr<LiteralInt>,
    std::shared_ptr<LiteralString>,
    std::shared_ptr<Binary>,
    std::shared_ptr<Unary>,
    std::shared_ptr<Grouping>,
    std::shared_ptr<VarRef>,
    std::shared_ptr<FunctionCall>,
    std::shared_ptr<Assign>,
    std::shared_ptr<Lambda>,
    std::shared_ptr<MemberAccess>>;

  explicit Expr() = delete;

  template <typename T>
  explicit Expr(std::shared_ptr<T> expr) : m_value(std::move(expr)) {}

  // 获取值类型的访问方法
  [[nodiscard]] auto get_value() const -> const ValueType& { return m_value; }

 private:
  ValueType m_value;
};

}  // namespace Parser::Expr