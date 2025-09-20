#pragma once

#include "Lexer/Lexer.h"
namespace Lexer::Builder {

template <TokenTypeConstraint TokenType, typename Derived>
class IBuilder {
 public:
  virtual auto build() -> Instance<TokenType> = 0;
  virtual ~IBuilder() = default;
  explicit IBuilder() = default;
  IBuilder(const IBuilder&) = delete;
  IBuilder(IBuilder&&) = delete;
  auto operator=(const IBuilder&) -> IBuilder& = delete;
  auto operator=(IBuilder&&) -> IBuilder& = delete;
  static auto get_instance() -> Instance<TokenType> {
    static Derived instance;
    return instance.build();
  }
};

}  // namespace Lexer::Builder