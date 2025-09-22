#pragma once
#include "Parser/Module.h"

namespace Parser {

class Listener {
 private:
  size_t indent = 0;

 public:
  virtual auto on_enter_module() -> void = 0;
  virtual auto on_exit_module(const ModulePtr& module) -> void = 0;
  virtual auto on_enter_statement() -> void = 0;
  virtual auto on_exit_statement(const StmtPtr& stmt) -> void = 0;
  virtual auto on_enter_expr() -> void = 0;
  virtual auto on_exit_expr(const ExprPtr& expr) -> void = 0;
  virtual ~Listener() = default;
  explicit Listener() = default;
  Listener(const Listener&) = delete;
  auto operator=(const Listener&) -> Listener& = delete;
  Listener(Listener&&) = delete;
  auto operator=(Listener&&) -> Listener& = delete;
  void increase_indent() { ++indent; }
  void decrease_indent() { --indent; }
  [[nodiscard]] auto make_indent_str() const -> std::string {
    return std::string(this->indent * 2, ' ');
  }
  [[nodiscard]] auto get_indent() const -> size_t { return indent; }
};

using ListenerPtr = std::shared_ptr<Listener>;
}  // namespace Parser::Kaubo