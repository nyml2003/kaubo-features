#include "Builder.h"
#include "Machines.h"

namespace Lexer {
auto Builder::build() -> Instance<TokenType> {
  auto lexer = std::make_unique<Lexer::Proto<TokenType>>(1024);

  // 注册关键字状态机
  lexer->register_machine(Machines::create_var_machine());
  lexer->register_machine(Machines::create_int_type_machine());

  // 注册运算符状态机
  lexer->register_machine(Machines::create_plus_machine());
  lexer->register_machine(Machines::create_minus_machine());
  lexer->register_machine(Machines::create_multiply_machine());
  lexer->register_machine(Machines::create_divide_machine());

  // 注册比较运算符状态机
  lexer->register_machine(Machines::create_equal_equal_machine());
  lexer->register_machine(Machines::create_not_equal_machine());
  lexer->register_machine(Machines::create_greater_machine());
  lexer->register_machine(Machines::create_less_machine());
  lexer->register_machine(Machines::create_greater_equal_machine());
  lexer->register_machine(Machines::create_less_equal_machine());

  // 注册标识符状态机
  lexer->register_machine(Machines::create_identifier_machine());

  // 注册标点符号状态机
  lexer->register_machine(Machines::create_colon_machine());
  lexer->register_machine(Machines::create_comma_machine());
  lexer->register_machine(Machines::create_equals_machine());
  lexer->register_machine(Machines::create_semicolon_machine());

  // 注册括号状态机
  lexer->register_machine(Machines::create_left_paren_machine());
  lexer->register_machine(Machines::create_right_paren_machine());
  lexer->register_machine(Machines::create_left_brace_machine());
  lexer->register_machine(Machines::create_right_brace_machine());

  // 注册整数状态机
  lexer->register_machine(Machines::create_integer_machine());

  // 注册空白字符状态机
  lexer->register_machine(Machines::create_whitespace_machine());
  lexer->register_machine(Machines::create_tab_machine());
  lexer->register_machine(Machines::create_newline_machine());

  return lexer;
}

}  // namespace Lexer