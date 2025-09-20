#pragma once

#include "Lexer/Builder.h"
#include "Lexer/Kaubo/TokenType.h"

namespace Lexer::Kaubo {
class Builder
  : public Lexer::Builder::IBuilder<Lexer::Kaubo::TokenType, Builder> {
 public:
  auto build() -> std::shared_ptr<Lexer::Proto<TokenType>> override;
};

}  // namespace Lexer::Kaubo