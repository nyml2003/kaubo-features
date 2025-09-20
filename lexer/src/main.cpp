#include <cassert>
#include <iostream>
#include <string>
#include "Lexer/Kaubo/Builder.h"
#include "Lexer/Utils.h"
#include "Parser/Kaubo/Parser.h"
#include "Utils/System.h"

// 测试用例结构：表达式字符串 + 预期C++计算结果
struct TestCase {
  std::string expression;
  int64_t expected_result;  // 假设运算结果为64位整数
};

// 执行单个测试用例并验证结果
void run_test() {
  try {
    auto lexer = Lexer::Kaubo::Builder::get_instance();
    auto source = Utils::System::read_file(
      R"(C:\Users\nyml\code\kaubo-features\lexer\test\programs\a.kaubo)"
    );
    lexer->feed(source);
    lexer->terminate();
    Parser::Kaubo::Parser parser1(lexer);
    auto parseResult1 = parser1.parse();

    if (parseResult1.is_ok()) {
      auto result = std::move(parseResult1).unwrap();
      Parser::Kaubo::print_ast(result);
    } else {
      std::cout << std::to_string(parseResult1.unwrap_err()) << "\n\n";
    }
    // Lexer::Utils::print_all_tokens(lexer);
  } catch (const std::exception& e) {
    std::cout << "  ❌ Exception: " << e.what() << "\n\n";
  }
}

auto main() -> int {
  run_test();

  return 0;
}
