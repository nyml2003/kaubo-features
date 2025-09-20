#pragma once

#include "Lexer/Builder.h"
#include "Lexer/Json/TokenType.h"

namespace Lexer::Json {
class Builder
  : public Lexer::Builder::IBuilder<Lexer::Json::TokenType, Builder> {
 public:
  auto build() -> std::shared_ptr<Lexer::Proto<TokenType>> override;
};

}  // namespace Lexer::Json