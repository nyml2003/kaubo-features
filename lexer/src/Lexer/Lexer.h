#pragma once
#include <cassert>
#include <iostream>
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
using Utils::Err;
using Utils::Ok;
using Utils::Result;
template <TokenTypeConstraint TokenType>
class StreamLexer {
 private:
  // 坐标系统
  Coordinate current_token_start = {.line = 1, .column = 1};
  Coordinate cursor_coordinate = {.line = 1, .column = 1};

  std::string buffer;
  size_t pos = 0;
  std::string token_buffer;
  StateMachineManager<TokenType> manager;
  bool eof = false;  // 标志不会再读取输入

  void shrink_buffer() { return; }

  void update_cursor_after_token() {
    cursor_coordinate.column += token_buffer.size();
    current_token_start = cursor_coordinate;
    cursor_coordinate.column++;
    token_buffer.clear();
    manager.reset();
  }

  auto handle_utf8_error() -> Token<TokenType> {
    auto err_token = Token<TokenType>{
      .type = TokenType::Utf8Error,
      .value = std::string(1, buffer[pos]),
      .coordinate = current_token_start,
    };

    pos++;
    cursor_coordinate.column++;
    current_token_start = cursor_coordinate;
    token_buffer.clear();
    manager.reset();

    return err_token;
  }

  // EOF 时强制结算最后一个 token（即使有活跃状态机）
  auto finalize_last_token() -> std::optional<Token<TokenType>> {
    if (token_buffer.empty()) {
      return std::nullopt;
    }

    auto [best_machine, match_length] = manager.select_best_match();
    if (match_length == 0) {
      match_length = 1;  // 至少消费一个字符
    }

    std::string token_value = token_buffer.substr(0, match_length);
    Token<TokenType> token;

    if (auto machine = best_machine.lock()) {
      token = Token<TokenType>{
        .type = machine->get_token_type(),
        .value = token_value,
        .coordinate = current_token_start,
      };
    } else {
      token = Token<TokenType>{
        .type = TokenType::InvalidToken,
        .value = token_value,
        .coordinate = current_token_start,
      };
    }

    update_cursor_after_token();

    current_token_start = cursor_coordinate;
    token_buffer.erase(0, match_length);
    manager.reset();

    return token;
  }

  void handle_newline() {
    cursor_coordinate.line++;
    cursor_coordinate.column = 1;
    current_token_start = cursor_coordinate;
    token_buffer.clear();
    manager.reset();
  }

  void handle_whitespace() {
    cursor_coordinate.column++;
    current_token_start = cursor_coordinate;
    token_buffer.clear();
    manager.reset();
  }

  void handle_tab() {
    cursor_coordinate.column += 4;
    current_token_start = cursor_coordinate;
    token_buffer.clear();
    manager.reset();
  }

  enum class EatStatus : uint8_t { Continue, Stop, Eof };

  auto eat() -> Result<EatStatus, Utils::Utf8::Error> {
    auto code_point_wrapper = Utils::Utf8::get_utf8_codepoint(buffer, pos);
    if (code_point_wrapper.is_err()) {
      return Err(std::move(code_point_wrapper.unwrap_err()));
    }

    auto [code_point, len] = code_point_wrapper.unwrap();
    for (size_t i = 0; i < len; i++) {
      char byte = buffer[pos];
      if (manager.process_event(byte)) {
        pos++;
        token_buffer += byte;
      } else {
        return Ok(EatStatus::Stop);
      }
    }
    return Ok(EatStatus::Continue);
  }

  auto build_utf8_error_token() -> Token<TokenType> {
    auto token = Token<TokenType>{
      .type = TokenType::Utf8Error,
      .value = std::string(1, buffer[pos]),
      .coordinate = current_token_start,
    };

    pos++;
    cursor_coordinate.column++;
    current_token_start = cursor_coordinate;
    token_buffer.clear();
    manager.reset();
    return token;
  }

  auto build_token() -> std::optional<Token<TokenType>> {
    auto [best_machine, match_length] = manager.select_best_match();
    if (auto machine = best_machine.lock()) {
      auto token_type = machine->get_token_type();
      if (token_type == TokenType::WhiteSpace) {
        handle_whitespace();
        return next_token();
      }
      if (token_type == TokenType::NewLine) {
        handle_newline();
        return next_token();
      }
      if (token_type == TokenType::Tab) {
        handle_tab();
        return next_token();
      }
      auto token = Token<TokenType>{
        .type = token_type,
        .value = token_buffer,
        .coordinate = current_token_start,
      };
      update_cursor_after_token();
      return token;
    }
    assert(false && "No machine to build token");
    return std::nullopt;
  }

 public:
  StreamLexer() = default;

  void feed(std::string_view data) {
    if (data.empty()) {
      return;
    }
    if (eof) {
      throw std::runtime_error("Cannot feed data after EOF");
    }
    buffer.append(data);
  }

  void terminate() { eof = true; }

  void register_machine(std::unique_ptr<StateMachine<TokenType>> machine) {
    manager.add_machine(std::move(machine));
  }

  auto next_token() -> std::optional<Token<TokenType>> {
    shrink_buffer();

    bool at_end = end_of_input();
    if (at_end) {
      if (eof) {
        return finalize_last_token();
      }
      throw std::runtime_error("Cannot read after EOF");
    }

    while (!end_of_input()) {
      auto eat_result = eat();
      if (eat_result.is_err()) {
        return build_utf8_error_token();
      }
      auto eat_status = eat_result.unwrap();

      if (eat_status == EatStatus::Stop) {
        return build_token();
      }
      if (eat_status == EatStatus::Continue) {
        continue;
      }
    }
    return finalize_last_token();
  }

  [[nodiscard]] auto end_of_input() const -> bool {
    if (buffer.empty()) {
      return true;
    }
    if (pos >= buffer.size()) {
      return true;
    }
    return false;
  }
};

}  // namespace Lexer