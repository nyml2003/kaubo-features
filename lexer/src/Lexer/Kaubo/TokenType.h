#pragma once

#include <cstdint>
#include <string>
namespace Lexer::Kaubo {

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

  /*--- 中优先级：字面量和标识符 ---*/
  // 整数字面量（支持64位有符号整数）
  Integer = 20,
  Identifier = 21,  // 标识符

  /*--- 标点符号 ---*/
  Colon = 25,      // 冒号 :
  Equals = 26,     // 等号 =
  Semicolon = 27,  // 分号 ;

  /*--- 较低优先级：括号 ---*/
  LeftParen = 30,   // 左括号 (
  RightParen = 31,  // 右括号 )

  // 空格和换行符
  WhiteSpace = 40,
  Tab = 41,  // 制表符
  NewLine = 42,

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
    case InvalidToken:
      return "InvalidToken";
    case WhiteSpace:
      return "Whitespace";
    case Tab:
      return "Tab";
    case NewLine:
      return "Newline";
    case Semicolon:
      return "Semicolon";
  }
}

}  // namespace Lexer::Kaubo