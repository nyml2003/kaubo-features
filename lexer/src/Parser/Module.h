#pragma once

#include "Stmt.h"
namespace Parser {
struct Module {
  std::vector<StmtPtr> statements;
};

using ModulePtr = std::shared_ptr<Module>;
}  // namespace Parser