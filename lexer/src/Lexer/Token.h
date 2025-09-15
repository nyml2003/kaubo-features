#pragma once
#include <format>
#include <vector>
#include "Lexer/TokenType.h"
#include "Utils/Utf8.h"
namespace Lexer {
// Token结构体：包含类型、值、行列号
struct Token {
  TokenType type = TokenType::Utf8Error;  // 带显式优先级的类型
  // 存储不同类型的值
  std::variant<
    std::vector<char32_t>,  // 标识符、关键字、运算符内容、字符串内容、无效Token
    int64_t,                // 整数（严格区分）
    double,                 // 浮点数（严格区分）
    Utils::Utf8::Error      // UTF-8解码错误
    >
    value;
  size_t line{};    // 行号（1-based）
  size_t column{};  // 列号（按Unicode码点计数，1-based）
};

}  // namespace Lexer

namespace std {
inline auto to_string(const Lexer::Token& token) -> std::string {
  auto value_str = std::visit(
    [](auto&& arg) -> std::string {
      using T = std::decay_t<decltype(arg)>;
      if constexpr (std::is_same_v<T, std::vector<char32_t>> ||
                    std::is_same_v<T, int64_t> || std::is_same_v<T, double> ||
                    std::is_same_v<T, Utils::Utf8::Error>) {
        return std::to_string(arg);
      } else {
        assert(false && "未处理的Token值类型");
      }
    },
    token.value
  );
  // 格式化输出：值(15字符) 类型(12字符) 行 列
  return std::format(
    "{:15} {:12} {:3} {:3}", value_str, std::to_string(token.type), token.line,
    token.column
  );
}
}  // namespace std