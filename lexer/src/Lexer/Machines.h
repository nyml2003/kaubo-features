#pragma once

#include "Lexer/StateMachine/Proto.h"
#include "Type.h"
#include "Utils/Utf8.h"

#include <memory>

namespace Lexer::Machines {
inline auto create_single_symbol_machine(char symbol, TokenType token_type)
  -> std::unique_ptr<StateMachine::Proto<TokenType>> {
  auto machine = std::make_unique<StateMachine::Proto<TokenType>>(token_type);

  StateMachine::Proto<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine::Proto<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [symbol](char c) { return c == symbol; });

  return machine;
}

inline auto create_integer_machine()
  -> std::unique_ptr<StateMachine::Proto<TokenType>> {
  auto machine =
    std::make_unique<StateMachine::Proto<TokenType>>(TokenType::Literal_Integer
    );

  // 状态定义：S0(初始) → S1(整数状态，接受状态)
  StateMachine::Proto<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine::Proto<TokenType>::StateId s1 = machine->add_state(true);

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
  -> std::unique_ptr<StateMachine::Proto<TokenType>> {
  auto machine =
    std::make_unique<StateMachine::Proto<TokenType>>(TokenType::Whitespace);

  StateMachine::Proto<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine::Proto<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == ' '; });

  return machine;
}

inline auto create_tab_machine()
  -> std::unique_ptr<StateMachine::Proto<TokenType>> {
  auto machine =
    std::make_unique<StateMachine::Proto<TokenType>>(TokenType::Tab);

  StateMachine::Proto<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine::Proto<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) { return c == '\t'; });

  return machine;
}

inline auto create_newline_machine()
  -> std::unique_ptr<StateMachine::Proto<TokenType>> {
  auto machine =
    std::make_unique<StateMachine::Proto<TokenType>>(TokenType::NewLine);

  // 状态定义：
  // s0: 初始状态
  // s1: 临时状态（处理\r后等待\n）
  // s2: 接受状态（已识别换行）
  StateMachine::Proto<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine::Proto<TokenType>::StateId s1 = machine->add_state(false);
  StateMachine::Proto<TokenType>::StateId s2 = machine->add_state(true);

  // 转移规则：
  // 1. 直接识别\n（Unix换行）
  machine->add_transition(s0, s2, [](char c) { return c == '\n'; });

  // 2. 识别\r\n（Windows换行）：先\r到s1，再\n到s2
  machine->add_transition(s0, s1, [](char c) { return c == '\r'; });
  machine->add_transition(s1, s2, [](char c) { return c == '\n'; });

  return machine;
}

inline auto create_keyword_machine(
  const std::string_view& keyword,
  TokenType token_type
) -> std::unique_ptr<StateMachine::Proto<TokenType>> {
  auto machine = std::make_unique<StateMachine::Proto<TokenType>>(token_type);
  StateMachine::Proto<TokenType>::StateId s0 = machine->get_current_state();
  for (size_t i = 0; i < keyword.size(); ++i) {
    bool is_accepting = i == keyword.size() - 1;
    StateMachine::Proto<TokenType>::StateId s =
      machine->add_state(is_accepting);
    machine->add_transition(s0, s, [keyword, i](char c) {
      return c == keyword[i];
    });
    s0 = s;
  }
  return machine;
}

inline auto create_identifier_machine()
  -> std::unique_ptr<StateMachine::Proto<TokenType>> {
  auto machine =
    std::make_unique<StateMachine::Proto<TokenType>>(TokenType::Identifier);

  StateMachine::Proto<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine::Proto<TokenType>::StateId s1 = machine->add_state(true);

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

inline auto create_double_symbol_machine(
  const std::string_view& symbols,
  TokenType token_type

) -> std::unique_ptr<StateMachine::Proto<TokenType>> {
  auto machine = std::make_unique<StateMachine::Proto<TokenType>>(token_type);

  StateMachine::Proto<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine::Proto<TokenType>::StateId s1 = machine->add_state(false);
  StateMachine::Proto<TokenType>::StateId s2 = machine->add_state(true);

  machine->add_transition(s0, s1, [symbols](char c) {
    return c == symbols[0];
  });
  machine->add_transition(s1, s2, [symbols](char c) {
    return c == symbols[1];
  });

  return machine;
}
inline auto create_string_machine()
  -> std::unique_ptr<StateMachine::Proto<TokenType>> {
  auto machine =
    std::make_unique<StateMachine::Proto<TokenType>>(TokenType::Literal_String);

  // 状态定义
  StateMachine::Proto<TokenType>::StateId s0 =
    machine->get_current_state();  // 初始状态：等待起始引号
  StateMachine::Proto<TokenType>::StateId s1 =
    machine->add_state(false);  // 双引号内容状态（已遇"）
  StateMachine::Proto<TokenType>::StateId s2 =
    machine->add_state(true);  // 双引号结束状态（接受状态）
  StateMachine::Proto<TokenType>::StateId s3 =
    machine->add_state(false);  // 单引号内容状态（已遇'）
  StateMachine::Proto<TokenType>::StateId s4 =
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

inline auto create_comment_machine()
  -> std::unique_ptr<StateMachine::Proto<TokenType>> {
  // 创建一个复合状态机，用于处理两种注释类型
  auto machine =
    std::make_unique<StateMachine::Proto<TokenType>>(TokenType::Comment);
  auto s0 = machine->get_current_state();  // 初始状态

  // 处理单行注释: // ...
  auto s1 = machine->add_state(false);  // 识别到第一个 '/'
  auto s2 = machine->add_state(true);   // 识别到第二个 '/'，进入单行注释状态

  // 处理多行注释: /* ... */
  auto s3 = machine->add_state(false);  // 识别到 '/' 后的 '*'
  auto s4 = machine->add_state(false);  // 多行注释内容状态
  auto s5 = machine->add_state(false);  // 多行注释中遇到 '*'
  auto s6 = machine->add_state(true);   // 多行注释结束 (识别到 '*/')

  // 初始状态转换: 遇到 '/' 进入 s1
  machine->add_transition(s0, s1, [](char c) { return c == '/'; });

  // 单行注释路径: s1 -> s2 (第二个 '/')
  machine->add_transition(s1, s2, [](char c) { return c == '/'; });

  // 单行注释中: 接受所有字符直到换行
  machine->add_transition(s2, s2, [](char c) {
    return c != '\n' && c != '\r';  // 不包含换行符
  });

  // 多行注释路径: s1 -> s3 (遇到 '*' 而不是第二个 '/')
  machine->add_transition(s1, s3, [](char c) { return c == '*'; });

  // 多行注释内容处理: s3 -> s4 (任意字符)
  machine->add_transition(s3, s4, [](char) { return true; });

  // 多行注释内容中: 大多数字符保持在s4，遇到 '*' 进入s5
  machine->add_transition(s4, s4, [](char c) { return c != '*'; });
  machine->add_transition(s4, s5, [](char c) { return c == '*'; });

  // 在s5状态(已遇到 '*'):
  // 遇到 '/' 则结束多行注释，进入接受状态s6
  machine->add_transition(s5, s6, [](char c) { return c == '/'; });
  // 遇到其他 '*' 保持在s5
  machine->add_transition(s5, s5, [](char c) { return c == '*'; });
  // 遇到其他字符回到s4继续寻找 '*'
  machine->add_transition(s5, s4, [](char c) { return c != '*' && c != '/'; });

  // 注释结束状态保持
  machine->add_transition(s6, s6, [](char) {
    return false;  // 一旦结束就不再接受字符
  });

  return machine;
}

}  // namespace Lexer::Machines