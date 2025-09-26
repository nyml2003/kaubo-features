#pragma once

#include "Error.h"
#include "Json/Lexer/TokenType.h"
#include "Json/Parser/Value.h"
#include "Lexer/Core/Proto.h"
#include "Utils/Result.h"

namespace Json {
using ::Utils::Err;
using ::Utils::Ok;
using ::Utils::Result;
using Value::ValuePtr;

class Parser {
 public:
  explicit Parser(Lexer::Instance<TokenType> lexer)
    : m_lexer(std::move(lexer)) {
    consume();  // 预读第一个token
  }

  auto parse() -> Result<ValuePtr, ParseError> { return parse_value(); }

 private:
  Lexer::Instance<TokenType> m_lexer;
  std::optional<Lexer::Token::Proto<TokenType>> current_token;

  // 消费当前token并读取下一个
  void consume() { current_token = m_lexer->next_token(); }

  // 检查当前token是否为指定类型
  [[nodiscard]] auto check(TokenType type) const -> bool {
    return current_token.has_value() && current_token->type == type;
  }

  // 检查并消费指定类型的token
  auto match(TokenType type) -> bool {
    if (check(type)) {
      consume();
      return true;
    }
    return false;
  }

  // 期望并消费指定类型的token，否则返回错误
  auto expect(TokenType type) -> Result<void, ParseError> {
    if (check(type)) {
      consume();
      return Ok();
    }
    return Err(ParseError::UnexpectedToken);
  }

  // 解析JSON值
  auto parse_value() -> Result<ValuePtr, ParseError>;

  // 解析JSON对象
  auto parse_object() -> Result<ValuePtr, ParseError>;

  // 解析JSON数组
  auto parse_array() -> Result<ValuePtr, ParseError>;

  // 解析JSON字符串
  auto parse_string() -> Result<ValuePtr, ParseError>;

  // 解析JSON数字
  auto parse_number() -> Result<ValuePtr, ParseError>;

  // 解析JSON布尔值
  auto parse_true() -> Result<ValuePtr, ParseError>;
  auto parse_false() -> Result<ValuePtr, ParseError>;

  // 解析JSON null
  auto parse_null() -> Result<ValuePtr, ParseError>;
};

}  // namespace Json
