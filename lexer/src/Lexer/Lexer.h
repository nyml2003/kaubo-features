#pragma once
#include <cassert>

#include <functional>
#include <optional>
#include <string>
#include <string_view>

#include "Lexer/Token.h"
#include "Lexer/TokenType.h"
#include "Result.h"
#include "Utf8Utils.h"

namespace Lexer {

// -------------------------- Token匹配器定义
// --------------------------
//
// 匹配器函数：输入缓冲区、当前位置(引用)、行列号(引用)，返回匹配的Token(可选)

using TokenMatcher = std::function<std::optional<
  Token>(std::string_view input, size_t& pos, size_t& line, size_t& column)>;

// -------------------------- 可扩展Lexer类
// --------------------------
class ExtensibleLexer {
 private:
  std::string buffer;                  // 输入缓冲区
  size_t pos = 0;                      // 当前字节位置(0-based)
  size_t line = 1;                     // 当前行号(1-based)
  size_t column = 1;                   // 当前列号(1-based)
  std::vector<TokenMatcher> matchers;  // 匹配器列表(按注册顺序即优先级)
  std::optional<Utf8Utils::Utf8Error> last_utf8_error;  // 最近UTF-8错误

  // 解码UTF-8码点（辅助函数）
  auto decode_utf8(std::string_view input, size_t decode_pos)
    -> std::optional<std::pair<char32_t, size_t>> {
    auto result = Utf8Utils::get_utf8_codepoint(input, decode_pos);
    if (result.is_err()) {
      last_utf8_error = result.unwrap_err();
      return std::nullopt;
    }
    last_utf8_error = std::nullopt;
    return result.unwrap();
  }

  // 跳过Unicode空白字符
  void skip_whitespace() {
    while (pos < buffer.size()) {
      auto codepoint_len = decode_utf8(buffer, pos);
      if (!codepoint_len) {
        break;  // 解码错误，停止跳过
      }

      auto [code_point, len] = codepoint_len.value();
      if (Utf8Utils::is_unicode_whitespace(code_point)) {
        if (code_point == U'\n') {  // 换行：更新行号，重置列号
          line++;
          column = 1;
        } else {
          column++;  // 其他空白：列号+1
        }
        pos += len;
      } else {
        break;  // 非空白，停止跳过
      }
    }
  }

  // 收缩缓冲区（避免内存膨胀）
  void shrink_buffer() {
    if (pos > buffer.size() / 2) {  // 已处理超过一半时收缩
      buffer = buffer.substr(pos);
      pos = 0;
    }
  }

 public:
  ExtensibleLexer() = default;

  // 追加输入数据
  void feed(std::string_view data) { buffer.append(data); }

  // 注册匹配器（按注册顺序决定优先级，早注册=高优先级）
  void register_matcher(TokenMatcher matcher) {
    matchers.emplace_back(std::move(matcher));
  }

  // 获取下一个Token（nullopt表示需要更多输入）
  auto next_token() -> std::optional<Token> {
    skip_whitespace();  // 跳过空白
    shrink_buffer();    // 收缩缓冲区

    if (pos >= buffer.size()) {
      return std::nullopt;  // 缓冲区空，需要更多输入
    }

    // 按优先级匹配（早注册的匹配器先执行）
    for (const auto& matcher : matchers) {
      size_t original_pos = pos;
      if (auto token = matcher(buffer, pos, line, column)) {
        return token;
      }
      pos = original_pos;  // 匹配失败，恢复位置
    }

    // 未匹配到任何规则：生成InvalidToken
    auto codepoint_len = decode_utf8(buffer, pos);
    if (!codepoint_len) {
      // UTF-8错误（优先级0）
      Token err_token{
        .type = TokenType::Utf8Error,
        .value = last_utf8_error.value(),
        .line = line,
        .column = column
      };
      pos++;  // 跳过错误字节
      column++;
      return err_token;
    }

    auto [code_point, len] = codepoint_len.value();
    Token invalid_token{
      .type = TokenType::InvalidToken,
      .value = std::string_view(&buffer[pos], len),
      .line = line,
      .column = column
    };
    pos += len;
    column++;
    return invalid_token;
  }

  // 判断是否处理完所有输入
  [[nodiscard]] auto is_eof() const -> bool {
    return pos >= buffer.size() && buffer.empty();
  }

  // 获取当前行列号
  [[nodiscard]] auto get_position() const -> std::pair<size_t, size_t> {
    return {line, column};
  }
};

// -------------------------- 常用Token匹配器（外部注册用）
// --------------------------
namespace Matchers {
// 1. 布尔值匹配器（true/false）
inline auto boolean_matcher() -> TokenMatcher {
  return [](
           std::string_view input, size_t& pos, size_t& line, size_t& column
         ) -> std::optional<Token> {
    const std::vector<std::string_view> bools = {"true", "false"};
    for (const auto& b : bools) {
      if (pos + b.size() > input.size()) {
        continue;
      }
      if (input.substr(pos, b.size()) != b) {
        continue;
      }

      // 确保后接非标识符字符（避免"truex"误判）
      if (pos + b.size() < input.size()) {
        auto next_code_point =
          Utf8Utils::get_utf8_codepoint(input, pos + b.size());
        if (next_code_point.is_ok() &&
            Utf8Utils::is_identifier_part(next_code_point.unwrap().first)) {
          continue;
        }
      }

      // 匹配成功
      size_t start_col = column;
      pos += b.size();
      column += b.size();  // ASCII字符，1字符=1码点
      return Token{
        .type = TokenType::Boolean,
        .value = b,
        .line = line,
        .column = start_col
      };
    }
    return std::nullopt;
  };
}

// 2. Null匹配器（null）
inline auto null_matcher() -> TokenMatcher {
  return [](
           std::string_view input, size_t& pos, size_t& line, size_t& column
         ) -> std::optional<Token> {
    const std::string_view null_str = "null";
    if (pos + null_str.size() > input.size())
      return std::nullopt;
    if (input.substr(pos, null_str.size()) != null_str)
      return std::nullopt;

    // 避免"nullptr"误判
    if (pos + null_str.size() < input.size()) {
      auto next_code_point =
        Utf8Utils::get_utf8_codepoint(input, pos + null_str.size());
      if (next_code_point.is_ok() &&
          Utf8Utils::is_identifier_part(next_code_point.unwrap().first)) {
        return std::nullopt;
      }
    }

    size_t start_col = column;
    pos += null_str.size();
    column += null_str.size();
    return Token{
      .type = TokenType::Null,
      .value = null_str,
      .line = line,
      .column = start_col
    };
  };
}

// 3. 关键字匹配器（通用函数，接收关键字字符串）
inline auto keyword_matcher(std::string_view keyword) -> TokenMatcher {
  return [keyword](
           std::string_view input, size_t& pos, size_t& line, size_t& column
         ) -> std::optional<Token> {
    if (pos + keyword.size() > input.size())
      return std::nullopt;
    if (input.substr(pos, keyword.size()) != keyword)
      return std::nullopt;

    // 避免关键字作为标识符前缀（如"ifx"不应匹配"if"）
    if (pos + keyword.size() < input.size()) {
      auto next_code_point =
        Utf8Utils::get_utf8_codepoint(input, pos + keyword.size());
      if (next_code_point.is_ok() &&
          Utf8Utils::is_identifier_part(next_code_point.unwrap().first)) {
        return std::nullopt;
      }
    }

    size_t start_col = column;
    pos += keyword.size();
    column += keyword.size();
    return Token{
      .type = TokenType::Keyword,
      .value = keyword,
      .line = line,
      .column = start_col
    };
  };
}

// 4. 字符串匹配器（支持双引号，含转义符）
inline auto string_matcher() -> TokenMatcher {
  return [](
           std::string_view input, size_t& pos, size_t& line, size_t& column
         ) -> std::optional<Token> {
    if (pos >= input.size() || input[pos] != '"') {
      return std::nullopt;
    }

    size_t start_pos = pos;
    size_t start_col = column;
    pos++;
    column++;
    bool escaped = false;

    while (pos < input.size()) {
      if (escaped) {
        escaped = false;
        pos++;
        column++;
        continue;
      }
      if (input[pos] == '"') {  // 结束引号
        pos++;
        column++;
        return Token{
          .type = TokenType::String,
          .value = input.substr(start_pos + 1, pos - start_pos - 2),
          .line = line,
          .column = start_col
        };
      }
      if (input[pos] == '\\') {  // 转义符
        escaped = true;
      }
      // 处理换行（字符串内换行需更新行号）
      if (input[pos] == '\n') {
        line++;
        column = 1;
      } else {
        column++;
      }
      pos++;
    }

    // 未闭合的字符串（等待更多输入）
    return std::nullopt;
  };
}

// 5. 数字匹配器（严格区分整数和浮点数）
inline auto number_matcher() -> TokenMatcher {
  return [](
           std::string_view input, size_t& pos, size_t& line, size_t& column
         ) -> std::optional<Token> {
    size_t start_pos = pos;
    size_t start_col = column;
    bool has_dot = false;
    bool has_digits_before_dot = false;

    // 匹配整数部分（前导数字）
    if (pos < input.size() && isdigit(static_cast<unsigned char>(input[pos]))) {
      has_digits_before_dot = true;
      while (pos < input.size() &&
             isdigit(static_cast<unsigned char>(input[pos]))) {
        pos++;
        column++;
      }
    }

    // 匹配小数点（必须有前或后数字）
    if (pos < input.size() && input[pos] == '.') {
      // 检查后接数字（避免单独的"."）
      if (pos + 1 < input.size() &&
          isdigit(static_cast<unsigned char>(input[pos + 1]))) {
        has_dot = true;
        pos++;
        column++;
        // 匹配小数部分
        while (pos < input.size() &&
               isdigit(static_cast<unsigned char>(input[pos]))) {
          pos++;
          column++;
        }
      } else {
        // 单独的"."不是数字（可能是分隔符）
        return std::nullopt;
      }
    }

    // 无效情况：无数字或仅小数点
    if (pos == start_pos ||
        (has_dot && !has_digits_before_dot && pos == start_pos + 1)) {
      return std::nullopt;
    }

    // 区分整数和浮点数
    std::string_view num_str = input.substr(start_pos, pos - start_pos);
    if (has_dot) {
      return Token{
        .type = TokenType::Float,
        .value = std::stod(std::string(num_str)),
        .line = line,
        .column = start_col
      };
    }
    return Token{
      .type = TokenType::Integer,
      .value = static_cast<int64_t>(std::stoll(std::string(num_str))),
      .line = line,
      .column = start_col
    };
  };
}

// 6. 三字符运算符匹配器（如===、!==）
inline auto operator3_matcher(std::string_view oprt) -> TokenMatcher {
  return [oprt](
           std::string_view input, size_t& pos, size_t& line, size_t& column
         ) -> std::optional<Token> {
    if (oprt.size() != 3) {
      return std::nullopt;  // 确保是三字符
    }
    if (pos + 3 > input.size()) {
      return std::nullopt;
    }
    if (input.substr(pos, 3) != oprt) {
      return std::nullopt;
    }

    size_t start_col = column;
    pos += 3;
    column += 3;  // ASCII字符，1字符=1码点
    return Token{
      .type = TokenType::Operator3,
      .value = oprt,
      .line = line,
      .column = start_col
    };
  };
}

// 7. 二字符运算符匹配器（如>=、<=、==）
inline auto operator2_matcher(std::string_view oprt) -> TokenMatcher {
  return [oprt](
           std::string_view input, size_t& pos, size_t& line, size_t& column
         ) -> std::optional<Token> {
    if (oprt.size() != 2) {
      return std::nullopt;  // 确保是二字符
    }
    if (pos + 2 > input.size()) {
      return std::nullopt;
    }
    if (input.substr(pos, 2) != oprt) {
      return std::nullopt;
    }

    size_t start_col = column;
    pos += 2;
    column += 2;
    return Token{
      .type = TokenType::Operator2,
      .value = oprt,
      .line = line,
      .column = start_col
    };
  };
}

// 8. 单字符运算符匹配器（如+、-、>、<）
inline auto operator1_matcher(char oprt) -> TokenMatcher {
  return [oprt](
           std::string_view input, size_t& pos, size_t& line, size_t& column
         ) -> std::optional<Token> {
    if (pos >= input.size() || input[pos] != oprt) {
      return std::nullopt;
    }

    size_t start_col = column;
    pos++;
    column++;
    return Token{
      .type = TokenType::Operator1,
      .value = std::string_view(&input[pos - 1], 1),
      .line = line,
      .column = start_col
    };
  };
}

// 9. 标识符匹配器（Unicode规则）
inline auto identifier_matcher() -> TokenMatcher {
  return [](
           std::string_view input, size_t& pos, size_t& line, size_t& column
         ) -> std::optional<Token> {
    size_t start_pos = pos;
    size_t start_col = column;

    // 检查首字符（必须是标识符起始字符）
    auto first_code_point = Utf8Utils::get_utf8_codepoint(input, pos);
    if (first_code_point.is_err() ||
        !Utf8Utils::is_identifier_start(first_code_point.unwrap().first)) {
      return std::nullopt;
    }
    size_t first_len = first_code_point.unwrap().second;
    pos += first_len;
    column++;  // 1个码点=1列

    // 匹配后续字符（标识符部分）
    while (pos < input.size()) {
      auto next_code_point = Utf8Utils::get_utf8_codepoint(input, pos);
      if (next_code_point.is_err() ||
          !Utf8Utils::is_identifier_part(next_code_point.unwrap().first)) {
        break;
      }
      pos += next_code_point.unwrap().second;
      column++;
    }

    return Token{
      .type = TokenType::Identifier,
      .value = input.substr(start_pos, pos - start_pos),
      .line = line,
      .column = start_col
    };
  };
}

// 11. 注释匹配器（单行//和多行/* */）
inline auto line_comment_matcher() -> TokenMatcher {
  return [](
           std::string_view input, size_t& pos, size_t& line, size_t& column
         ) -> std::optional<Token> {
    if (pos + 1 >= input.size() || input[pos] != '/' || input[pos + 1] != '/') {
      return std::nullopt;
    }
    // 跳过注释内容（到行尾）
    pos += 2;
    column += 2;
    while (pos < input.size() && input[pos] != '\n') {
      auto code_point = Utf8Utils::get_utf8_codepoint(input, pos);
      if (code_point.is_ok()) {
        pos += code_point.unwrap().second;
      } else {
        pos++;
      }
      column++;
    }
    // 处理换行
    if (pos < input.size() && input[pos] == '\n') {
      line++;
      column = 1;
      pos++;
    }
    return std::nullopt;  // 注释不生成Token
  };
}

inline auto block_comment_matcher() -> TokenMatcher {
  return [](
           std::string_view input, size_t& pos, size_t& line, size_t& column
         ) -> std::optional<Token> {
    if (pos + 1 >= input.size() || input[pos] != '/' || input[pos + 1] != '*') {
      return std::nullopt;
    }
    pos += 2;
    column += 2;

    // 跳过到*/
    while (pos + 1 < input.size()) {
      if (input[pos] == '*' && input[pos + 1] == '/') {
        pos += 2;
        column += 2;
        return std::nullopt;
      }
      if (input[pos] == '\n') {
        line++;
        column = 1;
        pos++;
      } else {
        auto code_point = Utf8Utils::get_utf8_codepoint(input, pos);
        if (code_point.is_ok()) {
          pos += code_point.unwrap().second;
        } else {
          pos++;
        }
        column++;
      }
    }
    return std::nullopt;  // 未闭合的注释，等待更多输入
  };
}
}  // namespace Matchers
   //
void register_default_matchers(Lexer::ExtensibleLexer& lexer) {
  // 最高优先级：注释
  lexer.register_matcher(Lexer::Matchers::line_comment_matcher());
  lexer.register_matcher(Lexer::Matchers::block_comment_matcher());

  // 高优先级：常量和关键字
  lexer.register_matcher(Lexer::Matchers::boolean_matcher());
  lexer.register_matcher(Lexer::Matchers::null_matcher());

  // 关键字列表
  const std::vector<std::string_view> keywords = {
    "if",       "else",     "for",    "while", "return",
    "function", "var",      "let",    "const", "fn",
    "template", "typename", "friend", "auto",  "throw"
  };
  for (const auto& keyword : keywords) {
    lexer.register_matcher(Lexer::Matchers::keyword_matcher(keyword));
  }

  // 中高优先级：字符串和数字
  lexer.register_matcher(Lexer::Matchers::string_matcher());
  lexer.register_matcher(Lexer::Matchers::number_matcher());

  // 运算符（按长度优先级）
  const std::vector<std::string_view> op3_list = {"===", "!=="};
  for (const auto& oprt : op3_list) {
    lexer.register_matcher(Lexer::Matchers::operator3_matcher(oprt));
  }
  const std::vector<std::string_view> op2_list = {">=", "<=", "==", "!=", "&&",
                                                  "||", "++", "--", "+=", "-=",
                                                  "*=", "/=", "->", "::"};
  for (const auto& oprt : op2_list) {
    lexer.register_matcher(Lexer::Matchers::operator2_matcher(oprt));
  }
  const std::vector<char> op1_list = {'+', '-', '*', '/', '%', '>', '<', '!',
                                      '&', '|', '^', '~', '?', '.', ',', '(',
                                      ')', '[', ']', '{', '}', ';', ':', '='};
  for (char oprt : op1_list) {
    lexer.register_matcher(Lexer::Matchers::operator1_matcher(oprt));
  }

  // 较低优先级：标识符
  lexer.register_matcher(Lexer::Matchers::identifier_matcher());
}
}  // namespace Lexer