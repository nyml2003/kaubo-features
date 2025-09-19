
#include <iostream>
#include <memory>
#include "Lexer/Lexer.h"
#include "Lexer/Token/Json.h"
#include "Parser/JsonParser.h"
#include "tools.h"

using Lexer::TokenType::Json::TokenType;
namespace {
void init(const std::shared_ptr<Lexer::StreamLexer<TokenType>>& lexer) {
  lexer->register_machine(Lexer::TokenType::Json::create_integer_machine());
  lexer->register_machine(
    Lexer::TokenType::Json::create_symbol_machine(TokenType::LeftBracket, '[')
  );
  lexer->register_machine(
    Lexer::TokenType::Json::create_symbol_machine(TokenType::RightBracket, ']')
  );
  lexer->register_machine(
    Lexer::TokenType::Json::create_symbol_machine(TokenType::LeftCurly, '{')
  );
  lexer->register_machine(
    Lexer::TokenType::Json::create_symbol_machine(TokenType::RightCurly, '}')
  );
  lexer->register_machine(
    Lexer::TokenType::Json::create_symbol_machine(TokenType::Comma, ',')
  );
  lexer->register_machine(
    Lexer::TokenType::Json::create_symbol_machine(TokenType::Colon, ':')
  );
  lexer->register_machine(
    Lexer::TokenType::Json::create_keyword_machine(TokenType::Bool, "true")
  );
  lexer->register_machine(
    Lexer::TokenType::Json::create_keyword_machine(TokenType::Bool, "false")
  );
  lexer->register_machine(
    Lexer::TokenType::Json::create_keyword_machine(TokenType::Null, "null")
  );
  lexer->register_machine(Lexer::TokenType::Json::create_string_machine());
  lexer->register_machine(Lexer::TokenType::Json::create_whitespace_machine());
  lexer->register_machine(Lexer::TokenType::Json::create_tab_machine());
  lexer->register_machine(Lexer::TokenType::Json::create_newline_machine());
}

}  // namespace

using Parser::JsonParser;
int main(int argc, char* argv[]) {
  // 检查是否提供了文件名参数
  if (argc < 2) {
    std::cerr << "请提供要读取的文件名作为参数！" << '\n';
    std::cerr << "用法: " << argv[0] << " <文件名>" << '\n';
    return 1;  // 返回非零值表示出错
  }
  // 使用命令行参数作为文件名
  std::string file = read_file(argv[1]);
  auto lexer = std::make_shared<Lexer::StreamLexer<TokenType>>(1024);
  init(lexer);
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