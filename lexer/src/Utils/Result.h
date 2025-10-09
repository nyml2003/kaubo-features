#pragma once

#include <cassert>
#include <type_traits>
#include <utility>
#include <variant>
#include "ResultHelper.h"

namespace utils {

template <typename T>
concept is_pod_like = std::is_trivial_v<T> && std::is_standard_layout_v<T>;

template <typename T>
concept is_cheap_to_copy = is_pod_like<T>;

template <typename T>
struct OkIntermediate;

template <typename E>
struct ErrIntermediate;

template <typename I, typename T>
concept OkIntermediateFor = std::is_same_v<I, OkIntermediate<T>>;

template <typename I, typename E>
concept ErrIntermediateFor = std::is_same_v<I, ErrIntermediate<E>>;

template <typename T>
struct OkIntermediate final {
  T value;
};

template <>
struct OkIntermediate<void> final {
  OkIntermediate() = default;
};

template <typename E>
struct ErrIntermediate final {
  E error;
};

template <>
struct ErrIntermediate<void> final {
  ErrIntermediate() = default;
};

template <typename T>
auto ok(T&& value) {
  return OkIntermediate<std::decay_t<T>>(std::forward<T>(value));
}

inline auto ok() {
  return OkIntermediate<void>();
}

template <typename E>
auto err(E&& error) {
  return ErrIntermediate<std::decay_t<E>>(std::forward<E>(error));
}

inline auto err() {
  return ErrIntermediate<void>();
}

template <typename T, typename E>
class Result final {
 private:
  struct OkVoid {};
  struct OkWithValue {
    T value;
  };

  using OkValue = std::conditional_t<std::is_void_v<T>, OkVoid, OkWithValue>;
  struct ErrVoid {};
  struct ErrWithValue {
    E error;
  };
  using ErrValue = std::conditional_t<std::is_void_v<E>, ErrVoid, ErrWithValue>;
  std::variant<OkValue, ErrValue> m_data;

  template <typename U, typename P = T>
  auto forward() const noexcept -> Result<P, U>
    requires(!std::is_void_v<P>)
  {
    return ok(unwrap());
  }

  // 针对void的E类型
  template <typename U, typename P = T>
  auto forward() const noexcept -> Result<P, U>
    requires(std::is_void_v<P>)
  {
    return ok();
  }

  template <typename U, typename Q = E>
  auto forward_err() const noexcept -> Result<U, Q>
    requires(!std::is_void_v<Q>)
  {
    return err(unwrap_err());
  }

  // 针对void的E类型
  template <typename U, typename Q = E>
  auto forward_err() const noexcept -> Result<U, Q>
    requires(std::is_void_v<Q>)
  {
    return err();
  }

 public:
  template <OkIntermediateFor<T> OkType>
  // NOLINTNEXTLINE(google-explicit-constructor)
  Result(OkType ok)
    requires(!std::is_void_v<T>)
    : m_data(OkValue{.value = std::move(ok.value)}) {}

  template <OkIntermediateFor<T> OkType>
  // NOLINTNEXTLINE(google-explicit-constructor)
  Result(OkType /*unused*/)
    requires std::is_void_v<T>
    : m_data(OkValue{}) {}

  template <ErrIntermediateFor<E> ErrType>
  // NOLINTNEXTLINE(google-explicit-constructor)
  Result(ErrType err)
    requires(!std::is_void_v<E>)
    : m_data(ErrValue{.error = std::move(err.error)}) {}

  template <ErrIntermediateFor<E> ErrType>
  // NOLINTNEXTLINE(google-explicit-constructor)
  Result(ErrType /*unused*/)
    requires std::is_void_v<E>
    : m_data(ErrValue{}) {}

  Result(Result&&) = default;
  auto operator=(Result&&) -> Result& = delete;
  Result(const Result&) = delete;
  auto operator=(const Result&) -> Result& = delete;

  ~Result() = default;

  [[nodiscard]] auto is_ok() const noexcept -> bool {
    return std::holds_alternative<OkValue>(m_data);
  }

  [[nodiscard]] auto is_err() const noexcept -> bool {
    return std::holds_alternative<ErrValue>(m_data);
  }

  explicit operator bool() const noexcept { return is_ok(); }

  /**
   * @brief 返回Ok值，如果是Err则崩溃
   * @details 使用前请确保is_ok()为true
   */
  auto unwrap() const noexcept -> void
    requires std::is_void_v<T>
  {
    assert(is_ok() && "unwrap() called on Err");
  }

  template <typename U = T>
  [[nodiscard]] auto unwrap() const noexcept
    -> std::conditional_t<is_cheap_to_copy<U>, U, const U&>
    requires(!std::is_void_v<T>)
  {
    assert(is_ok() && "unwrap() called on Err");
    return std::get_if<OkValue>(&m_data)->value;
  }

  /**
   * @brief 返回Err值，如果是Ok则崩溃
   * @details 使用前请确保is_err()为true
   */
  auto unwrap_err() const noexcept -> void
    requires std::is_void_v<E>
  {
    assert(is_err() && "unwrap_err() called on Ok");
  }

  template <typename U = E>
  [[nodiscard]] auto unwrap_err() const noexcept -> const U&
    requires(!std::is_void_v<U>)
  {
    assert(is_err() && "unwrap_err() called on Ok");
    return std::get_if<ErrValue>(&m_data)->error;
  }

  /**
   * @brief 如果is_ok()为true则调用f拿到T, 和原来的E一起返回一个新的Result
   * @tparam Function: (P | const P&) -> ReturnType
   * 应该是一个简单的函数, 在处理后重新包装成 Result<ReturnType, E>
   * @tparam P T的别名，不知道为什么要加这个别名
   * @tparam ReturnType Function的返回类型
   * @tparam std::conditional_t<is_cheap_to_copy<P>, P, const P&>>
   */
  template <
    typename Function,
    typename P = T,
    typename Arg = std::conditional_t<is_cheap_to_copy<P>, P, const P&>,
    typename ReturnType = std::invoke_result_t<Function, Arg>>
  [[nodiscard]] auto map(const Function& f) const noexcept
    -> Result<ReturnType, E>
    requires(!std::is_void_v<P> && std::is_nothrow_invocable_v<Function, Arg>)
  {
    if (is_ok()) {
      return ok(f(unwrap()));  // 包装一个ok，类型是ReturnType
    }
    return forward_err<ReturnType>();
  }

  /**
   * @brief 如果is_ok()为true则调用f拿到T, 和原来的E一起返回一个新的Result
   * @tparam Function: () -> ReturnType, Function
   * 应该是一个简单的函数, 在处理后重新包装成 Result<ReturnType, E>
   * @tparam P T的别名，不知道为什么要加这个别名
   * @tparam ReturnType Function的返回类型
   */
  template <
    typename Function,
    typename P = T,
    typename ReturnType = std::invoke_result_t<Function>>
  [[nodiscard]] auto map(const Function& f) const noexcept
    -> Result<ReturnType, E>
    requires(std::is_void_v<P> && std::is_nothrow_invocable_v<Function>)
  {
    if (is_ok()) {
      return ok(f());  // 包装一个ok，类型是ReturnType
    }
    return forward_err<ReturnType>();
  }

  template <
    typename Function,
    typename P = T,
    typename Arg = std::conditional_t<is_cheap_to_copy<P>, P, const P&>,
    typename ReturnType = std::invoke_result_t<Function, Arg>>
  [[nodiscard]] auto and_then(const Function& f) const noexcept -> ReturnType
    requires(
      !std::is_void_v<P> && std::is_nothrow_invocable_v<Function, Arg> &&
      std::is_same_v<ReturnType, Result<result_value_t<ReturnType>, E>>
    )
  {
    if (is_ok()) {
      return f(unwrap());
    }
    return forward_err<result_value_t<ReturnType>>();
  }

  template <
    typename Function,
    typename P = T,
    typename ReturnType = std::invoke_result_t<Function>>
  [[nodiscard]] auto and_then(const Function& f) const noexcept -> ReturnType
    requires(
      std::is_void_v<P> && std::is_nothrow_invocable_v<Function> &&
      std::is_same_v<ReturnType, Result<result_value_t<ReturnType>, E>>
    )
  {
    if (is_ok()) {
      return f();
    }
    return forward_err<result_value_t<ReturnType>>();
  }

  /**
   * @brief
   * 如果is_err()为true则调用f处理错误值E，返回包含原T和新错误类型的Result
   * @tparam Function: (E | const E&) -> NewE，用于处理错误的函数
   * @tparam Q E的别名，用于模板参数推导
   * @tparam NewE Function的返回类型，即新的错误类型
   */
  template <
    typename Function,
    typename Q = E,
    typename Arg = std::conditional_t<is_cheap_to_copy<Q>, Q, const Q&>,
    typename NewE = std::invoke_result_t<Function, Arg>>
  [[nodiscard]] auto map_err(const Function& f) const noexcept
    -> Result<T, NewE>
    requires(!std::is_void_v<Q> && std::is_nothrow_invocable_v<Function, Arg>)
  {
    if (is_err()) {
      return err(f(unwrap_err()));  // 处理错误值并包装为新的错误类型
    }
    return forward<NewE>();  // 转发正确值，错误类型变为NewE
  }

  /**
   * @brief
   * 如果is_err()为true则调用f处理无值错误，返回包含原T和新错误类型的Result
   * @tparam Function: () -> NewE，无参数的错误处理函数
   * @tparam Q E的别名，用于模板参数推导
   * @tparam NewE Function的返回类型，即新的错误类型
   */
  template <
    typename Function,
    typename Q = E,
    typename NewE = std::invoke_result_t<Function>>
  [[nodiscard]] auto map_err(const Function& f) const noexcept
    -> Result<T, NewE>
    requires(std::is_void_v<Q> && std::is_nothrow_invocable_v<Function>)
  {
    if (is_err()) {
      return err(f());  // 处理无值错误并包装为新的错误类型
    }
    return forward<NewE>();  // 转发正确值，错误类型变为NewE
  }

  /**
   * @brief
   * 如果is_err()为true则调用f处理错误值E，返回新的Result；如果is_ok()则直接返回原Ok值
   * @tparam Function: (E | const E&) -> Result<T, F>
   *         函数接收错误类型E，返回新的Result（成功类型保持T，错误类型可变为F）
   * @tparam P E的别名，用于处理参数传递方式（值或引用）
   * @tparam Arg 实际传递给函数的参数类型（根据是否易拷贝决定值传递或引用）
   * @tparam ReturnType 函数返回的Result类型（Result<T, F>）
   */
  template <
    typename Function,
    typename P = E,
    typename Arg = std::conditional_t<is_cheap_to_copy<P>, P, const P&>,
    typename ReturnType = std::invoke_result_t<Function, Arg>>
  [[nodiscard]] auto or_else(const Function& f) const noexcept -> ReturnType
    requires(
      !std::is_void_v<P> && std::is_nothrow_invocable_v<Function, Arg> &&
      std::is_same_v<
        ReturnType,
        Result<
          T,
          result_error_t<ReturnType>>>  // 确保返回Result<T, F>
    )
  {
    if (is_err()) {
      return f(unwrap_err());  // 调用函数处理错误值，返回新的Result
    }
    return forward<result_error_t<ReturnType>>();  // 转发原Ok值
  }

  /**
   * @brief 针对E为void的重载：is_err()为true时调用无参函数f，返回新的Result
   * @tparam Function: () -> Result<T, F>
   *         无参函数，返回新的Result（成功类型保持T，错误类型可变为F）
   */
  template <
    typename Function,
    typename P = E,
    typename ReturnType = std::invoke_result_t<Function>>
  [[nodiscard]] auto or_else(const Function& f) const noexcept -> ReturnType
    requires(
      std::is_void_v<P> && std::is_nothrow_invocable_v<Function> &&
      std::is_same_v<
        ReturnType,
        Result<
          T,
          result_error_t<ReturnType>>>  // 确保返回Result<T,
                                        // F>
    )
  {
    if (is_err()) {
      return f();  // 调用无参函数处理错误，返回新的Result
    }
    return forward<result_error_t<ReturnType>>();
  }
};

}  // namespace utils
