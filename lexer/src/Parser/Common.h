#pragma once

#include <memory>

namespace Parser {

namespace Expr {
class Expr;
}

using ExprPtr = std::shared_ptr<Expr::Expr>;

namespace Stmt {
class Stmt;
}

using StmtPtr = std::shared_ptr<Stmt::Stmt>;

}  // namespace Parser