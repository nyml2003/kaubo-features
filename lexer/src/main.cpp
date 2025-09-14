// #include <iostream>
// #include "Lexer/Lexer.h"
// #include "tools.h"

// auto main() -> int {
//   try {
//     // 初始化Lexer并注册匹配规则

//     Lexer::ExtensibleLexer lexer;
//     register_default_matchers(lexer);
//     auto file =
//       read_file(R"(C:\Users\nyml\code\kaubo-features\lexer\src\test.txt)");
//     lexer.feed(file);

//     while (!lexer.is_eof()) {
//       auto maybe_token = lexer.next_token();
//       if (maybe_token.has_value()) {
//         auto token = maybe_token.value();
//         std::cout << std::to_string(token) << '\n';
//       }
//     }

//   } catch (const std::exception& e) {
//     std::cerr << "\n错误: " << e.what() << '\n';
//     return 1;
//   }

//   return 0;
// }

#include <iostream>
#include <string>
#include "Result.h"

namespace {
inline auto divide(int a, int b) -> Result::Result<int, std::string> {
  if (b == 0) {
    return Result::Err<std::string>("除数不能为0");
  }
  return Result::Ok(a / b);
}

inline auto repeat(const std::string& str, int times)
  -> Result::Result<std::string, std::string> {
  if (times < 0) {
    return Result::Err<std::string>("重复次数不能为负数");
  }
  std::string temp;
  temp.reserve(str.size() * times);
  for (int i = 0; i < times; ++i) {
    temp += str;
  }
  return Result::Ok<std::string>(std::move(temp));
}

inline auto add(uint64_t a, uint64_t b)
  -> Result::Result<uint64_t, std::string> {
  if (a > std::numeric_limits<uint64_t>::max() - b) {
    return Result::Err<std::string>("溢出");
  }
  return Result::Ok(a + b);
}

inline auto mul(uint64_t a, uint64_t b)
  -> Result::Result<uint64_t, std::string> {
  if (a > std::numeric_limits<uint64_t>::max() / b) {
    return Result::Err<std::string>("溢出");
  }
  return Result::Ok(a * b);
}

}  // namespace

int main() {
  auto result = divide(10, 0);
  if (result.is_err()) {
    std::cout << "错误: " << result.unwrap_err() << '\n';
  }

  auto result2 = divide(10, 2);
  if (result2.is_ok()) {
    std::cout << "结果: " << result2.unwrap() << '\n';
  }
  auto result3 = repeat("abc", -1);
  if (result3.is_err()) {
    const std::string& str = result3.unwrap_err();
    std::cout << "错误: " << str << '\n';
  }
  auto result4 = repeat("abc", 20);
  if (result4.is_ok()) {
    const std::string& str = result4.unwrap();
    std::cout << "结果: " << str << '\n';
  }

  auto result5 = add(1ULL, 2ULL).map([=](uint64_t a) {
    return mul(a, static_cast<uint64_t>(static_cast<uint64_t>(1) << 63));
  });
  auto result5_flat = result5.flatten();
  if (result5_flat.is_ok()) {
    const uint64_t& a = result5_flat.unwrap();
    std::cout << "结果: " << a << '\n';
  } else {
    const std::string& str = result5_flat.unwrap_err();
    std::cout << "错误: " << str << '\n';
  }

  auto result6 = add(1ULL, 2ULL).and_then([=](uint64_t a) {
    return mul(a, static_cast<uint64_t>(static_cast<uint64_t>(1) << 63));
  });

  if (result6.is_ok()) {
    const uint64_t& a = result6.unwrap();
    std::cout << "结果: " << a << '\n';
  } else {
    const std::string& str = result6.unwrap_err();
    std::cout << "错误: " << str << '\n';
  }

  auto chain_result = add(1, 2).and_then([](auto s) { return add(s, 3); }
  ).and_then([](auto s) { return mul(s, 4); });

  if (chain_result.is_ok()) {
    const uint64_t& a = chain_result.unwrap();
    std::cout << "结果: " << a << '\n';
  } else {
    const std::string& str = chain_result.unwrap_err();
    std::cout << "错误: " << str << '\n';
  }

  return 0;
}
