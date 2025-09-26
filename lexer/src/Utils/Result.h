#pragma once
#include <stdexcept>
#include <utility>
#include <variant>

namespace Utils {

template <typename T>
struct OkIntermediate;

template <typename E>
struct ErrIntermediate;

template <typename I, typename T>
concept OkIntermediateFor = std::is_same_v<I, OkIntermediate<T>>;

template <typename I, typename E>
concept ErrIntermediateFor = std::is_same_v<I, ErrIntermediate<E>>;

template <typename T>
concept IsVoid = std::is_void_v<T>;

template <typename T>
concept IsNonVoid = !IsVoid<T>;

template <typename T>
struct OkIntermediate {
  explicit OkIntermediate(T value) : value(std::move(value)) {}
  auto get() const -> const T& { return value; }
  auto get() -> T& { return value; }

 private:
  T value;
};

template <>
struct OkIntermediate<void> {
  OkIntermediate() = default;
};

template <typename E>
struct ErrIntermediate {
  explicit ErrIntermediate(E error) : error(std::move(error)) {}
  auto get() const -> const E& { return error; }
  auto get() -> E& { return error; }

 private:
  E error;
};

template <typename T>
auto Ok(T&& value) {
  return OkIntermediate<std::decay_t<T>>(std::forward<T>(value));
}

inline auto Ok() {
  return OkIntermediate<void>();
}

template <typename E>
auto Err(E&& error) {
  return ErrIntermediate<std::decay_t<E>>(std::forward<E>(error));
}

template <typename U, typename E>
concept IsNestedResult = requires {
  typename U::ErrorType;
  std::is_same_v<typename U::ErrorType, E>;
};

template <typename T, typename E>
class Result {
 private:
  struct OkVoid {};
  struct OkWithValue {
    T value;
  };

  using OkValue = std::conditional_t<IsVoid<T>, OkVoid, OkWithValue>;
  struct ErrValue {
    E error;
  };
  std::variant<OkValue, ErrValue> m_data;

 public:
  template <OkIntermediateFor<T> OkType>
  // NOLINTNEXTLINE(google-explicit-constructor)
  Result(OkType ok)
    requires IsNonVoid<T>
    : m_data(OkValue{std::move(ok.get())}) {}

  template <OkIntermediateFor<T> OkType>
  // NOLINTNEXTLINE(google-explicit-constructor)
  Result(OkType /*unused*/)
    requires IsVoid<T>
    : m_data(OkValue{}) {}

  template <ErrIntermediateFor<E> ErrType>
  // NOLINTNEXTLINE(google-explicit-constructor)
  Result(ErrType err) : m_data(ErrValue{std::move(err.get())}) {}

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

  template <typename U = T>
  [[nodiscard]] auto unwrap() && -> U&&  // 注意这里的 &&：仅能在右值对象上调用
    requires IsNonVoid<U>
  {
    if (auto* ok_val = std::get_if<OkValue>(&m_data)) {
      return std::move(ok_val->value);  // 移动内部值的所有权
    }
    throw std::runtime_error("Called unwrap() on Err");
  }

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
  {}

  template <typename F, typename U = std::invoke_result_t<F, T>>
  [[nodiscard]] auto and_then(F&& f) const -> Result<typename U::OkType, E>
    requires(IsNestedResult<U, E> && std::is_invocable_v<F, T>)
  {
    if (is_err()) {
      return Err(unwrap_err());
    }
    return std::invoke(std::forward<F>(f), unwrap());
  }
};

}  // namespace Utils
