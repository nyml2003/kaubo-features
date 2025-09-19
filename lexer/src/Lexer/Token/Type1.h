#include <cstdint>
#include <memory>
#include "Lexer/StateMachine.h"
#include "Utils/Utf8.h"
namespace Lexer::TokenType::Type1 {

enum class TokenType : uint8_t {
  // 最高优先级：错误和特殊类型
  Utf8Error = 0,  // UTF-8解码错误（最高优先级，必须优先处理）

  /*---高优先级---*/
  /**
   * @brief 布尔值
   * - true
   * - false
   */
  Boolean = 5,

  /**
   * @brief 空值
   * null
   */
  Null = 6,

  /**
   * @brief 关键字
   */
  Keyword = 7,

  /**
   * @brief 字符串字面量
   * - 单引号：'hello'
   * - 双引号："hello"
   */
  String = 10,

  /**
   * @brief 整数字面量
   * - 十进制：123
   */
  Integer = 11,

  /**
   * @brief 浮点数字面量
   * - 十进制：123.456
   */
  Float = 12,

  /* ----运算符----*/
  /**
   * @brief 三字符运算符
   * - ===
   * - !==
   */
  Operator3 = 20,

  /**
   * @brief 二字符运算符
   * - >=
   * - <=
   * - ==
   * - !=
   * - &&
   * - ||
   * - ??
   */
  Operator2 = 21,

  /**
   * @brief 单字符运算符
   * ~!@#$%^&*()-+=[]{}|\\;:,.<>?/
   */
  Operator1 = 22,

  /* ----标识符----*/
  Identifier = 30,

  /*---最低优先级---*/
  Eof = 40,

  WhiteSpace = 41,  // 空白字符

  // 最低优先级：无效Token（最后匹配）
  InvalidToken = 50  // 合法UTF-8但无匹配规则
};
inline auto to_string(const TokenType& type) -> std::string {
  switch (type) {
    case TokenType::Utf8Error:
      return "Utf8Error";
    case TokenType::Boolean:
      return "Boolean";
    case TokenType::Null:
      return "Null";
    case TokenType::Keyword:
      return "Keyword";
    case TokenType::String:
      return "String";
    case TokenType::Integer:
      return "Integer";
    case TokenType::Float:
      return "Float";
    case TokenType::Operator3:
    case TokenType::Operator2:
    case TokenType::Operator1:
      return "Operator";
    case TokenType::Identifier:
      return "Identifier";
    case TokenType::InvalidToken:
      return "InvalidToken";
    case TokenType::Eof:
      return "Eof";
    case TokenType::WhiteSpace:
      return "WhiteSpace";
    default:
      assert(false && "未处理的TokenType");
  }
}
inline auto create_identifier_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine =
    std::make_unique<StateMachine<TokenType>>(TokenType::Identifier);

  // 状态定义：S0(初始) → S1(标识符中间状态，接受状态)
  StateMachine<TokenType>::StateId s0 =
    machine->get_current_state();  // 初始状态ID
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

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

inline auto create_single_symbol_machine(char target)
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine =
    std::make_unique<StateMachine<TokenType>>(TokenType::Operator1);

  // 状态定义：S0(初始) → S1(加号状态，接受状态)
  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  // 转移规则：S0 → S1：输入是'+'
  machine->add_transition(s0, s1, [target](char c) { return c == target; });

  // 加号无后续转移（接受后再输入任何字符都会失败）
  return machine;
}

inline auto create_whitespace_machine()
  -> std::unique_ptr<StateMachine<TokenType>> {
  auto machine =
    std::make_unique<StateMachine<TokenType>>(TokenType::WhiteSpace);

  // 状态定义：S0(初始) → S1(空格状态，接受状态)
  StateMachine<TokenType>::StateId s0 = machine->get_current_state();
  StateMachine<TokenType>::StateId s1 = machine->add_state(true);

  machine->add_transition(s0, s1, [](char c) {
    return Utils::Utf8::is_unicode_whitespace(c);
  });
  return machine;
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

inline auto create_keyword_machine(std::string_view keyword)
  -> std::unique_ptr<StateMachine<TokenType>> {
  // 确保关键字不为空
  assert(!keyword.empty() && "关键字不能为空字符串");

  // 创建关键字状态机，Token类型为Keyword
  auto machine = std::make_unique<StateMachine<TokenType>>(TokenType::Keyword);

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

}  // namespace Lexer::TokenType::Type1