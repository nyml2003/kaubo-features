#pragma once

#include "Parser/Parser.h"
namespace Parser::Utils {
inline auto get_precedence(TokenType op) -> int32_t {
  switch (op) {
    // 赋值运算符（最低优先级）
    case TokenType::Equals:
      return 5;

    // 比较运算符（二字符，优先级较高）
    case TokenType::EqualEqual:
    case TokenType::NotEqual:
    case TokenType::Greater:
    case TokenType::Less:
    case TokenType::GreaterEqual:
    case TokenType::LessEqual:
      return 15;

    // 算术运算符
    case TokenType::Plus:
    case TokenType::Minus:
      return 10;
    case TokenType::Multiply:
    case TokenType::Divide:
      return 20;
    default:
      return 0;
  }
}

inline auto get_associativity(TokenType /*op*/) -> bool {
  // 所有运算符都是左结合的
  return true;
}

template <typename T>
inline auto create(T&& obj) -> std::shared_ptr<T> {
  return std::make_shared<T>(std::forward<T>(obj));
}

template <typename T, typename... Args>
inline auto create(Args&&... args) -> std::shared_ptr<T> {
  return std::make_shared<T>(std::forward<Args>(args)...);
}

}  // namespace Parser::Utils