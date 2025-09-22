#pragma once

#include "Expr.h"
#include "Lexer/Core/Proto.h"
#include "Lexer/Token/Constraint.h"
#include "Parser/Error.h"
#include "Parser/Expr.h"
#include "Parser/Listener.h"
#include "Parser/Module.h"
#include "Parser/Stmt.h"
#include "Utils/Overloaded.h"
#include "Utils/Result.h"

#include <iostream>

namespace Parser {
using Lexer::TokenType;
using Utils::Err;
using Utils::Ok;
using Utils::Result;

// Pratt parser实现
class Parser {
 public:
  explicit Parser(Lexer::Instance<TokenType> lexer)
    : m_lexer(std::move(lexer)) {
    consume();  // 预读第一个token
  }

  auto parse() -> Result<ModulePtr, Error>;

  auto bind_listener(const ListenerPtr& listener) -> void {
    listeners.push_back(listener);
  }

 private:
  Lexer::Instance<TokenType> m_lexer;
  std::optional<Lexer::Token::Proto<TokenType>> current_token;

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
  auto parse_var_declaration() -> Result<StmtPtr, Error>;

  std::vector<ListenerPtr> listeners;

  auto enter_module() -> void {
    for (const auto& listener : listeners) {
      listener->on_enter_module();
    }
  }
  auto exit_module(const ModulePtr& module) -> void {
    for (const auto& listener : listeners) {
      listener->on_exit_module(module);
    }
  }
  auto enter_statement() -> void {
    for (const auto& listener : listeners) {
      listener->on_enter_statement();
    }
  }
  auto exit_statement(const StmtPtr& stmt) -> void {
    for (const auto& listener : listeners) {
      listener->on_exit_statement(stmt);
    }
  }
  auto enter_expr() -> void {
    for (const auto& listener : listeners) {
      listener->on_enter_expr();
    }
  }
  auto exit_expr(const ExprPtr& expr) -> void {
    for (const auto& listener : listeners) {
      listener->on_exit_expr(expr);
    }
  }
};

inline auto print_ast(const ExprPtr& expr, size_t indent) -> void {
  // 缩进字符串
  std::string indent_str(indent * 2, ' ');

  // 使用访问者模式处理不同类型的表达式
  std::visit(
    overloaded{
      [&](Expr::IntValue int_value_expr) {
        std::cout << indent_str << int_value_expr << '\n';
      },
      [&](const std::shared_ptr<Expr::Binary>& binary_expr) {
        std::cout << indent_str << "BinaryExpr" << '\n';
        std::cout << indent_str << "  " << Lexer::to_string(binary_expr->op)
                  << '\n';
        print_ast(binary_expr->left, indent + 1);
        print_ast(binary_expr->right, indent + 1);
      },
      [&](const std::shared_ptr<Expr::Unary>& unary_expr) {
        std::cout << indent_str << "UnaryExpr" << '\n';
        std::cout << indent_str << "  " << Lexer::to_string(unary_expr->op)
                  << '\n';
        print_ast(unary_expr->operand, indent + 1);
      },
      [&](const std::shared_ptr<Expr::VarRef>& var_ref_expr) {
        std::cout << indent_str << var_ref_expr->name << '\n';
      },
      [&](const std::shared_ptr<Expr::FunctionCall>& function_call_expr)
        -> void {
        std::cout << indent_str << "FunctionCall" << '\n';
        std::cout << indent_str << "  " << function_call_expr->function_name
                  << '\n';
        for (const auto& arg : function_call_expr->arguments) {
          print_ast(arg, indent + 1);
        }
      },
      [&](const std::shared_ptr<Expr::Grouping>& grouping_expr) -> void {
        std::cout << indent_str << "GroupingExpr" << '\n';
        print_ast(grouping_expr->expression, indent + 1);
      },
      [&](const std::shared_ptr<Expr::Assign>& var_assign_expr) -> void {
        std::cout << indent_str << "VarAssignExpr" << '\n';
        std::cout << indent_str << "  " << var_assign_expr->name << '\n';
        print_ast(var_assign_expr->value, indent + 1);
      }
    },
    expr->get_value()
  );
}

inline auto print_ast(const StmtPtr& stmt, size_t indent) -> void {
  // 缩进字符串
  std::string indent_str(indent * 2, ' ');

  // 使用访问者模式处理不同类型的语句
  std::visit(
    overloaded{
      [&](const std::shared_ptr<Stmt::Expr>& expr_stmt) {
        std::cout << indent_str << "ExprStmt:" << '\n';
        if (expr_stmt->expression) {
          print_ast(expr_stmt->expression, indent + 1);
        }
      },
      [&](const std::shared_ptr<Stmt::Empty>& /*empty_stmt*/) {
        std::cout << indent_str << "EmptyStmt;" << '\n';
      },
      [&](const std::shared_ptr<Stmt::Block>& block_stmt) {
        std::cout << indent_str << "BlockStmt" << '\n';
        for (const auto& stmt : block_stmt->statements) {
          print_ast(stmt, indent + 1);
        }
      },
      [&](const std::shared_ptr<Stmt::VarDecl>& var_decl_stmt) -> void {
        std::cout << indent_str << "VarDeclStmt" << var_decl_stmt->name << " = "
                  << '\n';
        if (var_decl_stmt->initializer) {
          print_ast(var_decl_stmt->initializer, indent + 1);
        }
      }
    },
    stmt->get_value()
  );
}

inline auto print_ast(const ModulePtr& module, size_t indent) -> void {
  // 缩进字符串
  std::string indent_str(indent * 2, ' ');

  std::cout << indent_str << "Module:" << '\n';
  for (const auto& stmt : module->statements) {
    print_ast(stmt, indent + 1);
  }
}

}  // namespace Parser
