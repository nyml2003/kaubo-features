// #pragma once

// #include <cstdint>
// #include <string>
// namespace Lexer::Json {

// enum TokenType : uint8_t {
//   // 最高优先级：错误类型（优先处理错误）
//   Utf8Error = 0,

//   /*--- 高优先级：关键字类字面量 ---*/
//   // 布尔值（true/false）
//   Bool = 5,
//   // 空值（null）
//   Null = 6,

//   /*--- 中优先级：基础数据类型字面量 ---*/
//   // 字符串字面量（"xxx"）
//   String = 10,
//   // 整数字面量（123）
//   Integer = 11,

//   /*--- 较低优先级：结构符号（单字符分隔符/括号） ---*/
//   // 左方括号 [
//   LeftBracket = 20,
//   // 右方括号 ]
//   RightBracket = 21,
//   // 左花括号 {
//   LeftCurly = 22,
//   // 右花括号 }
//   RightCurly = 23,
//   // 冒号 :
//   Colon = 24,
//   // 逗号 ,
//   Comma = 25,

//   // 空格（ ）、制表符（\t）
//   WhiteSpace = 30,
//   Tab = 31,
//   // 换行符（\n、\r\n）
//   NewLine = 32,

//   InvalidToken = 255,
// };

// inline auto to_string(TokenType type) -> std::string {
//   switch (type) {
//     case Utf8Error:
//       return "Utf8Error";
//     case Bool:
//       return "Bool";
//     case Null:
//       return "Null";
//     case String:
//       return "String";
//     case Integer:
//       return "Integer";
//     case LeftBracket:
//       return "[";
//     case RightBracket:
//       return "]";
//     case LeftCurly:
//       return "{";
//     case RightCurly:
//       return "}";
//     case Colon:
//       return ":";
//     case Comma:
//       return ",";
//     case InvalidToken:
//       return "InvalidToken";
//     case WhiteSpace:
//       return "Whitespace";
//     case NewLine:
//       return "Newline";
//     case Tab:
//       return "Tab";
//     default:
//       return "Unknown";
//   }
// }
// }  // namespace Lexer::Json