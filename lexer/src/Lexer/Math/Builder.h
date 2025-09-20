#pragma once

#include "Lexer/Builder.h"
#include "Lexer/Math/TokenType.h"

namespace Lexer::Math {
class Builder
  : public Lexer::Builder::IBuilder<Lexer::Math::TokenType, Builder> {
 public:
  auto build() -> std::shared_ptr<Lexer::Proto<TokenType>> override;
};

}  // namespace Lexer::Math