
#include <iostream>
#include "Lexer/Json/Builder.h"
#include "Parser/JsonParser.h"
#include "tools.h"

using Parser::JsonParser;
auto main(int argc, char* argv[]) -> int {
  // 检查是否提供了文件名参数
  if (argc < 2) {
    std::cerr << "请提供要读取的文件名作为参数！" << '\n';
    std::cerr << "用法: " << argv[0] << " <文件名>" << '\n';
    return 1;  // 返回非零值表示出错
  }
  // 使用命令行参数作为文件名
  std::string file = read_file(argv[1]);
  auto lexer = Lexer::Json::Builder::get_instance();
  lexer->feed(file);
  lexer->terminate();
  JsonParser parser(lexer);
  auto parseResult = parser.parse();
  if (parseResult.is_err()) {
    std::cout << Parser::to_string(parseResult.unwrap_err()) << "\n";
  } else {
    std::cout << parseResult.unwrap().to_string() << "\n";
  }
  return 0;
}