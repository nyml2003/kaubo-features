#pragma once

#include "Lexer/Core/Builder.h"
#include "Lexer/Core/Proto.h"
#include "Lexer/Type.h"

namespace Lexer {
class Builder : public IBuilder<Lexer::TokenType, Builder> {
 public:
  auto build() -> Instance<TokenType> override;
};

}  // namespace Lexer