#include "Builder.h"
#include "Machines.h"

namespace Lexer::Kaubo {
auto Builder::build() -> std::shared_ptr<Lexer::Proto<TokenType>> {
  auto lexer = std::make_shared<Lexer::Proto<TokenType>>(1024);

  // 注册关键字状态机
  lexer->register_machine(Machines::create_var_machine());
  lexer->register_machine(Machines::create_int_type_machine());

  // 注册运算符状态机
  lexer->register_machine(Machines::create_plus_machine());
  lexer->register_machine(Machines::create_minus_machine());
  lexer->register_machine(Machines::create_multiply_machine());
  lexer->register_machine(Machines::create_divide_machine());

  // 注册标识符状态机
  lexer->register_machine(Machines::create_identifier_machine());

  // 注册标点符号状态机
  lexer->register_machine(Machines::create_colon_machine());
  lexer->register_machine(Machines::create_equals_machine());
  lexer->register_machine(Machines::create_semicolon_machine());

  // 注册括号状态机
  lexer->register_machine(Machines::create_left_paren_machine());
  lexer->register_machine(Machines::create_right_paren_machine());

  // 注册整数状态机
  lexer->register_machine(Machines::create_integer_machine());

  // 注册空白字符状态机
  lexer->register_machine(Machines::create_whitespace_machine());
  lexer->register_machine(Machines::create_tab_machine());
  lexer->register_machine(Machines::create_newline_machine());

  return lexer;
}

}  // namespace Lexer::Kaubo