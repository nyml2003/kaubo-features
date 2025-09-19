#include "Parser/JsonParser.h"
#include <sstream>
#include <string>
#include "Utils/Overloaded.h"

namespace Parser {

using Utils::Err;
using Utils::Ok;

auto JsonValue::to_string() const -> std::string {
  return std::visit(
    overloaded{
      // 处理 Null 类型
      [](const JsonNull&) -> std::string { return "null"; },
      // 处理布尔类型
      [](const JsonBoolean& b) -> std::string { return b ? "true" : "false"; },
      // 处理数字类型
      [](const JsonNumber& n) -> std::string { return std::to_string(n); },
      // 处理字符串类型（JSON 字符串需要带双引号）
      [](const JsonString& s) -> std::string {
        return "\"" + s + "\"";  // 简化实现，实际应处理转义字符（如\"、\n等）
      },
      // 处理数组类型（递归调用 to_string）
      [](const std::unique_ptr<JsonArray>& arr) -> std::string {
        if (!arr) {
          return "[]";  // 空指针安全处理
        }
        std::stringstream ss;
        ss << "[";
        // 遍历数组元素，拼接每个元素的字符串
        for (size_t i = 0; i < arr->size(); ++i) {
          ss << arr->at(i).to_string();  // 递归调用元素的 to_string
          if (i != arr->size() - 1) {
            ss << ", ";
          }
        }
        ss << "]";
        return ss.str();
      },
      // 处理对象类型（递归调用 to_string）
      [](const std::unique_ptr<JsonObject>& obj) -> std::string {
        if (!obj)
          return "{}";  // 空指针安全处理
        std::stringstream ss;
        ss << "{";
        // 遍历键值对，拼接每个键值对的字符串
        size_t count = 0;
        for (const auto& [key, value] : *obj) {
          ss << "\"" << key
             << "\": " << value.to_string();  // 键加双引号，值递归转换
          if (count != obj->size() - 1) {
            ss << ", ";
          }
          ++count;
        }
        ss << "}";
        return ss.str();
      }
    },
    m_value
  );
}

auto JsonParser::parse() -> Result<JsonValue, ParseError> {
  return parse_value();
}

void JsonParser::consume() {
  current_token = m_lexer->next_token();
}

auto JsonParser::check(TokenType type) const -> bool {
  return current_token.has_value() && current_token->type == type;
}

auto JsonParser::match(TokenType type) -> bool {
  if (check(type)) {
    consume();
    return true;
  }
  return false;
}

auto JsonParser::expect(TokenType type) -> Result<void, ParseError> {
  if (check(type)) {
    consume();
    return Ok();
  }
  return Err(ParseError::UnexpectedToken);
}

auto JsonParser::parse_value() -> Result<JsonValue, ParseError> {
  if (!current_token.has_value()) {
    return Err(ParseError::UnexpectedEndOfInput);
  }

  switch (current_token->type) {
    case TokenType::LeftCurly: {
      auto obj_result = parse_object();
      if (obj_result.is_err()) {
        return Err(obj_result.unwrap_err());
      }
      return Ok(JsonValue(std::move(obj_result).unwrap()));
    }
    case TokenType::LeftBracket: {
      auto arr_result = parse_array();
      if (arr_result.is_err()) {
        return Err(arr_result.unwrap_err());
      }
      return Ok(JsonValue(std::move(arr_result).unwrap()));
    }
    case TokenType::String: {
      auto str_result = parse_string();
      if (str_result.is_err()) {
        return Err(str_result.unwrap_err());
      }
      return Ok(JsonValue(std::move(str_result).unwrap()));
    }
    case TokenType::Integer: {
      auto int_result = parse_number();
      if (int_result.is_err()) {
        return Err(int_result.unwrap_err());
      }
      return Ok(JsonValue(std::move(int_result).unwrap()));
    }
    case TokenType::Bool: {
      auto bool_result = parse_boolean();
      if (bool_result.is_err()) {
        return Err(bool_result.unwrap_err());
      }
      return Ok(JsonValue(std::move(bool_result).unwrap()));
    }
    case TokenType::Null: {
      auto null_result = parse_null();
      if (null_result.is_err()) {
        return Err(null_result.unwrap_err());
      }
      return Ok(JsonValue(std::move(null_result).unwrap()));
    }

    default:
      return Err(ParseError::UnexpectedToken);
  }
}

auto JsonParser::parse_object()
  -> Result<std::unique_ptr<JsonObject>, ParseError> {
  auto object = std::make_unique<JsonObject>();

  // 消耗左花括号
  auto err = expect(TokenType::LeftCurly);
  if (err.is_err()) {
    return Err(err.unwrap_err());
  }

  // 空对象
  if (check(TokenType::RightCurly)) {
    consume();
    return Ok(std::move(object));
  }

  // 解析键值对
  while (true) {
    // 解析键
    if (!check(TokenType::String)) {
      return Err(ParseError::UnexpectedToken);
    }

    auto key = current_token->value.substr(1, current_token->value.size() - 2);
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

    // 添加到对象
    object->emplace(std::move(key), std::move(value_result).unwrap());

    // 检查是否有更多键值对
    if (match(TokenType::RightCurly)) {
      break;
    }

    if (!match(TokenType::Comma)) {
      return Err(ParseError::MissingCommaOrBracket);
    }
  }

  return Ok(std::move(object));
}

auto JsonParser::parse_array()
  -> Result<std::unique_ptr<JsonArray>, ParseError> {
  auto array = std::make_unique<JsonArray>();

  // 消耗左方括号
  auto err = expect(TokenType::LeftBracket);
  if (err.is_err()) {
    return Err(err.unwrap_err());
  }

  // 空数组
  if (check(TokenType::RightBracket)) {
    consume();
    return Ok(std::move(array));
  }

  // 解析数组元素
  while (true) {
    // 解析元素
    auto element_result = parse_value();
    if (element_result.is_err()) {
      return Err(element_result.unwrap_err());
    }

    array->emplace_back(std::move(element_result).unwrap());

    // 检查是否有更多元素
    if (match(TokenType::RightBracket)) {
      break;
    }

    if (!match(TokenType::Comma)) {
      return Err(ParseError::MissingCommaOrBracket);
    }
  }

  return Ok(std::move(array));
}

auto JsonParser::parse_string() -> Result<JsonString, ParseError> {
  if (!check(TokenType::String)) {
    return Err(ParseError::UnexpectedToken);
  }

  // 简化实现：直接返回字符串值（假设词法分析器已处理转义序列）
  std::string value =
    current_token->value.substr(1, current_token->value.size() - 2);
  consume();

  return Ok(std::move(value));
}

auto JsonParser::parse_number() -> Result<JsonNumber, ParseError> {
  if (!check(TokenType::Integer)) {
    return Err(ParseError::UnexpectedToken);
  }

  const std::string& num_str = current_token->value;

  int64_t value = std::stoi(num_str);
  consume();
  return Ok(value);
}

auto JsonParser::parse_boolean() -> Result<JsonBoolean, ParseError> {
  if (!check(TokenType::Bool)) {
    return Err(ParseError::UnexpectedToken);
  }

  bool value = (current_token->value == "true");
  consume();

  return Ok(value);
}

Result<JsonNull, ParseError> JsonParser::parse_null() {
  if (!check(TokenType::Null)) {
    return Err(ParseError::UnexpectedToken);
  }

  consume();
  return Ok(JsonNull{});
}

}  // namespace Parser
