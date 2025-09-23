#pragma once

#include <cstdint>
namespace Parser {
// 解析错误类型
enum class Error : uint8_t {
  UnexpectedToken,                   // 遇到意外的标记
  UnexpectedEndOfInput,              // 遇到意外的输入结束
  InvalidNumberFormat,               // 遇到无效的数字格式
  MissingRightParen,                 // 缺少右括号
  DivisionByZero,                    // 除以零
  ExpectedLeftBraceAfterArrow,       // 预期在箭头后有一个左大括号
  ExpectedCommaOrRightParen,         // 预期逗号或右括号
  MissingRightBrace,                 // 缺少右大括号
  ExpectedIdentifierAfterDot,        // 预期在点后有一个标识符
  ExpectedPipe,                      // 预期管道
  ExpectedIdentifierInLambdaParams,  // 预期在Lambda参数中有一个标识符
  ExpectedCommaOrPipeInLambda,       // 预期在Lambda中有一个逗号或管道
  ExpectedLeftBraceInLambdaBody,     // 预期在Lambda体中有一个左大括号
};
}  // namespace Parser

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
    case Error::ExpectedLeftBraceAfterArrow:
      return "Expected left brace after arrow";
    case Error::ExpectedCommaOrRightParen:
      return "Expected comma or right parenthesis";
    case Error::MissingRightBrace:
      return "Missing right brace";
    case Error::ExpectedIdentifierAfterDot:
      return "Expected identifier after dot";
    case Error::ExpectedPipe:
      return "Expected pipe";
    case Error::ExpectedIdentifierInLambdaParams:
      return "Expected identifier in lambda parameters";
    case Error::ExpectedCommaOrPipeInLambda:
      return "Expected comma or pipe in lambda";
    case Error::ExpectedLeftBraceInLambdaBody:
      return "Expected left brace in lambda body";
  }
}
}  // namespace std