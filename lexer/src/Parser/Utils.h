#pragma once

#include "Parser/Parser.h"
namespace Parser::Utils {
inline auto get_precedence(TokenType op) -> int32_t {
  switch (op) {
    case TokenType::Equal:  // 假设这是赋值的Token类型
      return 50;
    // 逻辑或（低于And，高于赋值）
    case TokenType::Or:
      return 60;

    // 管道运算符
    case TokenType::Pipe:
      return 70;

    // 逻辑与（高于Or，低于比较运算符）
    case TokenType::And:
      return 80;

    // 比较运算符
    case TokenType::DoubleEqual:
    case TokenType::ExclamationEqual:
    case TokenType::GreaterThan:
    case TokenType::LessThan:
    case TokenType::GreaterThanEqual:
    case TokenType::LessThanEqual:
      return 100;

    // 加法/减法
    case TokenType::Plus:
    case TokenType::Minus:
      return 200;

    // 乘法/除法
    case TokenType::Asterisk:
    case TokenType::Slash:
      return 300;

    // 成员访问运算符
    case TokenType::Dot:
      return 400;

    // 逻辑非（一元运算符，优先级最高）
    case TokenType::Not:
      return 450;

    // 其他类型默认优先级为0（非运算符）
    default:
      return 0;
  }
}

inline auto get_associativity(TokenType /*unused*/) -> bool {
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