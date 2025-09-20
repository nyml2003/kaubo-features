#pragma once

#include "Lexer/StateMachine.h"
#include "TokenType.h"
#include "Utils/Utf8.h"

#include <memory>

namespace Lexer::Kaubo::Machines {

inline auto create_plus_machine() -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine = std::make_unique<StateMachine<TokenType>>(TokenType::Plus);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == '+'; });

  return machine;
}

inline auto create_minus_machine() -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine = std::make_unique<StateMachine<TokenType>>(TokenType::Minus);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == '-'; });

  return machine;
}

inline auto create_multiply_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine = std::make_unique<StateMachine<TokenType>>(TokenType::Multiply);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == '*'; });

  return machine;
}

inline auto create_divide_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine = std::make_unique<StateMachine<TokenType>>(TokenType::Divide);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == '/'; });

  return machine;
}

inline auto create_left_paren_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine =
    std::make_unique<StateMachine<TokenType>>(TokenType::LeftParen);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == '('; });

  return machine;
}

inline auto create_right_paren_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine =
    std::make_unique<StateMachine<TokenType>>(TokenType::RightParen);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == ')'; });

  return machine;
}

inline auto create_left_brace_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine =
    std::make_unique<StateMachine<TokenType>>(TokenType::LeftBrace);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == '{'; });

  return machine;
}

inline auto create_right_brace_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine =
    std::make_unique<StateMachine<TokenType>>(TokenType::RightBrace);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == '}'; });

  return machine;
}

inline auto create_integer_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine = std::make_unique<StateMachine<TokenType>>(TokenType::Integer);

  // 状态定义：S0(初始) → S1(整数状态，接受状态)
  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

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

inline auto create_whitespace_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine =
    std::make_unique<StateMachine<TokenType>>(TokenType::WhiteSpace);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == ' '; });

  return machine;
}

inline auto create_tab_machine() -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine = std::make_unique<StateMachine<TokenType>>(TokenType::Tab);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == '\t'; });

  return machine;
}

inline auto create_newline_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine = std::make_unique<StateMachine<TokenType>>(TokenType::NewLine);

  // 状态定义：
  // s0: 初始状态
  // s1: 临时状态（处理\r后等待\n）
  // s2: 接受状态（已识别换行）
  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(false);
  StateMachine<TokenType>::StateId s2 = machine->add_state(true);

  // 转移规则：
  // 1. 直接识别\n（Unix换行）
  machine->add_transition(s0, s2, [](char c) { return c == '\n'; });

  // 2. 识别\r\n（Windows换行）：先\r到s1，再\n到s2
  machine->add_transition(s0, s1, [](char c) { return c == '\r'; });
  machine->add_transition(s1, s2, [](char c) { return c == '\n'; });

  return machine;
}

inline auto create_var_machine() -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine = std::make_unique<StateMachine<TokenType>>(TokenType::Var);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(false);
  StateMachine<TokenType>::StateId s2 = machine->add_state(false);
  StateMachine<TokenType>::StateId s3 = machine->add_state(true);

  // v -> s1
  machine->add_transition(s0, s1, [](char c) { return c == 'v'; });
  // a -> s2
  machine->add_transition(s1, s2, [](char c) { return c == 'a'; });
  // r -> s3
  machine->add_transition(s2, s3, [](char c) { return c == 'r'; });

  return machine;
}

inline auto create_int_type_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine = std::make_unique<StateMachine<TokenType>>(TokenType::IntType);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(false);
  StateMachine<TokenType>::StateId s2 = machine->add_state(false);
  StateMachine<TokenType>::StateId s3 = machine->add_state(true);

  // i -> s1
  machine->add_transition(s0, s1, [](char c) { return c == 'i'; });
  // n -> s2
  machine->add_transition(s1, s2, [](char c) { return c == 'n'; });
  // t -> s3
  machine->add_transition(s2, s3, [](char c) { return c == 't'; });

  return machine;
}

inline auto create_identifier_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine =
    std::make_unique<StateMachine<TokenType>>(TokenType::Identifier);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  // 首字符必须是字母或下划线
  machine->add_transition(s0, s1, [](char c) {
    return Utils::Utf8::is_identifier_start(static_cast<char32_t>(c));
  });

  // 后续字符可以是字母、数字或下划线
  machine->add_transition(s1, s1, [](char c) {
    return Utils::Utf8::is_identifier_part(static_cast<char32_t>(c));
  });

  return machine;
}

inline auto create_colon_machine() -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine = std::make_unique<StateMachine<TokenType>>(TokenType::Colon);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == ':'; });

  return machine;
}

inline auto create_semicolon_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine =
    std::make_unique<StateMachine<TokenType>>(TokenType::Semicolon);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == ';'; });

  return machine;
}

inline auto create_comma_machine() -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine = std::make_unique<StateMachine<TokenType>>(TokenType::Comma);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == ','; });

  return machine;
}

inline auto create_equals_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine = std::make_unique<StateMachine<TokenType>>(TokenType::Equals);

  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == '='; });

  return machine;
}

}  // namespace Lexer::Kaubo::Machines