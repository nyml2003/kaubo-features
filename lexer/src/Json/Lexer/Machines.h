#pragma once

#include "Lexer/StateMachine/Proto.h"
#include "TokenType.h"
#include "Utils/Utf8.h"

#include <memory>

namespace Json::Machines {
inline auto create_string_machine()
  -> std::unique_ptr<Lexer::StateMachine::Proto<TokenType>> {
  auto machine =
    std::make_unique<Lexer::StateMachine::Proto<TokenType>>(TokenType::String);

  // 状态定义
  Lexer::StateMachine::Proto<TokenType>::StateId s0 =
    machine->get_current_state();  // 初始状态：等待起始引号
  Lexer::StateMachine::Proto<TokenType>::StateId s1 =
    machine->add_state(false);  // 双引号内容状态（已遇"）
  Lexer::StateMachine::Proto<TokenType>::StateId s2 =
    machine->add_state(true);  // 双引号结束状态（接受状态）
  Lexer::StateMachine::Proto<TokenType>::StateId s3 =
    machine->add_state(false);  // 单引号内容状态（已遇'）
  Lexer::StateMachine::Proto<TokenType>::StateId s4 =
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

inline auto create_symbol_machine(TokenType tokenType, char val)
  -> std::unique_ptr<Lexer::StateMachine::Proto<TokenType>> {
  auto machine = std::make_unique<Lexer::StateMachine::Proto<TokenType>>(tokenType);

  // 状态定义
  Lexer::StateMachine::Proto<TokenType>::StateId s0 =
    machine->get_current_state();  // 初始状态：等待符号
  Lexer::StateMachine::Proto<TokenType>::StateId s1 =
    machine->add_state(true);  // 符号结束状态（接受状态）

  // 转移规则：严格保证符号匹配
  // 1. 初始状态 -> 符号结束状态：遇到符号
  machine->add_transition(s0, s1, [val](char c) { return c == val; });

  return machine;
}

inline auto create_integer_machine()
  -> std::unique_ptr<Lexer::StateMachine::Proto<TokenType>> {
  auto machine =
    std::make_unique<Lexer::StateMachine::Proto<TokenType>>(TokenType::Integer);

  // 状态定义：S0(初始) → S1(整数状态，接受状态)
  Lexer::StateMachine::Proto<TokenType>::StateId s0 = machine->get_current_state();
  Lexer::StateMachine::Proto<TokenType>::StateId s1 = machine->add_state(true);

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

inline auto create_keyword_machine(TokenType type, std::string_view keyword)
  -> std::unique_ptr<Lexer::StateMachine::Proto<TokenType>> {
  // 确保关键字不为空

  auto machine = std::make_unique<Lexer::StateMachine::Proto<TokenType>>(type);

  // 初始状态
  Lexer::StateMachine::Proto<TokenType>::StateId current_state =
    machine->get_current_state();

  // 为关键字的每个字符创建对应的状态和转移规则
  for (size_t i = 0; i < keyword.size(); ++i) {
    // 转换为UTF-8字符（假设关键字是ASCII字符）
    auto current_char = keyword[i];

    // 最后一个字符对应的状态设为接受状态
    bool is_accepting = (i == keyword.size() - 1);
    Lexer::StateMachine::Proto<TokenType>::StateId next_state =
      machine->add_state(is_accepting);

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

inline auto create_whitespace_machine()
  -> std::unique_ptr<Lexer::StateMachine::Proto<TokenType>> {
  auto machine =
    std::make_unique<Lexer::StateMachine::Proto<TokenType>>(TokenType::Whitespace);

  // 状态定义
  Lexer::StateMachine::Proto<TokenType>::StateId s0 =
    machine->get_current_state();  // 初始状态：等待符号
  Lexer::StateMachine::Proto<TokenType>::StateId s1 =
    machine->add_state(true);  // 符号结束状态（接受状态）

  // 转移规则：严格保证符号匹配
  // 1. 初始状态 -> 符号结束状态：遇到符号
  machine->add_transition(s0, s1, [](char c) { return c == ' '; });

  return machine;
}

inline auto create_tab_machine()
  -> std::unique_ptr<Lexer::StateMachine::Proto<TokenType>> {
  auto machine =
    std::make_unique<Lexer::StateMachine::Proto<TokenType>>(TokenType::Tab);

  // 状态定义
  Lexer::StateMachine::Proto<TokenType>::StateId s0 =
    machine->get_current_state();  // 初始状态：等待符号
  Lexer::StateMachine::Proto<TokenType>::StateId s1 =
    machine->add_state(true);  // 符号结束状态（接受状态）

  // 转移规则：严格保证符号匹配
  // 1. 初始状态 -> 符号结束状态：遇到符号
  machine->add_transition(s0, s1, [](char c) { return c == '\t'; });

  return machine;
}

inline auto create_newline_machine()
  -> std::unique_ptr<Lexer::StateMachine::Proto<TokenType>> {
  auto machine =
    std::make_unique<Lexer::StateMachine::Proto<TokenType>>(TokenType::NewLine);

  // 状态定义：
  // s0: 初始状态
  // s1: 临时状态（处理\r后等待\n）
  // s2: 接受状态（已识别换行）
  Lexer::StateMachine::Proto<TokenType>::StateId s0 = machine->get_current_state();
  Lexer::StateMachine::Proto<TokenType>::StateId s1 = machine->add_state(false);
  Lexer::StateMachine::Proto<TokenType>::StateId s2 = machine->add_state(true);

  // 转移规则：
  // 1. 直接识别\n（Unix换行）
  machine->add_transition(s0, s2, [](char c) { return c == '\n'; });

  // 2. 识别\r\n（Windows换行）：先\r到s1，再\n到s2
  machine->add_transition(s0, s1, [](char c) { return c == '\r'; });
  machine->add_transition(s1, s2, [](char c) { return c == '\n'; });

  return machine;
}
}  // namespace Json::Machines
