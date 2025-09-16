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
class StreamLexer {
 private:
  std::string buffer;  // 输入缓冲区
  size_t pos = 0;      // 当前字节位置(0-based)
  size_t line = 1;     // 当前行号(1-based)
  size_t column = 1;   // 当前列号(1-based)
  std::string token_buffer;
  StateMachineManager manager;

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

  void register_machine(std::unique_ptr<StateMachine> machine) {
    manager.add_machine(std::move(machine));
  }

  // 获取下一个Token（nullopt表示需要更多输入）
  auto next_token() -> std::optional<Token> {
    shrink_buffer();  // 收缩缓冲区

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
    std::optional<Token> token = std::nullopt;
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
          if (token_type != Lexer::TokenType::WhiteSpace) {
            token = Token{
              .type = token_type,
              .value = token_buffer,
              .line = line,
              .column = column
            };
          }
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

// -------------------------- 常用Token匹配器（外部注册用）
// --------------------------
namespace Machines {

// 辅助函数：创建"标识符"状态机
inline auto create_identifier_machine() -> std::unique_ptr<StateMachine> {
  auto machine = std::make_unique<StateMachine>(Lexer::TokenType::Identifier);

  // 状态定义：S0(初始) → S1(标识符中间状态，接受状态)
  StateMachine::StateId s0 = machine->get_current_state();  // 初始状态ID
  StateMachine::StateId s1 = machine->add_state(true);

  // 转移规则：
  // S0 → S1：输入是字母
  machine->add_transition(s0, s1, [](char c) {
    return Utils::Utf8::is_identifier_start(c) || c < 0;
  });

  // S1 → S1：输入是字母或数字（保持在接受状态）
  machine->add_transition(s1, s1, [](char c) {
    return Utils::Utf8::is_identifier_part(c) || c < 0;
  });

  return machine;
}

// 辅助函数：创建"整数"状态机
inline auto create_integer_machine() -> std::unique_ptr<StateMachine> {
  auto machine = std::make_unique<StateMachine>(Lexer::TokenType::Integer);

  // 状态定义：S0(初始) → S1(整数状态，接受状态)
  StateMachine::StateId s0 = machine->get_current_state();
  StateMachine::StateId s1 = machine->add_state(true);

  // 转移规则：
  // S0 → S1：输入是数字
  machine->add_transition(s0, s1, [](char c) {
    return Utils::Utf8::is_digit(c);
  });

  // S1 → S1：输入是数字（保持接受状态）
  machine->add_transition(s1, s1, [](char c) {
    return Utils::Utf8::is_digit(c);
  });

  return machine;
}

inline auto create_single_symbol_machine(char target)
  -> std::unique_ptr<StateMachine> {
  auto machine = std::make_unique<StateMachine>(Lexer::TokenType::Operator1);

  // 状态定义：S0(初始) → S1(加号状态，接受状态)
  StateMachine::StateId s0 = machine->get_current_state();
  StateMachine::StateId s1 = machine->add_state(true);

  // 转移规则：S0 → S1：输入是'+'
  machine->add_transition(s0, s1, [target](char c) { return c == target; });

  // 加号无后续转移（接受后再输入任何字符都会失败）
  return machine;
}

inline auto create_whitespace_machine() -> std::unique_ptr<StateMachine> {
  auto machine = std::make_unique<StateMachine>(Lexer::TokenType::WhiteSpace);

  // 状态定义：S0(初始) → S1(空格状态，接受状态)
  StateMachine::StateId s0 = machine->get_current_state();
  StateMachine::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) {
    return Utils::Utf8::is_unicode_whitespace(c);
  });
  return machine;
}

inline auto create_string_machine() -> std::unique_ptr<StateMachine> {
  auto machine = std::make_unique<StateMachine>(Lexer::TokenType::String);

  // 状态定义
  StateMachine::StateId s0 =
    machine->get_current_state();  // 初始状态：等待起始引号
  StateMachine::StateId s1 =
    machine->add_state(false);  // 双引号内容状态（已遇"）
  StateMachine::StateId s2 =
    machine->add_state(true);  // 双引号结束状态（接受状态）
  StateMachine::StateId s3 =
    machine->add_state(false);  // 单引号内容状态（已遇'）
  StateMachine::StateId s4 =
    machine->add_state(true);  // 单引号结束状态（接受状态）

  // 转移规则：严格保证引号匹配
  // 1. 初始状态 -> 双引号内容状态：遇到双引号"
  machine->add_transition(s0, s1, [](char c) { return c == '"'; });

  // 2. 双引号内容状态 -> 双引号结束状态：遇到双引号"（匹配结束）
  machine->add_transition(s1, s2, [](char c) { return c == '"'; });

  // 3. 双引号内容状态保持：接受除"之外的字符
  machine->add_transition(s1, s1, [](char c) {
    return c != '"';  // 不允许未结束的双引号内出现新的双引号
  });

  // 4. 初始状态 -> 单引号内容状态：遇到单引号'
  machine->add_transition(s0, s3, [](char c) { return c == '\''; });

  // 5. 单引号内容状态 -> 单引号结束状态：遇到单引号'（匹配结束）
  machine->add_transition(s3, s4, [](char c) { return c == '\''; });

  // 6. 单引号内容状态保持：接受除'之外的字符
  machine->add_transition(s3, s3, [](char c) {
    return c != '\'';  // 不允许未结束的单引号内出现新的单引号
  });

  return machine;
}

inline auto create_keyword_machine(std::string_view keyword)
  -> std::unique_ptr<StateMachine> {
  // 确保关键字不为空
  assert(!keyword.empty() && "关键字不能为空字符串");

  // 创建关键字状态机，Token类型为Keyword
  auto machine = std::make_unique<StateMachine>(Lexer::TokenType::Keyword);

  // 初始状态
  StateMachine::StateId current_state = machine->get_current_state();

  // 为关键字的每个字符创建对应的状态和转移规则
  for (size_t i = 0; i < keyword.size(); ++i) {
    // 转换为UTF-8字符（假设关键字是ASCII字符）
    auto current_char = keyword[i];

    // 最后一个字符对应的状态设为接受状态
    bool is_accepting = (i == keyword.size() - 1);
    StateMachine::StateId next_state = machine->add_state(is_accepting);

    // 添加状态转移：当前状态遇到指定字符时，转移到下一个状态
    machine->add_transition(
      current_state, next_state,
      [current_char](char input) { return input == current_char; }
    );

    // 移动到下一个状态
    current_state = next_state;
  }

  return machine;
}

}  // namespace Machines
}  // namespace Lexer