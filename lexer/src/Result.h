#pragma once
#include <stdexcept>
#include <string>
#include <type_traits>
#include <utility>
#include <variant>

namespace Result {

// 前向声明 - 明确需要两个模板参数
template <typename T, typename E>
class Result;

// Ok构造函数的辅助函数 - 明确模板参数顺序：T是成功类型，E是错误类型
template <typename T, typename E>
auto Ok(T&& value) {
  return Result<T, E>(std::in_place_index<0>, std::forward<T>(value));
}

// 无值的Ok构造函数辅助函数（主要用于T为void的情况）
template <typename T, typename E>
auto Ok() {
  return Result<T, E>(std::in_place_index<0>);
}

// Err构造函数的辅助函数 - 明确模板参数顺序：T是成功类型，E是错误类型
template <typename T, typename E>
auto Err(E&& error) {
  return Result<T, E>(std::in_place_index<1>, std::forward<E>(error));
}

// 无值的Err构造函数辅助函数
template <typename T, typename E>
auto Err() {
  return Result<T, E>(std::in_place_index<1>);
}

// 明确需要两个模板参数：T(成功类型)和E(错误类型)
template <typename T, typename E>
class Result {
 private:
  std::variant<T, E> m_data;

  // 私有构造函数，只能通过Ok()和Err()辅助函数创建
  template <typename... Args>
  explicit Result(std::in_place_index_t<0> /**/, Args&&... args)
    : m_data(std::in_place_index<0>, std::forward<Args>(args)...) {}

  template <typename... Args>
  explicit Result(std::in_place_index_t<1> /**/, Args&&... args)
    : m_data(std::in_place_index<1>, std::forward<Args>(args)...) {}

 public:
  // 移动构造函数
  Result(
    Result&& other
  ) noexcept(std::is_nothrow_move_constructible_v<std::variant<T, E>>)
    : m_data(std::move(other.m_data)) {}

  // 禁止复制构造和赋值
  Result(const Result&) = delete;
  auto operator=(const Result&) -> Result& = delete;
  auto operator=(Result&&) -> Result& = delete;

  // 析构函数
  ~Result() = default;

  // 检查是否为Ok
  [[nodiscard]] auto is_ok() const noexcept -> bool {
    return std::holds_alternative<T>(m_data);
  }

  // 检查是否为Err
  [[nodiscard]] auto is_err() const noexcept -> bool {
    return std::holds_alternative<E>(m_data);
  }

  // 获取Ok值，如果是Err则抛出异常
  auto unwrap() & -> T& {
    if (is_ok()) {
      return std::get<T>(m_data);
    }
    throw std::runtime_error("Called unwrap() on an Err");
  }

  auto unwrap() const& -> const T& {
    if (is_ok()) {
      return std::get<T>(m_data);
    }
    throw std::runtime_error("Called unwrap() on an Err");
  }

  auto unwrap() && -> T&& {
    if (is_ok()) {
      return std::get<T>(std::move(m_data));
    }
    throw std::runtime_error("Called unwrap() on an Err");
  }

  // 获取Ok值，如果是Err则抛出带有自定义信息的异常
  auto expect(const std::string& msg) & -> T& {
    if (is_ok()) {
      return std::get<T>(m_data);
    }
    throw std::runtime_error(msg);
  }

  // 获取Err值，如果是Ok则抛出异常
  auto unwrap_err() & -> E& {
    if (is_err()) {
      return std::get<E>(m_data);
    }
    throw std::runtime_error("Called unwrap_err() on an Ok");
  }

  auto unwrap_err() const& -> const E& {
    if (is_err()) {
      return std::get<E>(m_data);
    }
    throw std::runtime_error("Called unwrap_err() on an Ok");
  }

  // 应用一个函数到Ok值上
  template <typename F>
  auto map(F&& func) & {
    using ReturnType = std::invoke_result_t<F, T&>;
    if (is_ok()) {
      return Ok<ReturnType, E>(std::forward<F>(func)(std::get<T>(m_data)));
    }
    return Err<ReturnType, E>(std::get<E>(m_data));
  }

  // 友元函数声明 - 与辅助函数模板参数匹配
  template <typename U, typename F>
  friend auto Ok(U&& value);

  template <typename U, typename F>
  friend auto Ok();

  template <typename U, typename F>
  friend auto Err(F&& error);

  template <typename U, typename F>
  friend auto Err();
};

// T为void的特化版本
template <typename E>
class Result<void, E> {
 private:
  // 对于void类型，我们只需要知道是成功还是失败，成功时不需要存储值
  std::variant<std::monostate, E> m_data;

  // 私有构造函数
  template <typename... Args>
  explicit Result(std::in_place_index_t<0> /**/, Args&&... args)
    : m_data(std::in_place_index<0>, std::forward<Args>(args)...) {}

  template <typename... Args>
  explicit Result(std::in_place_index_t<1> /**/, Args&&... args)
    : m_data(std::in_place_index<1>, std::forward<Args>(args)...) {}

 public:
  // 移动构造函数
  Result(Result&& other) noexcept(
    std::is_nothrow_move_constructible_v<std::variant<std::monostate, E>>
  )
    : m_data(std::move(other.m_data)) {}

  // 禁止复制构造和赋值
  Result(const Result&) = delete;
  auto operator=(const Result&) -> Result& = delete;
  auto operator=(Result&&) -> Result& = delete;

  // 析构函数
  ~Result() = default;

  // 检查是否为Ok
  [[nodiscard]] auto is_ok() const noexcept -> bool {
    return std::holds_alternative<std::monostate>(m_data);
  }

  // 检查是否为Err
  [[nodiscard]] auto is_err() const noexcept -> bool {
    return std::holds_alternative<E>(m_data);
  }

  // 对于void类型，unwrap不返回值，只检查是否为Ok
  auto unwrap() -> void {
    if (is_err()) {
      throw std::runtime_error("Called unwrap() on an Err");
    }
  }

  // 对于void类型，expect不返回值，只检查是否为Ok
  auto expect(const std::string& msg) -> void {
    if (is_err()) {
      throw std::runtime_error(msg);
    }
  }

  // 获取Err值，如果是Ok则抛出异常
  auto unwrap_err() & -> E& {
    if (is_err()) {
      return std::get<E>(m_data);
    }
    throw std::runtime_error("Called unwrap_err() on an Ok");
  }

  auto unwrap_err() const& -> const E& {
    if (is_err()) {
      return std::get<E>(m_data);
    }
    throw std::runtime_error("Called unwrap_err() on an Ok");
  }

  // 应用一个函数到Ok状态上（函数无参数，返回值作为新的Result的T类型）
  template <typename F>
  auto map(F&& func) & {
    using ReturnType = std::invoke_result_t<F>;
    if (is_ok()) {
      return Ok<ReturnType, E>(std::forward<F>(func)());
    }
    return Err<ReturnType, E>(std::get<E>(m_data));
  }

  // 友元函数声明
  template <typename U, typename F>
  friend auto Ok(U&& value);

  template <typename U, typename F>
  friend auto Ok();

  template <typename U, typename F>
  friend auto Err(F&& error);

  template <typename U, typename F>
  friend auto Err();
};

}  // namespace Result
