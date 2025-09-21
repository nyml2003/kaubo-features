#pragma once

#include "Expr.h"

namespace Parser::Kaubo::Stmt {
class Stmt;
}  // namespace Parser::Kaubo::Stmt

namespace Parser::Kaubo {
using StmtPtr = std::shared_ptr<Stmt::Stmt>;
}

namespace Parser::Kaubo::Stmt {

// 表达式语句
struct Expr {
  ExprPtr expression;
};

// 空语句
struct Empty {};

// Block语句（由{}包裹的语句列表）
struct Block {
  std::vector<StmtPtr> statements;
};

class Stmt {
 public:
  using ValueType = std::variant<
    std::shared_ptr<Expr>,
    std::shared_ptr<Empty>,
    std::shared_ptr<Block>>;  // 兼容现有的表达式

  explicit Stmt() = delete;

  template <typename T>
  explicit Stmt(std::shared_ptr<T> stmt) : m_value(std::move(stmt)) {}
  // 获取值类型的访问方法
  [[nodiscard]] auto get_value() const -> const ValueType& { return m_value; }

 private:
  ValueType m_value;
};

}  // namespace Parser::Kaubo::Stmt