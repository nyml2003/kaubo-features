#pragma once
#include <cassert>

#include <memory>
#include <optional>
#include <string>
#include <string_view>

#include "Lexer/StateMachine.h"
#include "Lexer/StateMachineManager.h"
#include "Lexer/Token.h"
#include "Lexer/TokenType.h"
#include "Utils/Result.h"
#include "Utils/Utf8.h"

namespace Lexer {
using Utils::Result;
template <TokenTypeConstraint TokenType>
class StreamLexer {
 private:
  std::string buffer;  // 输入缓冲区
  size_t pos = 0;      // 当前字节位置(0-based)
  size_t line = 1;     // 当前行号(1-based)
  size_t column = 1;   // 当前列号(1-based)
  std::string token_buffer;
  StateMachineManager<TokenType> manager;

  // 收缩缓冲区（避免内存膨胀）
  void shrink_buffer() {
    if (pos > buffer.size() / 2) {  // 已处理超过一半时收缩
      buffer = buffer.substr(pos);
      pos = 0;
    }
  }

 public:
  StreamLexer() = default;

  // 追加输入数据
  void feed(std::string_view data) { buffer.append(data); }

  void register_machine(std::unique_ptr<StateMachine<TokenType>> machine) {
    manager.add_machine(std::move(machine));
  }

  // 跳过空白字符
  void skip_whitespace() {
    while (pos < buffer.size()) {
      auto codepoint_len = Utils::Utf8::get_utf8_codepoint(buffer, pos);
      if (codepoint_len.is_err()) {
        pos++;
        column++;
        continue;
      }
      auto [code_point, len] = codepoint_len.unwrap();
      bool is_whitespace = false;
      if (Utils::Utf8::is_unicode_whitespace(code_point)) {
        pos += len;
        column += len;
        is_whitespace = true;
      }
      if (Utils::Utf8::is_unicode_newline(code_point)) {
        pos += len;
        line++;
        column = 1;
        is_whitespace = true;
      }
      if (!is_whitespace) {
        return;
      }
    }
  }

  // 获取下一个Token（nullopt表示需要更多输入）
  auto next_token() -> std::optional<Token<TokenType>> {
    shrink_buffer();  // 收缩缓冲区
    skip_whitespace();
    if (pos >= buffer.size()) {
      return std::nullopt;  // 缓冲区空，需要更多输入
    }

    // 未匹配到任何规则：生成InvalidToken
    auto codepoint_len = Utils::Utf8::get_utf8_codepoint(buffer, pos);
    if (codepoint_len.is_err()) {
      // UTF-8错误（优先级0）
      Token err_token{
        .type = TokenType::Utf8Error,
        .value = buffer[pos],
        .line = line,
        .column = column
      };
      pos++;
      column++;
      return err_token;
    }
    std::optional<Token<TokenType>> token = std::nullopt;
    auto [code_point, len] = codepoint_len.unwrap();
    for (size_t i = 0; i < len; i++) {
      char byte = buffer[pos];
      bool any_processed = manager.process_event(byte);

      if (any_processed) {
        pos++;
        token_buffer += byte;
      } else {
        auto [best_machine, match_length] = manager.select_best_match();
        if (auto machine = best_machine.lock()) {
          auto token_type = machine->get_token_type();

          token = Token<TokenType>{
            .type = token_type,
            .value = token_buffer,
            .line = line,
            .column = column
          };
          token_buffer.clear();

        } else {
          pos += len;
          token = Token{
            .type = TokenType::InvalidToken,
            .value = code_point,
            .line = line,
            .column = column
          };
        }
        manager.reset();
      }
    }

    return token;
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

}  // namespace Lexer