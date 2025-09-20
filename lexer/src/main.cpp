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

    // 测试函数调用
    std::cout << "=== Testing Function Call ===\n";
    lexer->feed("func(1, 2 + 3, 4 * 5);");
    lexer->terminate();
    Parser::Kaubo::Parser parser1(lexer);
    auto parseResult1 = parser1.parse();

    if (parseResult1.is_ok()) {
      auto result = std::move(parseResult1).unwrap();
      Parser::Kaubo::Parser::print_ast(result);
    } else {
      std::cout << "  ❌ Function call parse failed! Error: "
                << std::to_string(parseResult1.unwrap_err()) << "\n\n";
    }

    // 测试多条语句
    std::cout << "\n=== Testing Multiple Statements ===\n";
    auto lexer2 = Lexer::Kaubo::Builder::get_instance();
    lexer2->feed("var a = 123; var b = 456; a + b;");
    lexer2->terminate();
    Parser::Kaubo::Parser parser2(lexer2);
    auto parseResult2 = parser2.parse();

    if (parseResult2.is_ok()) {
      auto result = std::move(parseResult2).unwrap();
      Parser::Kaubo::Parser::print_ast(result);
    } else {
      std::cout << "  ❌ Multiple statements parse failed! Error: "
                << std::to_string(parseResult2.unwrap_err()) << "\n\n";
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
