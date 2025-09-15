#pragma once
#include <iostream>
#include <string>
#include "Lexer/Token.h"
#include "Utils/Utf8.h"

// ANSI颜色代码
namespace Color {
const std::string RESET = "\033[0m";
const std::string RED = "\033[31m";
const std::string GREEN = "\033[32m";
const std::string YELLOW = "\033[33m";
const std::string BLUE = "\033[34m";
const std::string MAGENTA = "\033[35m";
const std::string CYAN = "\033[36m";
const std::string WHITE = "\033[37m";
const std::string BOLD = "\033[1m";
const std::string GREY = "\033[90m";

// 根据Token类型获取对应的颜色
inline auto get_color(Lexer::TokenType type) -> std::string {
  switch (type) {
    case Lexer::TokenType::Utf8Error:
      return Color::RED + Color::BOLD;
    case Lexer::TokenType::Boolean:
      return Color::MAGENTA;
      return Color::MAGENTA;
    case Lexer::TokenType::Keyword:
      return Color::GREEN + Color::BOLD;
    case Lexer::TokenType::String:
      return Color::YELLOW;
    case Lexer::TokenType::Integer:
    case Lexer::TokenType::Float:
      return Color::CYAN;
    case Lexer::TokenType::Operator3:
    case Lexer::TokenType::Operator2:
    case Lexer::TokenType::Operator1:
      return Color::RED;
    case Lexer::TokenType::Identifier:
      return Color::BLUE;

    case Lexer::TokenType::InvalidToken:
      return Color::RED;
    default:
      return Color::WHITE;
  }
}

inline auto repeat(const std::string& str, size_t n) -> std::string {
  if (n <= 0) {
    return "";  // 处理无效输入
  }
  std::string result;
  result.reserve(str.size() * n);  // 预分配内存
  for (size_t i = 0; i < n; ++i) {
    result += str;
  }
  return result;
}

// 恢复并高亮显示源代码
class SourceHighlighter {
 private:
  size_t current_line = 0;     // 当前行号
  size_t current_column = 0;   // 当前列号
  bool is_first_token = true;  // 是否是第一个Token

 public:
  // 流式处理单个Token
  void process_token(const Lexer::Token& token) {
    if (is_first_token) {
      // 初始化行列号
      current_line = token.line;
      current_column = token.column;
      is_first_token = false;
    }

    // 处理换行
    if (token.line > current_line) {
      size_t line_diff = token.line - current_line;
      // 输出换行
      std::cout << std::string(line_diff, '\n');
      // 更新行号
      current_line = token.line;
      // 重置列号并补充空格到当前token的列
      size_t spaces = token.column;  // 新行从1列开始
      if (spaces > 1) {
        std::cout << repeat("·", spaces - 1);
      }
      current_column = token.column;
    }
    // 处理同一行内的空格
    else if (token.line == current_line && token.column > current_column) {
      size_t spaces = token.column - current_column;

      std::cout << repeat("·", spaces);

      current_column = token.column;
    }

    // 解析Token文本
    std::string token_text = std::visit(
      [&token](const auto& value) -> std::string {
        using T = std::decay_t<decltype(value)>;
        if constexpr (std::is_same_v<T, std::string_view>) {
          // 如果token.type是string
          if (token.type == Lexer::TokenType::String) {
            return std::format("\"{}\"", value);
          }
          return std::format("{}", value);
        } else if constexpr (std::is_same_v<T, int64_t> ||
                             std::is_same_v<T, double>) {
          return std::format("{}", value);
        } else if constexpr (std::is_same_v<T, Utils::Utf8::Error>) {
          return std::format("[UTF8 Error: {}]", std::to_string(value));
        } else {
          return std::format("[Unknown Token Value]");
        }
      },
      token.value
    );

    // 高亮并输出
    std::string highlighted_text =
      std::format("{}{}{}", get_color(token.type), token_text, Color::RESET);
    std::cout << highlighted_text;

    // 更新当前列号（加上当前token文本的长度）
    current_column += token_text.length();
  }

  // 结束流式处理（非静态版本）
  static void finalize() {
    // 可以根据需要添加最终处理，比如确保最后有一个换行
    std::cout << '\n';
  }
};
}  // namespace Color