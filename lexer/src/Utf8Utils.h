#pragma once
#include <cassert>
#include <cstdint>
#include <string_view>

#include "Result.h"

namespace Utf8Utils {
// UTF-8 错误类型枚举
enum class Utf8Error : uint8_t {
  InvalidPosition,      // 位置超出输入范围
  IncompleteSequence,   // 不完整的多字节序列
  InvalidContinuation,  // 无效的续字节（未符合 10xxxxxx 格式）
  OverlongEncoding,     // 过度编码（用超出必要长度的字节表示码点）
  InvalidCodePoint,     // 码点超出 Unicode 合法范围（>0x10FFFF）
  InvalidLeadingByte    // 无效的首字节（如 10xxxxxx/11111xxx 作为首字节）
};

// UTF-8 核心常量定义
namespace Utf8Const {
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

}  // namespace Utf8Const

// 结果类型定义
using Utf8Success = std::pair<char32_t, size_t>;  // (码点, 字节长度)
using DecodeResult = Result::Result<Utf8Success, Utf8Error>;

// 子函数1：分析首字节，返回预期的编码字节数
inline auto get_expected_byte_count(uint8_t leading_byte) noexcept
  -> Result::Result<size_t, Utf8Error> {
  if ((leading_byte & Utf8Const::MASK_1BYTE) == 0) {
    return Result::Ok<size_t, Utf8Error>(1);  // 单字节（0xxxxxxx）
  }
  if ((leading_byte & Utf8Const::MASK_2BYTE) == Utf8Const::MATCH_2BYTE) {
    return Result::Ok<size_t, Utf8Error>(2);  // 双字节（110xxxxx）
  }
  if ((leading_byte & Utf8Const::MASK_3BYTE) == Utf8Const::MATCH_3BYTE) {
    return Result::Ok<size_t, Utf8Error>(3);  // 三字节（1110xxxx）
  }
  if ((leading_byte & Utf8Const::MASK_4BYTE) == Utf8Const::MATCH_4BYTE) {
    return Result::Ok<size_t, Utf8Error>(4);  // 四字节（11110xxx）
  }
  return Result::Err<size_t, Utf8Error>(Utf8Error::InvalidLeadingByte
  );  // 无效首字节
}

// 子函数2：校验后续字节是否均符合"10xxxxxx"格式
struct ValidateContinuationBytesArgs {
  size_t start_pos;
  size_t expected_length;
  std::string_view input;
};

inline auto validate_continuation_bytes(
  ValidateContinuationBytesArgs args
) noexcept -> Result::Result<void, Utf8Error> {
  const auto [start_pos, expected_length, input] = args;
  // 遍历所有续字节
  for (size_t i = 1; i < expected_length; ++i) {
    const size_t current_pos = start_pos + i;
    const auto cont_byte = static_cast<uint8_t>(input[current_pos]);
    if ((cont_byte & Utf8Const::CONT_MASK) != Utf8Const::CONT_MATCH) {
      return Result::Err<void, Utf8Error>(Utf8Error::InvalidContinuation);
    }
  }
  return Result::Ok<void, Utf8Error>();  // 所有续字节均有效
}

// 子函数3：拼接多字节数据，计算最终Unicode码点
struct ComputeCodePointArgs {
  uint8_t leading_byte;
  size_t start_pos;
  size_t expected_length;
  std::string_view input;
};

inline auto compute_codepoint(ComputeCodePointArgs args) noexcept -> char32_t {
  const auto [leading_byte, start_pos, expected_length, input] = args;
  char32_t codepoint = 0;

  switch (expected_length) {
    case 1:
      codepoint = leading_byte;
      break;
    case 2: {
      const auto byte2 = static_cast<uint8_t>(input[start_pos + 1]);
      codepoint =
        ((leading_byte & Utf8Const::DATA_MASK_2BYTE) << Utf8Const::SHIFT_6) |
        (byte2 & Utf8Const::CONT_DATA_MASK);
      break;
    }
    case 3: {
      const auto byte2 = static_cast<uint8_t>(input[start_pos + 1]);
      const auto byte3 = static_cast<uint8_t>(input[start_pos + 2]);
      codepoint =
        ((leading_byte & Utf8Const::DATA_MASK_3BYTE) << Utf8Const::SHIFT_12) |
        ((byte2 & Utf8Const::CONT_DATA_MASK) << Utf8Const::SHIFT_6) |
        (byte3 & Utf8Const::CONT_DATA_MASK);
      break;
    }
    case 4: {
      const auto byte2 = static_cast<uint8_t>(input[start_pos + 1]);
      const auto byte3 = static_cast<uint8_t>(input[start_pos + 2]);
      const auto byte4 = static_cast<uint8_t>(input[start_pos + 3]);
      codepoint =
        ((leading_byte & Utf8Const::DATA_MASK_4BYTE) << Utf8Const::SHIFT_18) |
        ((byte2 & Utf8Const::CONT_DATA_MASK) << Utf8Const::SHIFT_12) |
        ((byte3 & Utf8Const::CONT_DATA_MASK) << Utf8Const::SHIFT_6) |
        (byte4 & Utf8Const::CONT_DATA_MASK);
      break;
    }
    default:
      assert(false && "不可能执行到的分支");
      break;
  }

  return codepoint;
}

// 子函数4：判断码点是否存在"过度编码"
struct IsOverlongEncodingArgs {
  char32_t codepoint;
  size_t byte_count;
};

inline auto is_overlong_encoding(IsOverlongEncodingArgs args) noexcept -> bool {
  const auto [codepoint, byte_count] = args;
  bool result = false;
  switch (byte_count) {
    case 1:
      result = codepoint > Utf8Const::MAX_1BYTE;
      break;
    case 2:
      result = codepoint < Utf8Const::MIN_2BYTE;
      break;
    case 3:
      result = codepoint < Utf8Const::MIN_3BYTE;
      break;
    case 4:
      result = codepoint < Utf8Const::MIN_4BYTE;
      break;
    default:
      assert(false && "不可能执行到的分支");
      result = true;
  }
  return result;
}

// 主函数：解析UTF-8编码的码点
inline auto get_utf8_codepoint(std::string_view input, size_t pos)
  -> DecodeResult {
  // 步骤1：检查起始位置是否有效
  if (pos >= input.size()) {
    return Result::Err<Utf8Success, Utf8Error>(Utf8Error::InvalidPosition);
  }

  // 步骤2：分析首字节
  const auto leading_byte = static_cast<uint8_t>(input[pos]);
  auto expected_length_result = get_expected_byte_count(leading_byte);
  if (expected_length_result.is_err()) {
    return Result::Err<Utf8Success, Utf8Error>(
      std::move(expected_length_result.unwrap_err())
    );
  }
  const size_t expected_length = expected_length_result.unwrap();

  // 步骤3：检查序列完整性
  if (pos + expected_length > input.size()) {
    return Result::Err<Utf8Success, Utf8Error>(Utf8Error::IncompleteSequence);
  }

  // 步骤4：校验续字节
  auto validation_result = validate_continuation_bytes(
    ValidateContinuationBytesArgs{
      .start_pos = pos, .expected_length = expected_length, .input = input
    }
  );
  if (validation_result.is_err()) {
    return Result::Err<Utf8Success, Utf8Error>(
      std::move(validation_result.unwrap_err())
    );
  }

  // 步骤5：计算码点
  auto codepoint_result = compute_codepoint(
    ComputeCodePointArgs{
      .leading_byte = leading_byte,
      .start_pos = pos,
      .expected_length = expected_length,
      .input = input
    }
  );
  const char32_t codepoint = codepoint_result;

  // 步骤6：检查过度编码
  auto overlong_result = is_overlong_encoding(
    IsOverlongEncodingArgs{
      .codepoint = codepoint, .byte_count = expected_length
    }
  );
  if (overlong_result) {
    return Result::Err<Utf8Success, Utf8Error>(Utf8Error::OverlongEncoding);
  }

  // 步骤7：检查码点范围
  if (codepoint > Utf8Const::MAX_UNICODE) {
    return Result::Err<Utf8Success, Utf8Error>(Utf8Error::InvalidCodePoint);
  }

  // 所有校验通过
  return Result::Ok<Utf8Success, Utf8Error>({codepoint, expected_length});
}

inline auto is_unicode_whitespace(char32_t codepoint) noexcept -> bool {
  return codepoint == Utf8Const::SPACE || codepoint == Utf8Const::TAB ||
         codepoint == Utf8Const::LineFeed ||
         codepoint == Utf8Const::CarriageReturn;
}

inline auto is_identifier_start(char32_t codepoint) noexcept -> bool {
  return (codepoint >= Utf8Const::MIN_LOWER_CASE &&
          codepoint <= Utf8Const::MAX_LOWER_CASE) ||
         (codepoint >= Utf8Const::MIN_UPPER_CASE &&
          codepoint <= Utf8Const::MAX_UPPER_CASE) ||
         codepoint == Utf8Const::UNDERSCORE;
}

inline auto is_identifier_part(char32_t codepoint) noexcept -> bool {
  return is_identifier_start(codepoint) ||
         (codepoint >= Utf8Const::MIN_DIGIT && codepoint <= Utf8Const::MAX_DIGIT
         ) ||
         is_identifier_start(codepoint);
}

}  // namespace Utf8Utils

namespace std {
inline auto to_string(Utf8Utils::Utf8Error error) noexcept -> std::string {
  switch (error) {
    case Utf8Utils::Utf8Error::InvalidPosition:
      return "Invalid position";
    case Utf8Utils::Utf8Error::IncompleteSequence:
      return "Incomplete sequence";
    case Utf8Utils::Utf8Error::OverlongEncoding:
      return "Overlong encoding";
    case Utf8Utils::Utf8Error::InvalidCodePoint:
      return "Invalid code point";
    case Utf8Utils::Utf8Error::InvalidContinuation:
      return "Invalid continuation";
    case Utf8Utils::Utf8Error::InvalidLeadingByte:
      return "Invalid leading byte";
    default:
      assert(false && "不可能执行到的分支");
  }
}
}  // namespace std