#pragma once

#include "Lexer/Builder.h"
#include "Lexer/Kaubo/TokenType.h"
#include "Lexer/Lexer.h"

namespace Lexer::Kaubo {
class Builder
  : public Lexer::Builder::IBuilder<Lexer::Kaubo::TokenType, Builder> {
 public:
  auto build() -> Instance<TokenType> override;
};

}  // namespace Lexer::Kaubo