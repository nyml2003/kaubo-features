// #pragma once

// #include <cstdint>
// #include <map>
// #include <memory>
// #include <optional>
// #include <string>
// #include <variant>
// #include <vector>

// #include "Lexer/Json/TokenType.h"
// #include "Lexer/Lexer.h"
// #include "Utils/Result.h"

// namespace Parser {
// using Lexer::Json::TokenType;
// using JsonNull = std::monostate;
// using JsonBoolean = bool;
// using JsonNumber = int64_t;  // 简化实现，仅支持整数
// using JsonString = std::string;
// using Utils::Result;

// class JsonValue;

// using JsonArray = std::vector<JsonValue>;
// using JsonObject = std::map<std::string, JsonValue>;

// class JsonValue {
//  public:
//   using ValueType = std::variant<
//     JsonNull,
//     JsonBoolean,
//     JsonNumber,
//     JsonString,
//     std::unique_ptr<JsonArray>,
//     std::unique_ptr<JsonObject>>;

//   JsonValue() = default;

//   // 各种类型的构造函数
//   // NOLINTNEXTLINE(google-explicit-constructor)
//   JsonValue(JsonNull /*unused*/) : m_value(JsonNull{}) {}
//   // NOLINTNEXTLINE(google-explicit-constructor)
//   JsonValue(JsonBoolean b) : m_value(b) {}
//   // NOLINTNEXTLINE(google-explicit-constructor)
//   JsonValue(JsonNumber n) : m_value(n) {}
//   // NOLINTNEXTLINE(google-explicit-constructor)
//   JsonValue(JsonString s) : m_value(std::move(s)) {}
//   // NOLINTNEXTLINE(google-explicit-constructor)
//   JsonValue(std::unique_ptr<JsonArray> arr) : m_value(std::move(arr)) {}
//   // NOLINTNEXTLINE(google-explicit-constructor)
//   JsonValue(std::unique_ptr<JsonObject> obj) : m_value(std::move(obj)) {}

//   // 获取值类型的访问方法
//   [[nodiscard]] auto get() const -> const ValueType& { return m_value; }

//   [[nodiscard]] auto to_string() const -> std::string;

//  private:
//   ValueType m_value;
// };

// enum class ParseError : uint8_t {
//   UnexpectedToken,
//   UnexpectedEndOfInput,
//   InvalidNumberFormat,
//   MissingColonInObject,
//   MissingCommaOrBracket,
//   MissingQuote,
//   InvalidEscapeSequence
// };

// inline auto to_string(ParseError error) -> const char* {
//   switch (error) {
//     case ParseError::UnexpectedToken:
//       return "Unexpected token";
//     case ParseError::UnexpectedEndOfInput:
//       return "Unexpected end of input";
//     case ParseError::InvalidNumberFormat:
//       return "Invalid number format";
//     case ParseError::MissingColonInObject:
//       return "Missing colon in object";
//     case ParseError::MissingCommaOrBracket:
//       return "Missing comma or bracket";
//     case ParseError::MissingQuote:
//       return "Missing quote";
//     case ParseError::InvalidEscapeSequence:
//       return "Invalid escape sequence";
//   }
// }

// class JsonParser {
//  public:
//   explicit JsonParser(const std::shared_ptr<Lexer::Proto<TokenType>>& lexer)
//     : m_lexer(lexer) {
//     consume();  // 预读第一个token
//   }

//   auto parse() -> Result<JsonValue, ParseError>;

//  private:
//   std::shared_ptr<Lexer::Proto<TokenType>> m_lexer;
//   std::optional<Lexer::Token<TokenType>> current_token;

//   // 消费当前token并读取下一个
//   void consume();

//   // 检查当前token是否为指定类型
//   [[nodiscard]] auto check(TokenType type) const -> bool;

//   // 检查并消费指定类型的token
//   auto match(TokenType type) -> bool;

//   // 期望并消费指定类型的token，否则返回错误
//   auto expect(TokenType type) -> Result<void, ParseError>;

//   // 解析JSON值
//   auto parse_value() -> Result<JsonValue, ParseError>;

//   // 解析JSON对象
//   auto parse_object() -> Result<std::unique_ptr<JsonObject>, ParseError>;

//   // 解析JSON数组
//   auto parse_array() -> Result<std::unique_ptr<JsonArray>, ParseError>;

//   // 解析JSON字符串
//   auto parse_string() -> Result<JsonString, ParseError>;

//   // 解析JSON数字
//   auto parse_number() -> Result<JsonNumber, ParseError>;

//   // 解析JSON布尔值
//   auto parse_boolean() -> Result<JsonBoolean, ParseError>;

//   // 解析JSON null
//   auto parse_null() -> Result<JsonNull, ParseError>;
// };

// }  // namespace Parser
