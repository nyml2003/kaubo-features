#include <iostream>
#include <optional>
#include "Lexer/Lexer.h"
#include "Lexer/Token/Json.h"

using Lexer::TokenType::Json::TokenType;

namespace {
void init(Lexer::StreamLexer<TokenType>& lexer) {
  lexer.register_machine(Lexer::TokenType::Json::create_integer_machine());
  lexer.register_machine(
    Lexer::TokenType::Json::create_symbol_machine(TokenType::LeftBracket, '[')
  );
  lexer.register_machine(
    Lexer::TokenType::Json::create_symbol_machine(TokenType::RightBracket, ']')
  );
  lexer.register_machine(
    Lexer::TokenType::Json::create_symbol_machine(TokenType::LeftCurly, '{')
  );
  lexer.register_machine(
    Lexer::TokenType::Json::create_symbol_machine(TokenType::RightCurly, '}')
  );
  lexer.register_machine(
    Lexer::TokenType::Json::create_symbol_machine(TokenType::Comma, ',')
  );
  lexer.register_machine(
    Lexer::TokenType::Json::create_symbol_machine(TokenType::Colon, ':')
  );
  lexer.register_machine(
    Lexer::TokenType::Json::create_keyword_machine(TokenType::Bool, "true")
  );
  lexer.register_machine(
    Lexer::TokenType::Json::create_keyword_machine(TokenType::Bool, "false")
  );
  lexer.register_machine(
    Lexer::TokenType::Json::create_keyword_machine(TokenType::Null, "null")
  );
  lexer.register_machine(Lexer::TokenType::Json::create_string_machine());
  lexer.register_machine(Lexer::TokenType::Json::create_whitespace_machine());
  lexer.register_machine(Lexer::TokenType::Json::create_tab_machine());
  lexer.register_machine(Lexer::TokenType::Json::create_newline_machine());
}

}  // namespace

using Lexer::TokenType::Json::TokenType;
auto main() -> int {
  Lexer::StreamLexer<TokenType> lexer(1024);
  init(lexer);
  lexer.feed(R"({ "a": 123})");
  lexer.terminate();

  while (!lexer.end_of_input()) {
    auto maybe_token = lexer.next_token();
    if (maybe_token == std::nullopt) {
      continue;
    }
    const auto& token = maybe_token.value();
    std::cout << std::to_string(token) << "\n";
  }

  return 0;
}