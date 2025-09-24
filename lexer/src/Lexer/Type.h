#pragma once

#include <cstdint>
#include <string>
namespace Lexer {

// 枚举名：优先体现「字符形态」，其次体现「类型归类」
enum TokenType : uint8_t {
  // 错误/状态类型（无需字符导向，保留语义）
  Utf8Error = 0,

  Comment = 1,  // 注释（无需字符导向，保留语义）

  /*--- 关键字（字符形态即关键字本身，保留原名）---*/
  Var = 11,        // var
  If = 12,         // if
  Else = 13,       // else
  Elif = 14,       // elif
  While = 15,      // while
  For = 16,        // for
  Return = 17,     // return
  In = 18,         // in
  Yield = 19,      // yield
  True = 20,       // true
  False = 21,      // false
  Null = 22,       // null
  Break = 23,      // break
  Continue = 24,   // continue
  Struct = 25,     // struct
  Interface = 26,  // interface
  Import = 27,     // import
  As = 28,         // as
  From = 29,       // from
  Pass = 30,       // pass
  And = 31,        // and
  Or = 32,         // or
  Not = 33,        // not
  Async = 34,      // async
  Await = 35,      // await

  /*--- 字面量（描述“字符内容类型”，保留语义归类）---*/
  Literal_Integer = 100,  // 数字字符组合（如 123）
  Literal_String = 101,   // 字符串字符组合（如 "abc"）

  /*--- 标识符（描述“字符构成规则”，保留语义）---*/
  Identifier = 120,  // 字母/下划线开头的字符组合（如 name）

  /*--- 双字符符号（突出“组合特征”）---*/
  DoubleEqual = 130,       // == （双等号）
  ExclamationEqual = 131,  // != （感叹号+等号）
  GreaterThanEqual = 132,  // >= （大于号+等号）
  LessThanEqual = 133,     // <= （小于号+等号）

  /*--- 单字符符号（突出“单个字符”）---*/
  GreaterThan = 150,         // > （大于号）
  LessThan = 151,            // < （小于号）
  Plus = 152,                // + （加号）
  Minus = 153,               // - （减号）
  Asterisk = 154,            // * （星号）
  Slash = 155,               // / （斜杠）
  Colon = 156,               // : （冒号）
  Equal = 157,               // = （等号）
  Comma = 158,               // , （逗号）
  Semicolon = 159,           // ; （分号）
  LeftParenthesis = 160,     // ( （左括号）
  RightParenthesis = 161,    // ) （右括号）
  LeftCurlyBrace = 162,      // { （左大括号）
  RightCurlyBrace = 163,     // } （右大括号）
  LeftSquareBracket = 164,   // [ （左中括号）
  RightSquareBracket = 165,  // ] （右中括号）
  Dot = 166,                 // . （点号）
  Pipe = 167,                // | （竖线）

  // 空白字符（直接描述字符类型）
  Whitespace = 240,  // 空格（原 WhiteSpace，修正大小写一致性）
  Tab = 241,         // 制表符（明确“字符”属性）
  NewLine = 242,     // 换行符（明确“字符”属性）

  InvalidToken = 255,
};

// 返回值：直接返回 Token 对应的「字面字符」或「类型描述」
inline auto to_string(TokenType type) -> std::string {
  switch (type) {
    // 错误/状态
    case Utf8Error:
      return "Utf8Error";
    case InvalidToken:
      return "InvalidToken";

    // 关键字（直接返回关键字本身）
    case Var:
      return "var";
    case If:
      return "if";
    case Else:
      return "else";
    case Elif:
      return "elif";
    case While:
      return "while";
    case For:
      return "for";
    case Return:
      return "return";
    case In:
      return "in";
    case Yield:
      return "yield";
    case True:
      return "true";
    case False:
      return "false";
    case Null:
      return "null";
    case Break:
      return "break";
    case Continue:
      return "continue";
    case Struct:
      return "struct";
    case Interface:
      return "interface";
    case Import:
      return "import";
    case As:
      return "as";
    case From:
      return "from";
    case Pass:
      return "pass";
    case And:
      return "and";
    case Or:
      return "or";
    case Not:
      return "not";
    case Async:
      return "async";
    case Await:
      return "await";

    // 字面量（描述类型）
    case Literal_Integer:
      return "Integer";
    case Literal_String:
      return "String";

    // 标识符
    case Identifier:
      return "Identifier";

    // 双字符符号（返回字符组合）
    case DoubleEqual:
      return "==";
    case ExclamationEqual:
      return "!=";
    case GreaterThanEqual:
      return ">=";
    case LessThanEqual:
      return "<=";

    // 单字符符号（返回单个字符）
    case GreaterThan:
      return ">";
    case LessThan:
      return "<";
    case Plus:
      return "+";
    case Minus:
      return "-";
    case Asterisk:
      return "*";
    case Slash:
      return "/";
    case Colon:
      return ":";
    case Equal:
      return "=";
    case Comma:
      return ",";
    case Semicolon:
      return ";";
    case LeftParenthesis:
      return "(";
    case RightParenthesis:
      return ")";
    case LeftCurlyBrace:
      return "{";
    case RightCurlyBrace:
      return "}";
    case LeftSquareBracket:
      return "[";
    case RightSquareBracket:
      return "]";
    case Dot:
      return ".";
    case Pipe:
      return "|";

    // 空白字符
    case Whitespace:
      return "Whitespace";
    case Tab:
      return "Tab";
    case NewLine:
      return "NewLine";
    case Comment:
      return "Comment";
  }
  return "UnknownToken";
}

}  // namespace Lexer