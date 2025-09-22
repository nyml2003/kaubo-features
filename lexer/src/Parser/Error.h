#pragma once

#include <cstdint>
namespace Parser {
// 解析错误类型
enum class Error : uint8_t {
  UnexpectedToken,
  UnexpectedEndOfInput,
  InvalidNumberFormat,
  MissingRightParen,
  DivisionByZero
};
}  // namespace Parser::Kaubo

namespace std {
using Parser::Error;
inline auto to_string(Error error) -> const char* {
  switch (error) {
    case Error::UnexpectedToken:
      return "Unexpected token";
    case Error::UnexpectedEndOfInput:
      return "Unexpected end of input";
    case Error::InvalidNumberFormat:
      return "Invalid number format";
    case Error::MissingRightParen:
      return "Missing right parenthesis";
    case Error::DivisionByZero:
      return "Division by zero";
  }
}
}  // namespace std