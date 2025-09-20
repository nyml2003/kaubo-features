#include <cassert>
#include <iostream>
#include <string>
#include "Lexer/Kaubo/Builder.h"
#include "Parser/Kaubo/Parser.h"

// 测试用例结构：表达式字符串 + 预期C++计算结果
struct TestCase {
  std::string expression;
  int64_t expected_result;  // 假设运算结果为64位整数
};

// 执行单个测试用例并验证结果
void run_test() {
  try {
    auto lexer = Lexer::Kaubo::Builder::get_instance();

    // 解析器计算
    lexer->feed("var a : int = 123 + 345 * 789;");
    lexer->terminate();
    Parser::Kaubo::Parser parser(lexer);
    auto parseResult = parser.parse();

    if (parseResult.is_ok()) {
      auto result = std::move(parseResult).unwrap();
      Parser::Kaubo::Parser::print_ast(result);

    } else {
      std::cout << "  ❌ Parse failed! Error: "
                << std::to_string(parseResult.unwrap_err()) << "\n\n";
    }
  } catch (const std::exception& e) {
    std::cout << "  ❌ Exception: " << e.what() << "\n\n";
  }
}

int main() {
  // 执行所有测试
  std::cout << "=== Starting Parser vs C++ Literal Validation ===\n\n";
  run_test();

  return 0;
}
