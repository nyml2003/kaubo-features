#include <gtest/gtest.h>
#include <string_view>
#include "Utils/Result.h"
#include "Utils/Utf8.h"

using Utils::Utf8::get_utf8_codepoint;

// 测试单字节ASCII字符（全部为成功场景）
TEST(UTF8DecoderTest, SingleByteCharacters) {
  // 常规ASCII字符
  EXPECT_EQ(get_utf8_codepoint("A", 0).unwrap(), std::make_pair(U'A', 1U));
  EXPECT_EQ(get_utf8_codepoint("a", 0).unwrap(), std::make_pair(U'a', 1U));
  EXPECT_EQ(get_utf8_codepoint("0", 0).unwrap(), std::make_pair(U'0', 1U));
  EXPECT_EQ(get_utf8_codepoint(" ", 0).unwrap(), std::make_pair(U' ', 1U));
  EXPECT_EQ(get_utf8_codepoint("!", 0).unwrap(), std::make_pair(U'!', 1U));

  // 空字符（0x00，合法单字节编码）
  std::string_view null_char("\0", 1);
  EXPECT_EQ(
    get_utf8_codepoint(null_char, 0).unwrap(), std::make_pair(U'\0', 1U)
  );
}

// 测试空字符相关场景（含成功/失败）
TEST(UTF8DecoderTest, NullCharacterScenarios) {
  // 用例1：单个空字符（成功）
  std::string_view single_null("\0", 1);
  auto result1 = get_utf8_codepoint(single_null, 0);
  EXPECT_TRUE(result1.is_ok());
  EXPECT_EQ(result1.unwrap(), std::make_pair(U'\0', 1U));

  // 用例2：空字符串（pos=0超出范围，失败）
  auto result2 = get_utf8_codepoint("", 0);
  EXPECT_TRUE(result2.is_err());
  EXPECT_EQ(result2.unwrap_err(), Utils::Utf8::Error::InvalidPosition);

  // 用例3：多个连续空字符（均成功）
  std::string_view double_null("\0\0", 2);
  auto result3_1 = get_utf8_codepoint(double_null, 0);
  auto result3_2 = get_utf8_codepoint(double_null, 1);
  EXPECT_TRUE(result3_1.is_ok());
  EXPECT_TRUE(result3_2.is_ok());
  EXPECT_EQ(result3_1.unwrap(), std::make_pair(U'\0', 1U));
  EXPECT_EQ(result3_2.unwrap(), std::make_pair(U'\0', 1U));

  // 用例4：空字符后跟ASCII字符（均成功）
  std::string_view null_plus_a("\0A", 2);
  auto result4_1 = get_utf8_codepoint(null_plus_a, 0);
  auto result4_2 = get_utf8_codepoint(null_plus_a, 1);
  EXPECT_TRUE(result4_1.is_ok());
  EXPECT_TRUE(result4_2.is_ok());
  EXPECT_EQ(result4_1.unwrap(), std::make_pair(U'\0', 1U));
  EXPECT_EQ(result4_2.unwrap(), std::make_pair(U'A', 1U));

  // 用例5：空字符的过度编码（"\xC0\x80"非法，失败）
  auto result5 = get_utf8_codepoint("\xC0\x80", 0);
  EXPECT_TRUE(result5.is_err());
  EXPECT_EQ(result5.unwrap_err(), Utils::Utf8::Error::OverlongEncoding);

  // 用例6：pos超出空字符范围（失败）
  auto result6 = get_utf8_codepoint(single_null, 1);
  EXPECT_TRUE(result6.is_err());
  EXPECT_EQ(result6.unwrap_err(), Utils::Utf8::Error::InvalidPosition);

  // 用例7：空字符+多字节字符（均成功）
  std::string_view null_plus_you("\0\xE4\xBD\xA0", 4);  // '\0' + "你"
  auto result7_1 = get_utf8_codepoint(null_plus_you, 0);
  auto result7_2 = get_utf8_codepoint(null_plus_you, 1);
  EXPECT_TRUE(result7_1.is_ok());
  EXPECT_TRUE(result7_2.is_ok());
  EXPECT_EQ(result7_1.unwrap(), std::make_pair(U'\0', 1U));
  EXPECT_EQ(result7_2.unwrap(), std::make_pair(U'你', 3U));
}

// 测试双字节UTF-8字符（成功场景）
TEST(UTF8DecoderTest, TwoByteCharacters) {
  // 带重音的拉丁字母（U+00E1 á, U+00F1 ñ, U+00DF ß）
  EXPECT_EQ(
    get_utf8_codepoint("\xC3\xA1", 0).unwrap(), std::make_pair(U'á', 2U)
  );
  EXPECT_EQ(
    get_utf8_codepoint("\xC3\xB1", 0).unwrap(), std::make_pair(U'ñ', 2U)
  );
  EXPECT_EQ(
    get_utf8_codepoint("\xC3\x9F", 0).unwrap(), std::make_pair(U'ß', 2U)
  );

  // 双字节边界值（U+0080 ~ U+07FF）
  EXPECT_EQ(
    get_utf8_codepoint("\xC2\x80", 0).unwrap(), std::make_pair(0x80U, 2U)
  );  // 最小值
  EXPECT_EQ(
    get_utf8_codepoint("\xDF\xBF", 0).unwrap(), std::make_pair(0x7FFU, 2U)
  );  // 最大值
}

// 测试三字节UTF-8字符（成功场景）
TEST(UTF8DecoderTest, ThreeByteCharacters) {
  // 中日韩字符（U+4F60 你, U+65E5 日, U+0928 梵文न）
  EXPECT_EQ(
    get_utf8_codepoint("\xE4\xBD\xA0", 0).unwrap(), std::make_pair(U'你', 3U)
  );
  EXPECT_EQ(
    get_utf8_codepoint("\xE6\x97\xA5", 0).unwrap(), std::make_pair(U'日', 3U)
  );
  EXPECT_EQ(
    get_utf8_codepoint("\xE0\xA4\xA8", 0).unwrap(), std::make_pair(0x0928U, 3U)
  );

  // 三字节边界值（U+0800 ~ U+FFFF）
  EXPECT_EQ(
    get_utf8_codepoint("\xE0\xA0\x80", 0).unwrap(), std::make_pair(0x800U, 3U)
  );  // 最小值
  EXPECT_EQ(
    get_utf8_codepoint("\xEF\xBF\xBF", 0).unwrap(), std::make_pair(0xFFFFU, 3U)
  );  // 最大值
}

// 测试四字节UTF-8字符（成功场景）
TEST(UTF8DecoderTest, FourByteCharacters) {
  // 表情符号（U+1F60A 😊, U+1F30E 🌎, U+1F4A9 💩）
  EXPECT_EQ(
    get_utf8_codepoint("\xF0\x9F\x98\x8A", 0).unwrap(),
    std::make_pair(U'😊', 4U)
  );
  EXPECT_EQ(
    get_utf8_codepoint("\xF0\x9F\x8C\x8E", 0).unwrap(),
    std::make_pair(U'🌎', 4U)
  );
  EXPECT_EQ(
    get_utf8_codepoint("\xF0\x9F\x92\xA9", 0).unwrap(),
    std::make_pair(U'💩', 4U)
  );

  // 四字节边界值（U+10000 ~ U+10FFFF）
  EXPECT_EQ(
    get_utf8_codepoint("\xF0\x90\x80\x80", 0).unwrap(),
    std::make_pair(0x10000U, 4U)
  );  // 最小值
  EXPECT_EQ(
    get_utf8_codepoint("\xF4\x8F\xBF\xBF", 0).unwrap(),
    std::make_pair(0x10FFFFU, 4U)
  );  // 最大值（Unicode上限）
}

// 测试多字符混合字符串（成功场景）
TEST(UTF8DecoderTest, MultipleMixedCharacters) {
  // 字符串构成：A(1字节) + á(2字节) + 你(3字节) + 😊(4字节)
  std::string_view mixed_str = "A\xC3\xA1\xE4\xBD\xA0\xF0\x9F\x98\x8A";

  // 逐个解码验证
  auto res1 = get_utf8_codepoint(mixed_str, 0);  // A (pos=0)
  auto res2 = get_utf8_codepoint(mixed_str, 1);  // á (pos=1)
  auto res3 = get_utf8_codepoint(mixed_str, 3);  // 你 (pos=1+2=3)
  auto res4 = get_utf8_codepoint(mixed_str, 6);  // 😊 (pos=3+3=6)

  EXPECT_TRUE(res1.is_ok());
  EXPECT_TRUE(res2.is_ok());
  EXPECT_TRUE(res3.is_ok());
  EXPECT_TRUE(res4.is_ok());

  EXPECT_EQ(res1.unwrap(), std::make_pair(U'A', 1U));
  EXPECT_EQ(res2.unwrap(), std::make_pair(U'á', 2U));
  EXPECT_EQ(res3.unwrap(), std::make_pair(U'你', 3U));
  EXPECT_EQ(res4.unwrap(), std::make_pair(U'😊', 4U));
}

// 测试无效位置（失败场景）
TEST(UTF8DecoderTest, InvalidPositions) {
  // 用例1：pos超出字符串长度（"test"长度4，pos=10）
  auto res1 = get_utf8_codepoint("test", 10);
  EXPECT_TRUE(res1.is_err());
  EXPECT_EQ(res1.unwrap_err(), Utils::Utf8::Error::InvalidPosition);

  // 用例2：空字符串（pos=0）
  auto res2 = get_utf8_codepoint("", 0);
  EXPECT_TRUE(res2.is_err());
  EXPECT_EQ(res2.unwrap_err(), Utils::Utf8::Error::InvalidPosition);

  // 用例3：pos等于字符串长度（"a"长度1，pos=1）
  auto res3 = get_utf8_codepoint("a", 1);
  EXPECT_TRUE(res3.is_err());
  EXPECT_EQ(res3.unwrap_err(), Utils::Utf8::Error::InvalidPosition);
}

// 测试不完整的多字节序列（失败场景）
TEST(UTF8DecoderTest, IncompleteSequences) {
  // 用例1：双字节序列缺续字节（"\xC3" → 应补1个续字节）
  auto res1 = get_utf8_codepoint("\xC3", 0);
  EXPECT_TRUE(res1.is_err());
  EXPECT_EQ(res1.unwrap_err(), Utils::Utf8::Error::IncompleteSequence);

  // 用例2：三字节序列缺1个续字节（"\xE4\xBD" → 应补1个续字节）
  auto res2 = get_utf8_codepoint("\xE4\xBD", 0);
  EXPECT_TRUE(res2.is_err());
  EXPECT_EQ(res2.unwrap_err(), Utils::Utf8::Error::IncompleteSequence);

  // 用例3：四字节序列缺1个续字节（"\xF0\x9F\x98" → 应补1个续字节）
  auto res3 = get_utf8_codepoint("\xF0\x9F\x98", 0);
  EXPECT_TRUE(res3.is_err());
  EXPECT_EQ(res3.unwrap_err(), Utils::Utf8::Error::IncompleteSequence);
}

// 测试无效的UTF-8序列（失败场景）
TEST(UTF8DecoderTest, InvalidSequences) {
  // --------------- 无效续字节 ---------------
  // 用例1：双字节序列续字节非"10xxxxxx"（"\xC3\xC3" → 第二个字节是首字节格式）
  auto res1 = get_utf8_codepoint("\xC3\xC3", 0);
  EXPECT_TRUE(res1.is_err());
  EXPECT_EQ(res1.unwrap_err(), Utils::Utf8::Error::InvalidContinuation);

  // 用例2：三字节序列第二个字节无效（"\xE4\xC3\xA1" → 第二个字节是首字节格式）
  auto res2 = get_utf8_codepoint("\xE4\xC3\xA1", 0);
  EXPECT_TRUE(res2.is_err());
  EXPECT_EQ(res2.unwrap_err(), Utils::Utf8::Error::InvalidContinuation);

  // --------------- 无效首字节 ---------------
  // 用例3：首字节为续字节格式（0x80~0xBF → 不能作为首字节）
  auto res3 = get_utf8_codepoint("\x80", 0);
  auto res4 = get_utf8_codepoint("\xBF", 0);
  EXPECT_TRUE(res3.is_err());
  EXPECT_TRUE(res4.is_err());
  EXPECT_EQ(res3.unwrap_err(), Utils::Utf8::Error::InvalidLeadingByte);
  EXPECT_EQ(res4.unwrap_err(), Utils::Utf8::Error::InvalidLeadingByte);

  // 用例4：首字节超出UTF-8范围（0xF8~0xFF →
  // 最多4字节，首字节最高位只能是0/110/1110/11110）
  auto res5 = get_utf8_codepoint("\xF8", 0);
  auto res6 = get_utf8_codepoint("\xFF", 0);
  EXPECT_TRUE(res5.is_err());
  EXPECT_TRUE(res6.is_err());
  EXPECT_EQ(res5.unwrap_err(), Utils::Utf8::Error::InvalidLeadingByte);
  EXPECT_EQ(res6.unwrap_err(), Utils::Utf8::Error::InvalidLeadingByte);
}

// 测试过度编码（UTF-8明确禁止，失败场景）
TEST(UTF8DecoderTest, OverlongEncoding) {
  // 核心规则：码点必须用最短字节数表示（1字节→0~7F，2→80~7FF，3→800~FFFF，4→10000~10FFFF）

  // --------------- 3字节表示1/2字节码点 ---------------
  // 用例1：3字节表示1字节码点（0x00 → 合法应为0x00，非法为"\xE0\x80\x80"）
  auto res1 = get_utf8_codepoint("\xE0\x80\x80", 0);
  EXPECT_TRUE(res1.is_err());
  EXPECT_EQ(res1.unwrap_err(), Utils::Utf8::Error::OverlongEncoding);

  // 用例2：3字节表示1字节最大值（0x7F → 合法应为0x7F，非法为"\xE0\x80\x7F"）
  auto res2 = get_utf8_codepoint("\xE0\x80\x7F", 0);
  EXPECT_TRUE(res2.is_err());
  EXPECT_EQ(res2.unwrap_err(), Utils::Utf8::Error::InvalidContinuation);

  // 用例3：3字节表示2字节最小值（0x80 →
  // 合法应为"\xC2\x80"，非法为"\xE0\x80\x80"）
  auto res3 = get_utf8_codepoint("\xE0\x80\x80", 0);
  EXPECT_TRUE(res3.is_err());
  EXPECT_EQ(res3.unwrap_err(), Utils::Utf8::Error::OverlongEncoding);

  // 用例4：3字节表示2字节最大值（0x7FF →
  // 合法应为"\xDF\xBF"，非法为"\xE0\x9F\xBF"）
  auto res4 = get_utf8_codepoint("\xE0\x9F\xBF", 0);
  EXPECT_TRUE(res4.is_err());
  EXPECT_EQ(res4.unwrap_err(), Utils::Utf8::Error::OverlongEncoding);

  // --------------- 4字节表示1/2/3字节码点 ---------------
  // 用例5：4字节表示1字节码点（0x00 → 非法为"\xF0\x80\x80\x80"）
  auto res5 = get_utf8_codepoint("\xF0\x80\x80\x80", 0);
  EXPECT_TRUE(res5.is_err());
  EXPECT_EQ(res5.unwrap_err(), Utils::Utf8::Error::OverlongEncoding);

  // 用例6：4字节表示2字节最大值（0x7FF → 非法为"\xF0\x80\x9F\xBF"）
  auto res6 = get_utf8_codepoint("\xF0\x80\x9F\xBF", 0);
  EXPECT_TRUE(res6.is_err());
  EXPECT_EQ(res6.unwrap_err(), Utils::Utf8::Error::OverlongEncoding);

  // 用例7：4字节表示3字节最小值（0x800 →
  // 合法应为"\xE0\xA0\x80"，非法为"\xF0\x80\xA0\x80"）
  auto res7 = get_utf8_codepoint("\xF0\x80\xA0\x80", 0);
  EXPECT_TRUE(res7.is_err());
  EXPECT_EQ(res7.unwrap_err(), Utils::Utf8::Error::OverlongEncoding);

  // 用例8：4字节表示3字节最大值（0xFFFF →
  // 合法应为"\xEF\xBF\xBF"，非法为"\xF0\x8F\xBF\xBF"）
  auto res8 = get_utf8_codepoint("\xF0\x8F\xBF\xBF", 0);
  EXPECT_TRUE(res8.is_err());
  EXPECT_EQ(res8.unwrap_err(), Utils::Utf8::Error::OverlongEncoding);

  // --------------- 码点超出Unicode上限（附加场景） ---------------
  // 用例9：码点0x110000（超出0x10FFFF，非法）
  auto res9 = get_utf8_codepoint(std::string_view("\xF4\x90\x80\x80", 4), 0);
  EXPECT_TRUE(res9.is_err());
  EXPECT_EQ(res9.unwrap_err(), Utils::Utf8::Error::InvalidCodePoint);
}