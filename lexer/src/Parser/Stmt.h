#pragma once

#include "Common.h"

#include <string>
#include <variant>
#include <vector>

namespace Parser::Stmt {

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

// 变量声明语句
struct VarDecl {
  std::string name;
  ExprPtr initializer;
};

// If 语句
struct If {
  ExprPtr if_condition;
  std::vector<ExprPtr> elif_conditions;
  std::vector<StmtPtr> elif_bodies;
  StmtPtr else_body;
  StmtPtr then_body;
};

// While 语句
struct While {
  ExprPtr condition;
  StmtPtr body;
};

// For 语句
struct For {
  ExprPtr iterator;
  ExprPtr iterable;
  StmtPtr body;
};

struct Return {
  ExprPtr value;
};

class Stmt {
 public:
  using ValueType = std::variant<
    std::shared_ptr<Expr>,
    std::shared_ptr<Empty>,
    std::shared_ptr<Block>,
    std::shared_ptr<VarDecl>,
    std::shared_ptr<If>,
    std::shared_ptr<While>,
    std::shared_ptr<For>,
    std::shared_ptr<Return>>;

  explicit Stmt() = delete;

  template <typename T>
  explicit Stmt(std::shared_ptr<T> stmt) : m_value(std::move(stmt)) {}
  // 获取值类型的访问方法
  [[nodiscard]] auto get_value() const -> const ValueType& { return m_value; }

 private:
  ValueType m_value;
};

}  // namespace Parser::Stmt