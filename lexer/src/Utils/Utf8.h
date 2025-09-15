#pragma once
#include <cassert>
#include <cstdint>
#include <string_view>
#include <vector>

#include "Utils/Result.h"

namespace Utils::Utf8 {
// UTF-8 错误类型枚举
enum class Error : uint8_t {
  InvalidPosition,      // 位置超出输入范围
  IncompleteSequence,   // 不完整的多字节序列
  InvalidContinuation,  // 无效的续字节（未符合 10xxxxxx 格式）
  OverlongEncoding,     // 过度编码（用超出必要长度的字节表示码点）
  InvalidCodePoint,     // 码点超出 Unicode 合法范围（>0x10FFFF）
  InvalidLeadingByte    // 无效的首字节（如 10xxxxxx/11111xxx 作为首字节）
};

// UTF-8 核心常量定义
namespace Const {
// 首字节掩码/匹配值
constexpr uint8_t MASK_1BYTE = 0x80;
constexpr uint8_t MASK_2BYTE = 0xE0;
constexpr uint8_t MATCH_2BYTE = 0xC0;
constexpr uint8_t MASK_3BYTE = 0xF0;
constexpr uint8_t MATCH_3BYTE = 0xE0;
constexpr uint8_t MASK_4BYTE = 0xF8;
constexpr uint8_t MATCH_4BYTE = 0xF0;

// 续字节掩码/匹配值
constexpr uint8_t CONT_MASK = 0xC0;
constexpr uint8_t CONT_MATCH = 0x80;
constexpr uint8_t CONT_DATA_MASK = 0x3F;

// 首字节数据位掩码
constexpr uint8_t DATA_MASK_2BYTE = 0x1F;
constexpr uint8_t DATA_MASK_3BYTE = 0x0F;
constexpr uint8_t DATA_MASK_4BYTE = 0x07;

// 位移量
constexpr uint8_t SHIFT_6 = 6;
constexpr uint8_t SHIFT_12 = 12;
constexpr uint8_t SHIFT_18 = 18;

// 码点范围
constexpr char32_t MAX_1BYTE = 0x7F;
constexpr char32_t MAX_2BYTE = 0x7FF;
constexpr char32_t MIN_2BYTE = 0x80;
constexpr char32_t MIN_3BYTE = 0x800;
constexpr char32_t MIN_4BYTE = 0x10000;
constexpr char32_t MAX_UNICODE = 0x10FFFF;

// whitespace
constexpr char32_t SPACE = 0x20;
constexpr char32_t TAB = 0x09;
constexpr char32_t LineFeed = 0x0A;
constexpr char32_t CarriageReturn = 0x0D;

// 字母
constexpr char32_t MIN_LOWER_CASE = 0x0061;  // 'a'
constexpr char32_t MAX_LOWER_CASE = 0x007A;  // 'z'
constexpr char32_t MIN_UPPER_CASE = 0x0041;  // 'A'
constexpr char32_t MAX_UPPER_CASE = 0x005A;  // 'Z'
constexpr char32_t UNDERSCORE = 0x005F;      // '_'

// 数字
constexpr char32_t MIN_DIGIT = 0x0030;  // '0'
constexpr char32_t MAX_DIGIT = 0x0039;  // '9'

// '
constexpr char32_t SINGLE_QUOTE = 0x0027;  // '''
constexpr char32_t DOUBLE_QUOTE = 0x0022;  // '"'

}  // namespace Const

namespace Internal {
/**
 * @brief 获取UTF-8编码的预期字节长度
 *
 * @param leading_byte UTF-8编码的首字节
 * @return Result<size_t, Error>
 */
inline auto get_expected_byte_count(uint8_t leading_byte) noexcept
  -> Result<size_t, Error> {
  if ((leading_byte & Const::MASK_1BYTE) == 0) {
    return Ok<size_t>(1);  // 单字节（0xxxxxxx）
  }
  if ((leading_byte & Const::MASK_2BYTE) == Const::MATCH_2BYTE) {
    return Ok<size_t>(2);  // 双字节（110xxxxx）
  }
  if ((leading_byte & Const::MASK_3BYTE) == Const::MATCH_3BYTE) {
    return Ok<size_t>(3);  // 三字节（1110xxxx）
  }
  if ((leading_byte & Const::MASK_4BYTE) == Const::MATCH_4BYTE) {
    return Ok<size_t>(4);  // 四字节（11110xxx）
  }
  return Err(Error::InvalidLeadingByte);  // 无效首字节
}

struct ValidateContinuationBytesArgs {
  size_t start_pos;        // 起始位置
  size_t expected_length;  // 预期长度
  std::string_view input;  // 输入字符串
};

/**
 * @brief 校验后续字节是否均符合"10xxxxxx"格式
 *
 * @param args
 * @return Result<void, Error>
 */
inline auto validate_continuation_bytes(
  ValidateContinuationBytesArgs args
) noexcept -> Result<void, Error> {
  const auto [start_pos, expected_length, input] = args;
  // 遍历所有续字节
  for (size_t i = 1; i < expected_length; ++i) {
    const size_t current_pos = start_pos + i;
    const auto cont_byte = static_cast<uint8_t>(input[current_pos]);
    if ((cont_byte & Const::CONT_MASK) != Const::CONT_MATCH) {
      return Err(Error::InvalidContinuation);
    }
  }
  return Ok();  // 所有续字节均有效
}

struct ComputeCodePointArgs {
  uint8_t leading_byte;    // 首字节
  size_t start_pos;        // 开始位置
  size_t expected_length;  // 预期长度
  std::string_view input;  // 输入字符串
};

/**
 * @brief 拼接多字节数据，计算最终Unicode码点
 *
 * @param args
 * @return char32_t Utf8码点
 */
inline auto compute_codepoint(ComputeCodePointArgs args) noexcept -> char32_t {
  const auto [leading_byte, start_pos, expected_length, input] = args;
  char32_t codepoint = 0;

  switch (expected_length) {
    case 1:
      codepoint = leading_byte;
      break;
    case 2: {
      const auto byte2 = static_cast<uint8_t>(input[start_pos + 1]);
      codepoint = ((leading_byte & Const::DATA_MASK_2BYTE) << Const::SHIFT_6) |
                  (byte2 & Const::CONT_DATA_MASK);
      break;
    }
    case 3: {
      const auto byte2 = static_cast<uint8_t>(input[start_pos + 1]);
      const auto byte3 = static_cast<uint8_t>(input[start_pos + 2]);
      codepoint = ((leading_byte & Const::DATA_MASK_3BYTE) << Const::SHIFT_12) |
                  ((byte2 & Const::CONT_DATA_MASK) << Const::SHIFT_6) |
                  (byte3 & Const::CONT_DATA_MASK);
      break;
    }
    case 4: {
      const auto byte2 = static_cast<uint8_t>(input[start_pos + 1]);
      const auto byte3 = static_cast<uint8_t>(input[start_pos + 2]);
      const auto byte4 = static_cast<uint8_t>(input[start_pos + 3]);
      codepoint = ((leading_byte & Const::DATA_MASK_4BYTE) << Const::SHIFT_18) |
                  ((byte2 & Const::CONT_DATA_MASK) << Const::SHIFT_12) |
                  ((byte3 & Const::CONT_DATA_MASK) << Const::SHIFT_6) |
                  (byte4 & Const::CONT_DATA_MASK);
      break;
    }
    default:
      assert(false && "不可能执行到的分支");
      break;
  }

  return codepoint;
}

struct IsOverlongEncodingArgs {
  char32_t codepoint;  // Utf8码点
  size_t byte_count;   // 字节长度
};

/**
 * @brief 判断码点是否存在"过度编码"
 *
 * @param args
 * @return true 不合法的字符
 * @return false 合法字符
 */
inline auto is_overlong_encoding(IsOverlongEncodingArgs args) noexcept -> bool {
  const auto [codepoint, byte_count] = args;
  bool result = false;
  switch (byte_count) {
    case 1:
      result = codepoint > Const::MAX_1BYTE;
      break;
    case 2:
      result = codepoint < Const::MIN_2BYTE;
      break;
    case 3:
      result = codepoint < Const::MIN_3BYTE;
      break;
    case 4:
      result = codepoint < Const::MIN_4BYTE;
      break;
    default:
      assert(false && "不可能执行到的分支");
      result = true;
  }
  return result;
}

}  // namespace Internal

inline auto is_unicode_whitespace(char32_t codepoint) noexcept -> bool {
  return codepoint == Const::SPACE || codepoint == Const::TAB ||
         codepoint == Const::LineFeed || codepoint == Const::CarriageReturn;
}

inline auto is_identifier_start(char32_t codepoint) noexcept -> bool {
  return (codepoint >= Const::MIN_LOWER_CASE &&
          codepoint <= Const::MAX_LOWER_CASE) ||
         (codepoint >= Const::MIN_UPPER_CASE &&
          codepoint <= Const::MAX_UPPER_CASE) ||
         codepoint == Const::UNDERSCORE;
}

inline auto is_identifier_part(char32_t codepoint) noexcept -> bool {
  return is_identifier_start(codepoint) ||
         (codepoint >= Const::MIN_DIGIT && codepoint <= Const::MAX_DIGIT) ||
         is_identifier_start(codepoint);
}

inline auto is_digit(char32_t codepoint) noexcept -> bool {
  return codepoint >= Const::MIN_DIGIT && codepoint <= Const::MAX_DIGIT;
}

inline auto is_string_start(char32_t codepoint) noexcept -> bool {
  return codepoint == Const::DOUBLE_QUOTE || codepoint == Const::SINGLE_QUOTE;
}

inline auto is_string_end(char32_t codepoint, char32_t begin_codepoint) noexcept
  -> bool {
  return codepoint == begin_codepoint;
}

using CodePoint = std::pair<char32_t, size_t>;  // (码点, 字节长度)

/**
 * @brief 获取第一个Utf8码点及其长度
 *
 * @param input 输入字符串
 * @param pos    起始位置
 * @return DecodeResult
 */
inline auto get_utf8_codepoint(std::string_view input, size_t pos)
  -> Result<CodePoint, Error> {
  // 检查起始位置是否有效
  if (pos >= input.size()) {
    return Err(Error::InvalidPosition);
  }

  // 步骤2：分析首字节
  const auto leading_byte = static_cast<uint8_t>(input[pos]);
  auto expected_length_result = Internal::get_expected_byte_count(leading_byte);
  if (expected_length_result.is_err()) {
    return Err(std::move(expected_length_result.unwrap_err()));
  }
  const size_t expected_length = expected_length_result.unwrap();

  // 步骤3：检查序列完整性
  if (pos + expected_length > input.size()) {
    return Err(Error::IncompleteSequence);
  }

  // 步骤4：校验续字节
  auto validation_result = validate_continuation_bytes(
    Internal::ValidateContinuationBytesArgs{
      .start_pos = pos, .expected_length = expected_length, .input = input
    }
  );
  if (validation_result.is_err()) {
    return Err(std::move(validation_result.unwrap_err()));
  }

  // 步骤5：计算码点
  auto codepoint_result = compute_codepoint(
    Internal::ComputeCodePointArgs{
      .leading_byte = leading_byte,
      .start_pos = pos,
      .expected_length = expected_length,
      .input = input
    }
  );
  const char32_t codepoint = codepoint_result;

  // 步骤6：检查过度编码
  auto overlong_result = is_overlong_encoding(
    Internal::IsOverlongEncodingArgs{
      .codepoint = codepoint, .byte_count = expected_length
    }
  );
  if (overlong_result) {
    return Err(Error::OverlongEncoding);
  }

  // 步骤7：检查码点范围
  if (codepoint > Const::MAX_UNICODE) {
    return Err(Error::InvalidCodePoint);
  }

  // 所有校验通过
  return Ok<CodePoint>({codepoint, expected_length});
}

inline auto to_utf8(char32_t codepoint) noexcept -> std::string {
  std::string result;

  // 检查是否为有效的Unicode码点
  if (codepoint > Const::MAX_UNICODE) {
    return result;  // 返回空字符串表示无效
  }

  if (codepoint <= Const::MAX_1BYTE) {
    // 1字节编码: 0xxxxxxx
    result += static_cast<char>(codepoint);
  } else if (codepoint <= Const::MAX_2BYTE) {
    // 2字节编码: 110xxxxx 10xxxxxx
    result += static_cast<char>(
      Const::MATCH_2BYTE | (static_cast<uint8_t>(codepoint >> Const::SHIFT_6) &
                            Const::DATA_MASK_2BYTE)
    );
    result += static_cast<char>(
      Const::CONT_MATCH |
      (static_cast<uint8_t>(codepoint) & Const::CONT_DATA_MASK)
    );
  } else if (codepoint < Const::MIN_4BYTE) {
    // 3字节编码: 1110xxxx 10xxxxxx 10xxxxxx
    result += static_cast<char>(
      Const::MATCH_3BYTE | (static_cast<uint8_t>(codepoint >> Const::SHIFT_12) &
                            Const::DATA_MASK_3BYTE)
    );
    result += static_cast<char>(
      Const::CONT_MATCH | (static_cast<uint8_t>(codepoint >> Const::SHIFT_6) &
                           Const::CONT_DATA_MASK)
    );
    result += static_cast<char>(
      Const::CONT_MATCH |
      (static_cast<uint8_t>(codepoint) & Const::CONT_DATA_MASK)
    );
  } else {
    // 4字节编码: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
    result += static_cast<char>(
      Const::MATCH_4BYTE | (static_cast<uint8_t>(codepoint >> Const::SHIFT_18) &
                            Const::DATA_MASK_4BYTE)
    );
    result += static_cast<char>(
      Const::CONT_MATCH | (static_cast<uint8_t>(codepoint >> Const::SHIFT_12) &
                           Const::CONT_DATA_MASK)
    );
    result += static_cast<char>(
      Const::CONT_MATCH | (static_cast<uint8_t>(codepoint >> Const::SHIFT_6) &
                           Const::CONT_DATA_MASK)
    );
    result += static_cast<char>(
      Const::CONT_MATCH |
      (static_cast<uint8_t>(codepoint) & Const::CONT_DATA_MASK)
    );
  }

  return result;
}

inline auto build_string(const std::vector<char32_t>& codepoints) noexcept
  -> std::string {
  std::string result;
  for (const auto codepoint : codepoints) {
    result += to_utf8(codepoint);
  }
  return result;
}

}  // namespace Utils::Utf8

namespace std {
inline auto to_string(Utils::Utf8::Error error) noexcept -> std::string {
  switch (error) {
    case Utils::Utf8::Error::InvalidPosition:
      return "Invalid position";
    case Utils::Utf8::Error::IncompleteSequence:
      return "Incomplete sequence";
    case Utils::Utf8::Error::OverlongEncoding:
      return "Overlong encoding";
    case Utils::Utf8::Error::InvalidCodePoint:
      return "Invalid code point";
    case Utils::Utf8::Error::InvalidContinuation:
      return "Invalid continuation";
    case Utils::Utf8::Error::InvalidLeadingByte:
      return "Invalid leading byte";
    default:
      assert(false && "不可能执行到的分支");
  }
}
inline auto to_string(const std::vector<char32_t>& codepoints) noexcept
  -> std::string {
  return Utils::Utf8::build_string(codepoints);
}
}  // namespace std