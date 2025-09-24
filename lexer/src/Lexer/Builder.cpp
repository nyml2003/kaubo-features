#include "Builder.h"
#include "Machines.h"

namespace Lexer {
auto Builder::build() -> Instance<TokenType> {
  auto lexer = std::make_unique<Lexer::Proto<TokenType>>(1024);

  // 注册关键字状态机
  for (auto [keyword, type] :
       std::initializer_list<std::pair<std::string_view, TokenType>>{
         {"var", TokenType::Var},       {"if", TokenType::If},
         {"else", TokenType::Else},     {"elif", TokenType::Elif},
         {"while", TokenType::While},   {"for", TokenType::For},
         {"return", TokenType::Return}, {"in", TokenType::In},
         {"yield", TokenType::Yield},   {"true", TokenType::True},
         {"false", TokenType::False},   {"null", TokenType::Null},
         {"break", TokenType::Break},   {"continue", TokenType::Continue},
         {"struct", TokenType::Struct}, {"interface", TokenType::Interface},
         {"import", TokenType::Import}, {"as", TokenType::As},
         {"from", TokenType::From},     {"pass", TokenType::Pass},
         {"and", TokenType::And},       {"or", TokenType::Or},
         {"not", TokenType::Not},       {"async", TokenType::Async},
         {"await", TokenType::Await},
       }) {
    lexer->register_machine(Machines::create_keyword_machine(keyword, type));
  }

  // 注册字面量状态机
  lexer->register_machine(Machines::create_string_machine());
  lexer->register_machine(Machines::create_integer_machine());

  /*--- 双字符符号---*/
  for (auto [symbol, type] :
       std::initializer_list<std::pair<std::string_view, TokenType>>{
         {"==", TokenType::DoubleEqual},
         {"!=", TokenType::ExclamationEqual},
         {">=", TokenType::GreaterThanEqual},
         {"<=", TokenType::LessThanEqual},
       }) {
    lexer->register_machine(
      Machines::create_double_symbol_machine(symbol, type)
    );
  }

  /*--- 单字符符号（突出“单个字符”）---*/
  for (auto [symbol, type] : std::initializer_list<std::pair<char, TokenType>>{
         {'>', TokenType::GreaterThan},
         {'<', TokenType::LessThan},
         {'+', TokenType::Plus},
         {'-', TokenType::Minus},
         {'*', TokenType::Asterisk},
         {'/', TokenType::Slash},
         {':', TokenType::Colon},
         {'=', TokenType::Equal},
         {',', TokenType::Comma},
         {';', TokenType::Semicolon},
         {'(', TokenType::LeftParenthesis},
         {')', TokenType::RightParenthesis},
         {'{', TokenType::LeftCurlyBrace},
         {'}', TokenType::RightCurlyBrace},
         {'[', TokenType::LeftSquareBracket},
         {']', TokenType::RightSquareBracket},
         {'.', TokenType::Dot},
         {'|', TokenType::Pipe},
       }) {
    lexer->register_machine(
      Machines::create_single_symbol_machine(symbol, type)
    );
  }

  /*--- 标识符---*/
  lexer->register_machine(Machines::create_identifier_machine());

  /*--- 空白字符---*/
  lexer->register_machine(Machines::create_whitespace_machine());
  lexer->register_machine(Machines::create_comment_machine());
  lexer->register_machine(Machines::create_newline_machine());
  lexer->register_machine(Machines::create_tab_machine());

  return lexer;
}

}  // namespace Lexer