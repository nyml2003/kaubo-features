#pragma once
#include <cstdint>
#include <memory>
#include "Lexer/StateMachine.h"
#include "Utils/Utf8.h"
namespace Lexer::TokenType::Json {

enum TokenType : uint8_t {
  // 最高优先级：错误类型（优先处理错误）
  Utf8Error = 0,

  /*--- 高优先级：关键字类字面量 ---*/
  // 布尔值（true/false）
  Bool = 5,
  // 空值（null）
  Null = 6,

  /*--- 中优先级：基础数据类型字面量 ---*/
  // 字符串字面量（"xxx"）
  String = 10,
  // 整数字面量（123）
  Integer = 11,

  /*--- 较低优先级：结构符号（单字符分隔符/括号） ---*/
  // 左方括号 [
  LeftBracket = 20,
  // 右方括号 ]
  RightBracket = 21,
  // 左花括号 {
  LeftCurly = 22,
  // 右花括号 }
  RightCurly = 23,
  // 冒号 :
  Colon = 24,
  // 逗号 ,
  Comma = 25,

  // 空格（ ）、制表符（\t）
  WhiteSpace = 30,
  Tab = 31,
  // 换行符（\n、\r\n）
  NewLine = 32,

  InvalidToken = 255,
};

inline auto to_string(TokenType type) -> std::string {
  switch (type) {
    case Utf8Error:
      return "Utf8Error";
    case Bool:
      return "Bool";
    case Null:
      return "Null";
    case String:
      return "String";
    case Integer:
      return "Integer";
    case LeftBracket:
      return "[";
    case RightBracket:
      return "]";
    case LeftCurly:
      return "{";
    case RightCurly:
      return "}";
    case Colon:
      return ":";
    case Comma:
      return ",";
    case InvalidToken:
      return "InvalidToken";
    case WhiteSpace:
      return "Whitespace";
    case NewLine:
      return "Newline";
    case Tab:
      return "Tab";
    default:
      assert(false);
  }
}

inline auto create_string_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine = std::make_unique<StateMachine<TokenType>>(TokenType::String);

  // 状态定义
  StateMachine<TokenType>::StateId s0 =
    machine->get_current_state();  // 初始状态：等待起始引号
  StateMachine<TokenType>::StateId s1 =
    machine->add_state(false);  // 双引号内容状态（已遇"）
  StateMachine<TokenType>::StateId s2 =
    machine->add_state(true);  // 双引号结束状态（接受状态）
  StateMachine<TokenType>::StateId s3 =
    machine->add_state(false);  // 单引号内容状态（已遇'）
  StateMachine<TokenType>::StateId s4 =
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
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine = std::make_unique<StateMachine<TokenType>>(tokenType);

  // 状态定义
  StateMachine<TokenType>::StateId s0 =
    machine->get_current_state();  // 初始状态：等待符号
  StateMachine<TokenType>::StateId s1 =
    machine->add_state(true);  // 符号结束状态（接受状态）

  // 转移规则：严格保证符号匹配
  // 1. 初始状态 -> 符号结束状态：遇到符号
  machine->add_transition(s0, s1, [val](char c) { return c == val; });

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

inline auto create_keyword_machine(TokenType type, std::string_view keyword)
  -> std::unique_ptr<StateMachine<TokenType>> {
  // 确保关键字不为空
  assert(!keyword.empty() && "关键字不能为空字符串");

  auto machine = std::make_unique<StateMachine<TokenType>>(type);

  // 初始状态
  StateMachine<TokenType>::StateId current_state = machine->get_current_state();

  // 为关键字的每个字符创建对应的状态和转移规则
  for (size_t i = 0; i < keyword.size(); ++i) {
    // 转换为UTF-8字符（假设关键字是ASCII字符）
    auto current_char = keyword[i];

    // 最后一个字符对应的状态设为接受状态
    bool is_accepting = (i == keyword.size() - 1);
    StateMachine<TokenType>::StateId next_state =
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
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine =
    std::make_unique<StateMachine<TokenType>>(TokenType::WhiteSpace);

  // 状态定义
  StateMachine<TokenType>::StateId s0 =
    machine->get_current_state();  // 初始状态：等待符号
  StateMachine<TokenType>::StateId s1 =
    machine->add_state(true);  // 符号结束状态（接受状态）

  // 转移规则：严格保证符号匹配
  // 1. 初始状态 -> 符号结束状态：遇到符号
  machine->add_transition(s0, s1, [](char c) { return c == ' '; });

  return machine;
}

inline auto create_tab_machine() -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine = std::make_unique<StateMachine<TokenType>>(TokenType::Tab);

  // 状态定义
  StateMachine<TokenType>::StateId s0 =
    machine->get_current_state();  // 初始状态：等待符号
  StateMachine<TokenType>::StateId s1 =
    machine->add_state(true);  // 符号结束状态（接受状态）

  // 转移规则：严格保证符号匹配
  // 1. 初始状态 -> 符号结束状态：遇到符号
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

}  // namespace Lexer::TokenType::Json