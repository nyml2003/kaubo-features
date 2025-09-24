#pragma once

#include "Parser/Parser.h"
namespace Parser::Utils {
inline auto get_precedence(TokenType op) -> int32_t {
  switch (op) {
    // 赋值运算符（最低优先级）
    case TokenType::Equal:
      return 5;

    // 比较运算符（优先级低于算术运算符）
    case TokenType::DoubleEqual:
    case TokenType::ExclamationEqual:
    case TokenType::GreaterThan:
    case TokenType::LessThan:
    case TokenType::GreaterThanEqual:
    case TokenType::LessThanEqual:
      return 10;  // 降低比较运算符优先级

    // 加法/减法（优先级高于比较，低于乘除）
    case TokenType::Plus:
    case TokenType::Minus:
      return 20;

    // 乘法/除法（最高优先级）
    case TokenType::Asterisk:
    case TokenType::Slash:
      return 30;

    // 成员访问运算符（如果之前添加了Dot，优先级应最高）
    case TokenType::Dot:
      return 40;

    default:
      return 0;
  }
}

inline auto get_associativity(TokenType op) -> bool {
  // 赋值运算符是右结合（a = b = c 等价于 a = (b = c)）
  if (op == TokenType::Equal) {
    return false;
  }
  // 其他运算符左结合
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