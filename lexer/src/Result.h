#pragma once
#include <stdexcept>
#include <utility>
#include <variant>

namespace Result {

template <typename T>
struct OkIntermediate;

template <typename E>
struct ErrIntermediate;

// 概念：检查类型I是否为OkIntermediate<T>
template <typename I, typename T>
concept OkIntermediateFor = std::is_same_v<I, OkIntermediate<T>>;

// 概念：检查类型I是否为ErrIntermediate<E>
template <typename I, typename E>
concept ErrIntermediateFor = std::is_same_v<I, ErrIntermediate<E>>;

// 概念：检查类型是否为void
template <typename T>
concept IsVoid = std::is_void_v<T>;

// 概念：检查类型是否为非void
template <typename T>
concept IsNonVoid = !IsVoid<T>;

// 中间态标签类型 - Ok专用
template <typename T>
struct OkIntermediate {
  explicit OkIntermediate(T value) : value(std::move(value)) {}
  auto get() const -> const T& { return value; }
  auto get() -> T& { return value; }

 private:
  T value;
};

// 中间态标签特化 - 处理void类型（无需存储值）
template <>
struct OkIntermediate<void> {
  OkIntermediate() = default;  // 无参数构造（因为void没有值）
};

// 中间态标签类型 - Err专用
template <typename E>
struct ErrIntermediate {
  explicit ErrIntermediate(E error) : error(std::move(error)) {}
  auto get() const -> const E& { return error; }
  auto get() -> E& { return error; }

 private:
  E error;
};

// 工厂函数 - 返回Ok中间态（非void）
template <typename T>
auto Ok(T&& value) {
  return OkIntermediate<std::decay_t<T>>(std::forward<T>(value));
}

// 工厂函数 - void版本（无值）
inline auto Ok() {
  return OkIntermediate<void>();
}

// 工厂函数 - 返回Err中间态
template <typename E>
auto Err(E&& error) {
  return ErrIntermediate<std::decay_t<E>>(std::forward<E>(error));
}

template <typename U, typename E>
concept IsNestedResult = requires {
  // 检查 U 是否是 Result<UInner, E> 的实例（错误类型必须为 E）
  typename U::ErrorType;  // 需为 Result 新增 ErrorType 别名，暴露错误类型
  std::is_same_v<typename U::ErrorType, E>;  // 内层错误类型 ≡ 外层错误类型
};

template <typename T, typename E>
class Result {
 private:
  struct OkVoid {};  // T为void时的Ok状态（空结构体）
  struct OkWithValue {
    T value;
  };  // T非void时的Ok状态（带值）

  using OkValue = std::conditional_t<IsVoid<T>, OkVoid, OkWithValue>;
  struct ErrValue {
    E error;
  };
  std::variant<OkValue, ErrValue> m_data;

 public:
  // 从Ok中间态构造（非void T）
  template <OkIntermediateFor<T> OkType>
  // NOLINTNEXTLINE(google-explicit-constructor)
  Result(OkType ok)
    requires IsNonVoid<T>
    : m_data(OkValue{std::move(ok.get())}) {}

  // 从Ok中间态构造（void T）
  template <OkIntermediateFor<T> OkType>
  // NOLINTNEXTLINE(google-explicit-constructor)
  Result(OkType /*unused*/)
    requires IsVoid<T>
    : m_data(OkValue{}) {}

  // 从Err中间态构造
  template <ErrIntermediateFor<E> ErrType>
  // NOLINTNEXTLINE(google-explicit-constructor)
  Result(ErrType err) : m_data(ErrValue{std::move(err.get())}) {}

  // 禁用移动和复制语义
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

  template <typename U = T>
  [[nodiscard]] auto unwrap() const -> const U&
    requires IsNonVoid<U>
  {
    if (const auto* ok_val = std::get_if<OkValue>(&m_data)) {
      return ok_val->value;
    }
    throw std::runtime_error("Called unwrap() on Err");
  }

  // T为void时的版本，无返回值
  auto unwrap() const -> void
    requires IsVoid<T>
  {
    if (!is_ok()) {
      throw std::runtime_error("Called unwrap() on Err");
    }
  }

  [[nodiscard]] auto unwrap_err() const -> const E& {
    if (const auto* err_val = std::get_if<ErrValue>(&m_data)) {
      return err_val->error;
    }
    throw std::runtime_error("Called unwrap_err() on Ok");
  }

  template <typename U = T>
  [[nodiscard]] auto expect(const std::string& msg) const -> const U&
    requires IsNonVoid<U>
  {
    if (is_ok()) {
      return unwrap();
    }
    throw std::runtime_error(msg);
  }

  // T为void时的版本，无返回值
  auto expect(const std::string& msg) const -> void
    requires IsVoid<T>
  {
    if (!is_ok()) {
      throw std::runtime_error(msg);
    }
  }

  template <typename F, typename U = std::invoke_result_t<F, T>>
  auto map(F&& f) const -> Result<U, E>
    requires IsNonVoid<U>
  {
    if (is_ok()) {
      return Ok(std::invoke(std::forward<F>(f), unwrap()));
    }
    return Err(unwrap_err());
  }

  using OkType = std::conditional_t<IsVoid<T>, void, T>;
  using ErrorType = E;
  template <typename U = T>
  [[nodiscard]] auto flatten() const -> Result<typename U::OkType, E>
    requires IsNestedResult<U, E>
  {
    if (is_err()) {
      return Err(unwrap_err());
    }
    const auto& inner_result = unwrap();
    if (inner_result.is_ok()) {
      return Ok(inner_result.unwrap());
    }
    return Err(inner_result.unwrap_err());
  }

  template <typename U = T>
  [[nodiscard]] auto flatten() const -> Result<void, E>
    requires IsVoid<U> && IsNestedResult<U, E>
  {
    static_assert(false, "Result<void, E> 无需 flatten（无嵌套 Ok 值）");
  }

  template <typename F, typename U = std::invoke_result_t<F, T>>
  [[nodiscard]] auto and_then(F&& f) const -> Result<
    typename U::OkType,
    E>
    requires(
      IsNestedResult<U, E> &&
      std::is_invocable_v<F, T> 
    )
  {
    if (is_err()) {
      return Err(unwrap_err());
    }
    return std::invoke(std::forward<F>(f), unwrap());
  }
};

}  // namespace Result
