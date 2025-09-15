#pragma once
#include <cassert>
#include <cstdint>
#include <string>

namespace Lexer {

// TokenType枚举：显式指定值表示优先级（值越小优先级越高）
enum class TokenType : uint8_t {
  // 最高优先级：错误和特殊类型
  Utf8Error = 0,  // UTF-8解码错误（最高优先级，必须优先处理）

  /*---高优先级---*/
  /**
   * @brief 布尔值
   * - true
   * - false
   */
  Boolean = 5,

  /**
   * @brief 空值
   * null
   */
  Null = 6,

  /**
   * @brief 关键字
   */
  Keyword = 7,

  /**
   * @brief 字符串字面量
   * - 单引号：'hello'
   * - 双引号："hello"
   */
  String = 10,

  /**
   * @brief 整数字面量
   * - 十进制：123
   */
  Integer = 11,

  /**
   * @brief 浮点数字面量
   * - 十进制：123.456
   */
  Float = 12,

  /* ----运算符----*/
  /**
   * @brief 三字符运算符
   * - ===
   * - !==
   */
  Operator3 = 20,

  /**
   * @brief 二字符运算符
   * - >=
   * - <=
   * - ==
   * - !=
   * - &&
   * - ||
   * - ??
   */
  Operator2 = 21,

  /**
   * @brief 单字符运算符
   * ~!@#$%^&*()-+=[]{}|\\;:,.<>?/
   */
  Operator1 = 22,

  /* ----标识符----*/
  Identifier = 30,

  /*---最低优先级---*/
  Eof = 40,

  WhiteSpace = 41,  // 空白字符

  // 最低优先级：无效Token（最后匹配）
  InvalidToken = 50  // 合法UTF-8但无匹配规则
};

inline auto ahead_of(TokenType a, TokenType b) -> bool {
  return static_cast<uint8_t>(a) < static_cast<uint8_t>(b);
}

}  // namespace Lexer

namespace std {
inline auto to_string(const Lexer::TokenType& type) -> std::string {
  switch (type) {
    case Lexer::TokenType::Utf8Error:
      return "Utf8Error";
    case Lexer::TokenType::Boolean:
      return "Boolean";
    case Lexer::TokenType::Null:
      return "Null";
    case Lexer::TokenType::Keyword:
      return "Keyword";
    case Lexer::TokenType::String:
      return "String";
    case Lexer::TokenType::Integer:
      return "Integer";
    case Lexer::TokenType::Float:
      return "Float";
    case Lexer::TokenType::Operator3:
    case Lexer::TokenType::Operator2:
    case Lexer::TokenType::Operator1:
      return "Operator";
    case Lexer::TokenType::Identifier:
      return "Identifier";
    case Lexer::TokenType::InvalidToken:
      return "InvalidToken";
    case Lexer::TokenType::Eof:
      return "Eof";
    case Lexer::TokenType::WhiteSpace:
      return "WhiteSpace";
    default:
      assert(false && "未处理的TokenType");
  }
}
}  // namespace std
