#pragma once
#include <cstdint>
#include <string>
#include <type_traits>
namespace Lexer::Token {

// 定义枚举约束概念
template <typename T>
concept Constraint = std::is_enum_v<T> &&  // 必须是枚举类型
                     std::is_same_v<
                       std::underlying_type_t<T>,
                       uint8_t> &&    // 检查底层类型是否为uint8_t
                     requires(T t) {  // 支持std::to_string
                       { to_string(t) } -> std::same_as<std::string>;
                     };
}  // namespace Lexer::Token

namespace std {}  // namespace std
