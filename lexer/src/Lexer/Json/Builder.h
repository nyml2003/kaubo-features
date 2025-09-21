#pragma once

#include "Lexer/Builder.h"
#include "Lexer/Json/TokenType.h"
#include "Lexer/Lexer.h"

namespace Lexer::Json {
class Builder
  : public Lexer::Builder::IBuilder<Lexer::Json::TokenType, Builder> {
 public:
  auto build() -> Instance<TokenType> override;
};

}  // namespace Lexer::Json