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

  /*--- 字面量 ---*/
  Integer = 100,  // 64位有符号整数
  String = 101,   // 字符串字面量

  /*--- 标识符 ---*/
  Identifier = 120,

  /*--- 二字符运算符 ---*/
  EqualEqual = 130,    // 等于 ==
  NotEqual = 131,      // 不等于 !=
  GreaterEqual = 132,  // 大于等于 >=
  LessEqual = 133,     // 小于等于 <=
  RightArrow = 134,    // 右箭头 ->

  /*--- 一字符运算符 ---*/
  Greater = 150,     // 大于 >
  Less = 151,        // 小于 <
  Plus = 152,        // 加法 + (一元或二元)
  Minus = 153,       // 减法 - (一元或二元)
  Multiply = 154,    // 乘法 *
  Divide = 155,      // 除法 /
  Colon = 156,       // 冒号 :
  Equals = 157,      // 等号 =
  Comma = 158,       // 逗号 ,
  Semicolon = 159,   // 分号 ;
  LeftParen = 160,   // 左括号 (
  RightParen = 161,  // 右括号 )
  LeftBrace = 162,   // 左大括号 {
  RightBrace = 163,  // 右大括号 }

  // 空格和换行符
  WhiteSpace = 240,
  Tab = 241,  // 制表符
  NewLine = 242,

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
      return "==";
    case NotEqual:
      return "!=";
    case RightArrow:
      return "->";
    case Greater:
      return "Greater";
    case Less:
      return "Less";
    case GreaterEqual:
      return ">=";
    case LessEqual:
      return "<=";
    case Integer:
      return "Integer";
    case String:
      return "String";
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