#pragma once

#include "Lexer/StateMachine.h"
#include "Lexer/StateMachineManager.h"
#include "Lexer/Token.h"
#include "Lexer/TokenType.h"
#include "Utils/Result.h"
#include "Utils/RingBuffer.h"
#include "Utils/Utf8.h"

#include <cstdint>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>

namespace Lexer {
using Utils::Err;
using Utils::Ok;
using Utils::Result;
using Utils::RingBuffer;
template <TokenTypeConstraint TokenType>
class Proto {
 private:
  // 坐标系统
  Coordinate current_token_start = {.line = 1, .column = 1};
  Coordinate cursor_coordinate = {.line = 1, .column = 1};

  std::unique_ptr<RingBuffer> ring_buffer;
  size_t current_token_length = 0;
  StateMachineManager<TokenType> manager;
  bool eof = false;  // 标志不会再读取输入
  std::string token_buffer;

  enum class BuildTokenBufferError : uint8_t {
    InsufficientBytes,  // 缓冲区中字节数不足
  };

  void read_token_buffer(size_t length) {
    if (!ring_buffer->is_size_at_least(length)) {
      throw std::runtime_error("Insufficient bytes in buffer to read token");
    }
    reset_token_buffer();
    token_buffer.resize(length);
    for (size_t i = 0; i < length; ++i) {
      token_buffer[i] = ring_buffer->try_pop().value();
    }
  }

  auto watch_token_buffer(size_t length, size_t offset)
    -> Result<void, BuildTokenBufferError> {
    if (!ring_buffer->is_size_at_least(length + offset)) {
      return Err(BuildTokenBufferError::InsufficientBytes);
    }
    reset_token_buffer();
    token_buffer.resize(length);
    for (size_t i = 0; i < length; ++i) {
      token_buffer[i] = ring_buffer->try_peek(i + offset).value();
    }
    return Ok();
  }

  void reset_token_buffer() { token_buffer.clear(); }

  // 重置当前 Token 的状态（长度、状态机、起始坐标）
  void reset_token_state() {
    current_token_length = 0;
    manager.reset();
    current_token_start = cursor_coordinate;
  }

  void update_cursor_after_token() {
    cursor_coordinate.column += current_token_length;
    reset_token_state();
  }

  auto handle_utf8_error() -> Token<TokenType> {
    auto maybe_byte = ring_buffer->try_pop();
    if (!maybe_byte) {
      throw std::runtime_error("Lexer::handle_utf8_error ring_buffer is empty");
    }
    auto byte = maybe_byte.value();
    auto err_token = Token<TokenType>{
      .type = TokenType::Utf8Error,
      .value = std::string(1, byte),
      .coordinate = current_token_start
    };

    cursor_coordinate.column++;
    reset_token_state();

    return err_token;
  }

  // EOF 时强制结算最后一个 token（即使有活跃状态机）
  auto finalize_last_token() -> std::optional<Token<TokenType>> {
    if (current_token_length == 0) {
      return std::nullopt;
    }

    auto [best_machine, _] = manager.select_best_match();

    read_token_buffer(current_token_length);
    Token<TokenType> token;

    if (auto machine = best_machine.lock()) {
      token = Token<TokenType>{
        .type = machine->get_token_type(),
        .value = token_buffer,
        .coordinate = current_token_start,
      };
    } else {
      token = Token<TokenType>{
        .type = TokenType::InvalidToken,
        .value = token_buffer,
        .coordinate = current_token_start,
      };
    }

    update_cursor_after_token();
    reset_token_state();

    return token;
  }

  void handle_newline() {
    cursor_coordinate.line++;
    cursor_coordinate.column = 1;
    reset_token_state();
    ring_buffer->pop();
  }

  void handle_whitespace() {
    cursor_coordinate.column++;
    reset_token_state();
    ring_buffer->pop();
  }

  void handle_tab() {
    cursor_coordinate.column += 4;
    reset_token_state();
    ring_buffer->pop();
  }

  enum class EatStatus : uint8_t { Continue, Stop, Eof, Wait };

  auto eat() -> Result<EatStatus, Utils::Utf8::Error> {
    auto maybe_leading_byte = ring_buffer->try_peek(current_token_length);
    if (!maybe_leading_byte) {
      return Ok(EatStatus::Wait);
    }
    auto leading_byte = maybe_leading_byte.value();
    auto maybe_code_point_len =
      Utils::Utf8::quick_get_utf8_byte_length(leading_byte);
    if (maybe_code_point_len.is_err()) {
      return Err(std::move(maybe_code_point_len.unwrap_err()));
    }
    auto code_point_len = maybe_code_point_len.unwrap();
    auto watch_token_buffer_result =
      watch_token_buffer(code_point_len, current_token_length);
    if (watch_token_buffer_result.is_err()) {
      return Ok(EatStatus::Eof);
    }
    auto code_point_wrapper = Utils::Utf8::get_utf8_codepoint(token_buffer, 0);
    if (code_point_wrapper.is_err()) {
      return Err(std::move(code_point_wrapper.unwrap_err()));
    }

    auto [code_point, len] = code_point_wrapper.unwrap();
    for (size_t i = 0; i < len; i++) {
      char byte = token_buffer[i];
      if (manager.process_event(byte)) {
        current_token_length++;
      } else {
        return Ok(EatStatus::Stop);
      }
    }
    return Ok(EatStatus::Continue);
  }

  auto build_utf8_error_token() -> Token<TokenType> {
    auto maybe_leading_byte = ring_buffer->try_pop();
    if (!maybe_leading_byte) {
      throw std::runtime_error("Cannot build UTF-8 error token");
    }
    auto leading_byte = maybe_leading_byte.value();
    auto token = Token<TokenType>{
      .type = TokenType::Utf8Error,
      .value = std::string(1, leading_byte),
      .coordinate = current_token_start,
    };

    cursor_coordinate.column++;
    reset_token_state();
    return token;
  }

  auto build_token() -> std::optional<Token<TokenType>> {
    auto [best_machine, _] = manager.select_best_match();
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
      read_token_buffer(current_token_length);
      auto token = Token<TokenType>{
        .type = token_type,
        .value = token_buffer,
        .coordinate = current_token_start,
      };
      update_cursor_after_token();
      return token;
    }
    throw std::runtime_error("Cannot build token");
    return std::nullopt;
  }

 public:
  explicit Proto(size_t buffer_size)
    : ring_buffer(std::make_unique<Utils::RingBuffer>(buffer_size)) {}

  void feed(std::string_view data) {
    if (data.empty()) {
      return;
    }
    if (eof) {
      throw std::runtime_error("Cannot feed data after EOF");
    }
    for (char c : data) {
      ring_buffer->push(c);
    }
  }

  void terminate() { eof = true; }

  void register_machine(std::unique_ptr<StateMachine<TokenType>> machine) {
    manager.add_machine(std::move(machine));
  }

  auto next_token() -> std::optional<Token<TokenType>> {
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
      if (eat_status == EatStatus::Eof) {
        return finalize_last_token();
      }
    }
    return finalize_last_token();
  }

  [[nodiscard]] auto end_of_input() const -> bool {
    return ring_buffer->is_empty();
  }
};

template <TokenTypeConstraint TokenType>
using Instance = std::shared_ptr<Lexer::Proto<TokenType>>;

}  // namespace Lexer