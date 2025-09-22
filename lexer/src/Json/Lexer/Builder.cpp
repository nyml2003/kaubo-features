// #include "Builder.h"
// #include <memory>
// #include "Machines.h"

// namespace Lexer::Json {
// auto Builder::build() -> Instance<TokenType> {
//   auto lexer = std::make_unique<Lexer::Proto<TokenType>>(1024);
//   lexer->register_machine(Machines::create_integer_machine());
//   lexer->register_machine(
//     Machines::create_symbol_machine(TokenType::LeftBracket, '[')
//   );
//   lexer->register_machine(
//     Machines::create_symbol_machine(TokenType::RightBracket, ']')
//   );
//   lexer->register_machine(
//     Machines::create_symbol_machine(TokenType::LeftCurly, '{')
//   );
//   lexer->register_machine(
//     Machines::create_symbol_machine(TokenType::RightCurly, '}')
//   );
//   lexer->register_machine(
//     Machines::create_symbol_machine(TokenType::Comma, ',')
//   );
//   lexer->register_machine(
//     Machines::create_symbol_machine(TokenType::Colon, ':')
//   );
//   lexer->register_machine(
//     Machines::create_keyword_machine(TokenType::Bool, "true")
//   );
//   lexer->register_machine(
//     Machines::create_keyword_machine(TokenType::Bool, "false")
//   );
//   lexer->register_machine(
//     Machines::create_keyword_machine(TokenType::Null, "null")
//   );
//   lexer->register_machine(Machines::create_string_machine());
//   lexer->register_machine(Machines::create_whitespace_machine());
//   lexer->register_machine(Machines::create_tab_machine());
//   lexer->register_machine(Machines::create_newline_machine());
//   return std::move(lexer);
// }

// }  // namespace Lexer::Json