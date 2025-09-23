#include <cassert>
#include <iostream>
#include <string>
#include "Lexer/Builder.h"
#include "Lexer/Core/Utils.h"
#include "Parser/Parser.h"
#include "Utils/System.h"


// 测试用例结构：表达式字符串 + 预期C++计算结果
struct TestCase {
  std::string expression;
  int64_t expected_result;  // 假设运算结果为64位整数
};

namespace {
void run_test() {
  try {
    auto lexer = Lexer::Builder::get_instance();
    auto source = Utils::System::read_file(
      R"(C:\Users\nyml\code\kaubo-features\lexer\test\programs\a.kaubo)"
    );
    lexer->feed(source);
    lexer->terminate();
    //Lexer::Utils::print_all_tokens(std::move(lexer));
    Parser::Parser parser(std::move(lexer));
    auto parseResult1 = parser.parse();
    if (parseResult1.is_ok()) {
      Parser::print_ast(parseResult1.unwrap(), 0);
    }

  } catch (const std::exception& e) {
    std::cout << "  ❌ Exception: " << e.what() << "\n\n";
  }
}
}  // namespace
// 执行单个测试用例并验证结果

auto main() -> int {
  run_test();

  return 0;
}
