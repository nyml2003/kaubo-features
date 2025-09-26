#pragma once

#include <cstdint>
namespace Json {
enum class ParseError : uint8_t {
  UnexpectedToken,
  UnexpectedEndOfInput,
  InvalidNumberFormat,
  MissingColonInObject,
  MissingCommaOrBracket,
  MissingQuote,
  InvalidEscapeSequence
};

}  // namespace Json

namespace std {

inline auto to_string(Json::ParseError error) -> const char* {
  switch (error) {
    case Json::ParseError::UnexpectedToken:
      return "Unexpected token";
    case Json::ParseError::UnexpectedEndOfInput:
      return "Unexpected end of input";
    case Json::ParseError::InvalidNumberFormat:
      return "Invalid number format";
    case Json::ParseError::MissingColonInObject:
      return "Missing colon in object";
    case Json::ParseError::MissingCommaOrBracket:
      return "Missing comma or bracket";
    case Json::ParseError::MissingQuote:
      return "Missing quote";
    case Json::ParseError::InvalidEscapeSequence:
      return "Invalid escape sequence";
  }
}
}  // namespace std