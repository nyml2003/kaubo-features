#pragma once

#include "Lexer/Token/Type.h"

#include <format>

namespace Lexer {
struct Coordinate {
  size_t line;
  size_t column;
};
}  // namespace Lexer

namespace Lexer::Token {

// Token结构体：包含类型、值、行列号
template <Constraint TokenType>
struct Proto {
  TokenType type;  // 带显式优先级的类型
  // 存储不同类型的值
  std::string value;
  Coordinate coordinate{};
};

}  // namespace Lexer::Token

namespace std {
template <Lexer::Token::Constraint TokenType>
inline auto to_string(const Lexer::Token::Proto<TokenType>& token)
  -> std::string {
  // 格式化输出：值(15字符) 类型(12字符) 行 列
  return std::format(
    "{:15} {:12} {:3} {:3}", token.value, to_string(token.type),
    token.coordinate.line, token.coordinate.column
  );
}
}  // namespace std