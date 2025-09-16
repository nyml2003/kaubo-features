// #include <cctype>  // 用于isalpha、isdigit判断
// #include <format>
// #include <iostream>
// #include <memory>
// #include <string>
// #include "Lexer/TokenType.h"
// #include "StateMachine.h"
// #include "StateMachineManager.h"
// namespace {
// // 辅助函数：创建"标识符"状态机
// auto create_identifier_machine() -> std::unique_ptr<StateMachine> {
//   auto machine =
//   std::make_unique<StateMachine>(Lexer::TokenType::Identifier);

//   // 状态定义：S0(初始) → S1(标识符中间状态，接受状态)
//   StateMachine::StateId s0 = machine->get_current_state();  // 初始状态ID
//   StateMachine::StateId s1 = machine->add_state(
//     true,  // S1是接受状态（只要进入就表示匹配到标识符）
//     [](StateMachine::StateId /*id*/, char /**/) {

//     }
//   );

//   // 转移规则：
//   // S0 → S1：输入是字母
//   machine->add_transition(s0, s1, [](char c) {
//     return std::isalpha(static_cast<unsigned char>(c)) != 0;
//   });

//   // S1 → S1：输入是字母或数字（保持在接受状态）
//   machine->add_transition(s1, s1, [](char c) {
//     return std::isalpha(static_cast<unsigned char>(c)) != 0 ||
//            std::isdigit(static_cast<unsigned char>(c)) != 0;
//   });

//   return machine;
// }

// // 辅助函数：创建"整数"状态机
// auto create_integer_machine() -> std::unique_ptr<StateMachine> {
//   auto machine = std::make_unique<StateMachine>(Lexer::TokenType::Integer);

//   // 状态定义：S0(初始) → S1(整数状态，接受状态)
//   StateMachine::StateId s0 = machine->get_current_state();
//   StateMachine::StateId s1 = machine->add_state(
//     true,  // S1是接受状态
//     [](StateMachine::StateId /*id*/, char /**/) {

//     }
//   );

//   // 转移规则：
//   // S0 → S1：输入是数字
//   machine->add_transition(s0, s1, [](char c) {
//     return std::isdigit(static_cast<unsigned char>(c)) != 0;
//   });

//   // S1 → S1：输入是数字（保持接受状态）
//   machine->add_transition(s1, s1, [](char c) {
//     return std::isdigit(static_cast<unsigned char>(c)) != 0;
//   });

//   return machine;
// }

// auto create_single_symbol_machine(char target)
//   -> std::unique_ptr<StateMachine> {
//   auto machine = std::make_unique<StateMachine>(Lexer::TokenType::Operator1);

//   // 状态定义：S0(初始) → S1(加号状态，接受状态)
//   StateMachine::StateId s0 = machine->get_current_state();
//   StateMachine::StateId s1 = machine->add_state(
//     true,  // S1是接受状态（匹配单个'+'）
//     [](StateMachine::StateId /*id*/, char /**/) {

//     }
//   );

//   // 转移规则：S0 → S1：输入是'+'
//   machine->add_transition(s0, s1, [target](char c) { return c == target; });

//   // 加号无后续转移（接受后再输入任何字符都会失败）
//   return machine;
// }

// auto create_whitespace_machine() -> std::unique_ptr<StateMachine> {
//   auto machine =
//   std::make_unique<StateMachine>(Lexer::TokenType::WhiteSpace);

//   // 状态定义：S0(初始) → S1(空格状态，接受状态)
//   StateMachine::StateId s0 = machine->get_current_state();
//   StateMachine::StateId s1 = machine->add_state(
//     true,  // S1是接受状态（匹配单个'+'）
//     [](StateMachine::StateId /*id*/, char /**/) {

//     }
//   );

//   machine->add_transition(s0, s1, [](char c) { return c == ' '; });
//   return machine;
// }

// }  // namespace
// auto main() -> int {
//   // 1. 初始化状态机管理器
//   StateMachineManager manager;

//   // 2. 创建并添加3个词法规则对应的状态机
//   // 标识符：优先级1
//   manager.add_machine(create_identifier_machine());
//   // 整数：优先级1
//   manager.add_machine(create_integer_machine());
//   // 加号：优先级0
//   manager.add_machine(create_single_symbol_machine('+'));
//   // 减号：优先级0
//   manager.add_machine(create_single_symbol_machine('-'));
//   // 乘号：优先级0
//   manager.add_machine(create_single_symbol_machine('*'));
//   manager.add_machine(create_single_symbol_machine('='));
//   manager.add_machine(create_whitespace_machine());

//   // 3. 待处理的输入字符串（词法分析目标）
//   const std::string input = "var abc = 123 + 45";

//   // 4. 逐字符处理事件（核心逻辑）
//   size_t input_pos = 0;
//   while (input_pos < input.size()) {
//     char current_char = input[input_pos];

//     // 驱动所有活跃状态机处理当前字符
//     bool any_processed = manager.process_event(current_char);

//     if (any_processed) {
//       input_pos++;
//     } else {
//       auto [best_machine, match_length] = manager.select_best_match();

//       if (auto machine = best_machine.lock()) {
//         auto token_type = machine->get_token_type();
//         if (token_type != Lexer::TokenType::WhiteSpace) {
//           std::string token_value =
//             input.substr(input_pos - match_length, match_length);
//           std::cout << std::format(
//                          "  🎯 匹配结果: {} {}",
//                          std::to_string(machine->get_token_type()),
//                          token_value
//                        )
//                     << "\n";
//         }

//       } else {
//         // 无匹配结果（非法字符）
//         std::cout << "  ⚠️  无匹配规则，非法字符: " << current_char << "\n";
//         input_pos++;  // 跳过非法字符
//       }

//       manager.reset();
//     }
//   }

//   // 5. 处理输入结束后的剩余匹配（若有）
//   std::cout << "--- 输入处理完毕，检查剩余匹配 ---\n";
//   auto [final_best, final_match_len] = manager.select_best_match();
//   if (auto machine = final_best.lock()) {
//     std::string token_value =
//       input.substr(input.size() - final_match_len, final_match_len);
//     std::cout << std::format(
//       "  🎯 匹配结果: {} {}", std::to_string(machine->get_token_type()),
//       token_value
//     );
//   }
//   manager.reset();

//   return 0;
// }

#include <iostream>
#include <optional>
#include "Lexer/Lexer.h"
#include "Lexer/Token/Type1.h"
#include "tools.h"
auto main() -> int {
  Lexer::StreamLexer<Lexer::TokenType::Type1::TokenType> lexer;
  lexer.register_machine(Lexer::TokenType::Type1::create_integer_machine());
  lexer.register_machine(Lexer::TokenType::Type1::create_whitespace_machine());
  lexer.register_machine(
    Lexer::TokenType::Type1::create_single_symbol_machine('[')
  );
  lexer.register_machine(
    Lexer::TokenType::Type1::create_single_symbol_machine(']')
  );
  lexer.register_machine(
    Lexer::TokenType::Type1::create_single_symbol_machine('(')
  );
  lexer.register_machine(
    Lexer::TokenType::Type1::create_single_symbol_machine(')')
  );
  lexer.register_machine(
    Lexer::TokenType::Type1::create_single_symbol_machine('{')
  );
  lexer.register_machine(
    Lexer::TokenType::Type1::create_single_symbol_machine('}')
  );
  lexer.register_machine(
    Lexer::TokenType::Type1::create_single_symbol_machine(',')
  );
  lexer.register_machine(
    Lexer::TokenType::Type1::create_single_symbol_machine(':')
  );
  lexer.register_machine(
    Lexer::TokenType::Type1::create_keyword_machine("true")
  );
  lexer.register_machine(
    Lexer::TokenType::Type1::create_keyword_machine("false")
  );
  lexer.register_machine(
    Lexer::TokenType::Type1::create_keyword_machine("null")
  );
  lexer.register_machine(Lexer::TokenType::Type1::create_string_machine());

  std::string input =
    read_file(R"(C:\Users\nyml\code\kaubo-features\lexer\src\t.json)");
  lexer.feed(input + '\n');

  while (!lexer.is_eof()) {
    auto maybe_token = lexer.next_token();
    if (maybe_token == std::nullopt) {
      continue;
    }
    const auto& token = maybe_token.value();
    std::cout << std::to_string(token) << "\n";
  }
  return 0;
}