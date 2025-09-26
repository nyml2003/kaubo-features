#include "Json/Parser/Parser.h"
#include "Utils.h"
#include "Utils/Result.h"
#include "Value.h"

namespace Json {

auto Parser::parse_value()  // NOLINT(misc-no-recursion)
  -> Result<ValuePtr, ParseError> {
  if (!current_token.has_value()) {
    return Err(ParseError::UnexpectedEndOfInput);
  }

  switch (current_token->type) {
    case TokenType::LeftCurly: {
      auto obj_result = parse_object();
      if (obj_result.is_err()) {
        return Err(obj_result.unwrap_err());
      }
      return Ok(obj_result.unwrap());
    }
    case TokenType::LeftBracket: {
      auto arr_result = parse_array();
      if (arr_result.is_err()) {
        return Err(arr_result.unwrap_err());
      }
      return Ok(arr_result.unwrap());
    }
    case TokenType::String: {
      auto str_result = parse_string();
      if (str_result.is_err()) {
        return Err(str_result.unwrap_err());
      }
      return Ok(str_result.unwrap());
    }
    case TokenType::Integer: {
      auto int_result = parse_number();
      if (int_result.is_err()) {
        return Err(int_result.unwrap_err());
      }
      return Ok(int_result.unwrap());
    }
    case TokenType::True: {
      auto bool_result = parse_true();
      if (bool_result.is_err()) {
        return Err(bool_result.unwrap_err());
      }
      return Ok(bool_result.unwrap());
    }
    case TokenType::False: {
      auto bool_result = parse_false();
      if (bool_result.is_err()) {
        return Err(bool_result.unwrap_err());
      }
      return Ok(bool_result.unwrap());
    }
    case TokenType::Null: {
      auto null_result = parse_null();
      if (null_result.is_err()) {
        return Err(null_result.unwrap_err());
      }
      return Ok(null_result.unwrap());
    }

    default:
      return Err(ParseError::UnexpectedToken);
  }
}

auto Parser::parse_object()  // NOLINT(misc-no-recursion)
  -> Result<ValuePtr, ParseError> {
  // 消耗左花括号
  auto err = expect(TokenType::LeftCurly);
  if (err.is_err()) {
    return Err(err.unwrap_err());
  }
  auto object = Utils::create(Json::Value::Object{});

  // 空对象
  if (check(TokenType::RightCurly)) {
    consume();
    return Ok(Utils::create<Json::Value::Value>(object));
  }

  // 解析键值对
  while (true) {
    // 解析键
    if (!check(TokenType::String)) {
      return Err(ParseError::UnexpectedToken);
    }

    auto key_value =
      current_token->value.substr(1, current_token->value.size() - 2);
    consume();  // 消耗字符串

    // 期望冒号
    auto err = expect(TokenType::Colon);
    if (err.is_err()) {
      return Err(err.unwrap_err());
    }

    // 解析值
    auto value_result = parse_value();
    if (value_result.is_err()) {
      return Err(value_result.unwrap_err());
    }

    auto value = value_result.unwrap();

    // 添加到对象
    object->value.emplace(key_value, std::move(value));

    // 检查是否有更多键值对
    if (match(TokenType::RightCurly)) {
      break;
    }

    if (!match(TokenType::Comma)) {
      return Err(ParseError::MissingCommaOrBracket);
    }
  }

  return Ok(Utils::create<Json::Value::Value>(object));
}

auto Parser::parse_array()  // NOLINT(misc-no-recursion)
  -> Result<ValuePtr, ParseError> {
  // 消耗左方括号
  auto err = expect(TokenType::LeftBracket);
  if (err.is_err()) {
    return Err(err.unwrap_err());
  }

  auto array = Utils::create(Json::Value::Array{});

  // 空数组
  if (check(TokenType::RightBracket)) {
    consume();
    return Ok(Utils::create<Json::Value::Value>(array));
  }

  // 解析数组元素
  while (true) {
    // 解析元素
    auto element_result = parse_value();
    if (element_result.is_err()) {
      return Err(element_result.unwrap_err());
    }

    auto element = element_result.unwrap();

    array->value.emplace_back(std::move(element));

    // 检查是否有更多元素
    if (match(TokenType::RightBracket)) {
      break;
    }

    if (!match(TokenType::Comma)) {
      return Err(ParseError::MissingCommaOrBracket);
    }
  }

  return Ok(Utils::create<Json::Value::Value>(array));
}

auto Parser::parse_string() -> Result<ValuePtr, ParseError> {
  if (!check(TokenType::String)) {
    return Err(ParseError::UnexpectedToken);
  }

  // 简化实现：直接返回字符串值（假设词法分析器已处理转义序列）
  std::string string_value =
    current_token->value.substr(1, current_token->value.size() - 2);
  consume();
  auto string_json_value = Utils::create<Json::Value::String>(string_value);

  return Ok(Utils::create<Json::Value::Value>(string_json_value));
}

auto Parser::parse_number() -> Result<ValuePtr, ParseError> {
  if (!check(TokenType::Integer)) {
    return Err(ParseError::UnexpectedToken);
  }

  const std::string& num_str = current_token->value;

  int64_t value = std::stoll(num_str);
  consume();
  return Ok(
    Utils::create<Json::Value::Value>(Utils::create(Json::Value::Number{value}))
  );
}

auto Parser::parse_true() -> Result<ValuePtr, ParseError> {
  if (!check(TokenType::True)) {
    return Err(ParseError::UnexpectedToken);
  }

  bool value = (current_token->value == "true");
  if (!value) {
    return Err(ParseError::UnexpectedToken);
  }
  consume();

  return Ok(
    Utils::create<Json::Value::Value>(Utils::create(Json::Value::True{}))
  );
}

auto Parser::parse_false() -> Result<ValuePtr, ParseError> {
  if (!check(TokenType::False)) {
    return Err(ParseError::UnexpectedToken);
  }

  bool value = (current_token->value == "false");
  if (!value) {
    return Err(ParseError::UnexpectedToken);
  }
  consume();

  return Ok(
    Utils::create<Json::Value::Value>(Utils::create(Json::Value::False{}))
  );
}

auto Parser::parse_null() -> Result<ValuePtr, ParseError> {
  if (!check(TokenType::Null)) {
    return Err(ParseError::UnexpectedToken);
  }

  bool value = (current_token->value == "null");
  if (!value) {
    return Err(ParseError::UnexpectedToken);
  }
  consume();

  return Ok(
    Utils::create<Json::Value::Value>(Utils::create(Json::Value::Null{}))
  );
}

}  // namespace Json
