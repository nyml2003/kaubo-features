#pragma once

#include "Json/Lexer/TokenType.h"
#include "Lexer/Core/Builder.h"
#include "Lexer/Core/Proto.h"

namespace Json {
class Builder : public Lexer::IBuilder<Json::TokenType, Builder> {
 public:
  auto build() -> Lexer::Instance<TokenType> override;
};

}  // namespace Json