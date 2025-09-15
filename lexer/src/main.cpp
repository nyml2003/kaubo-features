#include <iostream>
#include "Lexer/Lexer.h"
#include "tools.h"

auto main() -> int {
  try {
    // 初始化Lexer并注册匹配规则

    Lexer::ExtensibleLexer lexer;
    register_default_matchers(lexer);
    auto file =
      read_file(R"(C:\Users\nyml\code\kaubo-features\lexer\src\test.txt)");
    lexer.feed(file);

    while (!lexer.is_eof()) {
      auto maybe_token = lexer.next_token();
      if (maybe_token.has_value()) {
        auto token = maybe_token.value();
        std::cout << std::to_string(token) << '\n';
      }
    }

  } catch (const std::exception& e) {
    std::cerr << "\n错误: " << e.what() << '\n';
    return 1;
  }

  return 0;
}