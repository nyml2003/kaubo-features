#pragma once

#include <cstdint>
#include <string>
namespace Lexer {

enum TokenType : uint8_t {
  // 最高优先级：错误类型
  Utf8Error = 0,

  /*--- 关键字 ---*/
  Var = 1,      // var 关键字
  IntType = 2,  // int 类型关键字

  /*--- 高优先级：运算符 ---*/
  Plus = 5,      // 加法 + (一元或二元)
  Minus = 6,     // 减法 - (一元或二元)
  Multiply = 7,  // 乘法 *
  Divide = 8,    // 除法 /

  /*--- 比较运算符 ---*/
  EqualEqual = 9,     // 等于 ==
  NotEqual = 10,      // 不等于 !=
  GreaterEqual = 11,  // 大于等于 >=
  LessEqual = 12,     // 小于等于 <=
  Greater = 13,       // 大于 >
  Less = 14,          // 小于 <

  /*--- 中优先级：字面量和标识符 ---*/
  // 整数字面量（支持64位有符号整数）
  Integer = 20,
  Identifier = 21,  // 标识符

  /*--- 标点符号 ---*/
  Colon = 25,      // 冒号 :
  Equals = 26,     // 等号 =
  Comma = 27,      // 逗号 ,
  Semicolon = 28,  // 分号 ;

  /*--- 较低优先级：括号 ---*/
  LeftParen = 32,   // 左括号 (
  RightParen = 33,  // 右括号 )
  LeftBrace = 34,   // 左大括号 {
  RightBrace = 35,  // 右大括号 }

  // 空格和换行符
  WhiteSpace = 44,
  Tab = 45,  // 制表符
  NewLine = 46,

  InvalidToken = 255,
};

inline auto to_string(TokenType type) -> std::string {
  switch (type) {
    case Utf8Error:
      return "Utf8Error";
    case Var:
      return "Var";
    case IntType:
      return "IntType";
    case Plus:
      return "Plus";
    case Minus:
      return "Minus";
    case Multiply:
      return "Multiply";
    case Divide:
      return "Divide";
    case EqualEqual:
      return "EqualEqual";
    case NotEqual:
      return "NotEqual";
    case Greater:
      return "Greater";
    case Less:
      return "Less";
    case GreaterEqual:
      return "GreaterEqual";
    case LessEqual:
      return "LessEqual";
    case Integer:
      return "Integer";
    case Identifier:
      return "Identifier";
    case Colon:
      return "Colon";
    case Equals:
      return "Equals";
    case LeftParen:
      return "(";
    case RightParen:
      return ")";
    case LeftBrace:
      return "{";
    case RightBrace:
      return "}";
    case InvalidToken:
      return "InvalidToken";
    case WhiteSpace:
      return "Whitespace";
    case Tab:
      return "Tab";
    case NewLine:
      return "Newline";
    case Comma:
      return "Comma";
    case Semicolon:
      return "Semicolon";
  }
  return "UnknownToken";
}

}  // namespace Lexer