#pragma once

#include <iostream>
#include <memory>
#include "Lexer/Core/Proto.h"
namespace Lexer::Utils {

// 强行读取lexer的所有字符，打印出来
template <Token::Constraint TokenType>
inline auto print_all_tokens(const Proto<TokenType>& lexer) {
  while (!lexer->end_of_input()) {
    auto maybe_token = lexer->next_token();
    if (maybe_token) {
      const auto& token = maybe_token.value();
      std::cout << std::to_string(token) << "\n";
    } else {
      break;
    }
  }
}

template <Token::Constraint TokenType>
inline auto print_all_tokens(std::shared_ptr<Proto<TokenType>> lexer) {
  while (!lexer->end_of_input()) {
    auto maybe_token = lexer->next_token();
    if (maybe_token) {
      const auto& token = maybe_token.value();
      std::cout << std::to_string(token) << "\n";
    } else {
      break;
    }
  }
}

}  // namespace Lexer::Utils