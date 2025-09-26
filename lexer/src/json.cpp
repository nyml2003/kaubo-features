
#include <iostream>
#include <string>
#include "Json/Lexer/Builder.h"
#include "Json/Parser/Parser.h"
#include "Json/Parser/Utils.h"
#include "Json/Parser/Value.h"
#include "Utils/System.h"

namespace {
void run_test() {
  try {
    auto lexer = Json::Builder::get_instance();
    auto source = Utils::System::read_file(
      R"(C:\Users\nyml\code\kaubo-features\lexer\test\programs\a.json)"
    );
    lexer->feed(source);
    lexer->terminate();
    // Lexer::Utils::print_all_tokens(std::move(lexer));
    Json::Parser parser(std::move(lexer));
    auto json_result = parser.parse();
    if (json_result.is_err()) {
      std::cout << "  ❌ " << std::to_string(json_result.unwrap_err())
                << "\n\n";
    } else {
      std::cout << "  ✔️  " << json_result.unwrap()->to_string() << "\n\n";
      const auto& json = json_result.unwrap();
      json->set(
        "a", Json::Utils::create<Json::Value::Value>(
               Json::Utils::create(Json::Value::String("hello world"))
             )
      );
      std::cout << "  ✔️  " << json->to_string() << "\n\n";
      auto a_result = json->get("a");
      if (a_result.is_err()) {
        std::cout << "  ❌  " << a_result.unwrap_err() << "\n\n";
      } else {
        std::cout << "  ✔️  " << a_result.unwrap()->to_string() << "\n\n";
      }
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
