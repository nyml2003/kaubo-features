#pragma once

namespace utils {

template <typename T, typename E>
class Result;

// 在result.h中，utils命名空间内
template <typename R>
struct result_value_type;  // 未定义，用于触发编译错误

// 特化：当R是Result<U, E>时，萃取U
template <typename T, typename E>
struct result_value_type<utils::Result<T, E>> {
  using okType = T;
  using errorType = E;
};

// 辅助别名，简化使用
template <typename R>
using result_value_t = typename result_value_type<R>::okType;

template <typename R>
using result_error_t = typename result_value_type<R>::errorType;

}  // namespace utils