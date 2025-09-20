#pragma once

#include <mutex>
#include <string>

namespace Utils {

class StringBuilder {
 private:
  std::string m_buffer;        // 缓冲区
  mutable std::mutex m_mutex;  // 互斥锁，mutable允许在const成员函数中锁定

 public:
  // 构造函数，可指定初始容量
  explicit StringBuilder(size_t initial_capacity = 0) {
    if (initial_capacity > 0) {
      m_buffer.reserve(initial_capacity);
    }
  }
  ~StringBuilder() = default;

  // 拷贝构造函数
  StringBuilder(const StringBuilder& other) = delete;

  // 移动构造函数
  StringBuilder(StringBuilder&& other) noexcept
    : m_buffer(std::move(other.m_buffer)) {
    std::lock_guard<std::mutex> lock(other.m_mutex);
  }

  // 拷贝赋值运算符
  auto operator=(const StringBuilder& other) = delete;

  // 移动赋值运算符
  auto operator=(StringBuilder&& other) noexcept -> StringBuilder& {
    if (this != &other) {
      std::scoped_lock lock(m_mutex, other.m_mutex);
      m_buffer = std::move(other.m_buffer);
    }
    return *this;
  }

  // 拼接C风格字符串
  auto append(const char* str) -> StringBuilder& {
    if (str != nullptr) {
      std::lock_guard<std::mutex> lock(m_mutex);
      m_buffer += str;
    }
    return *this;
  }

  // 拼接std::string
  auto append(const std::string& str) -> StringBuilder& {
    std::lock_guard<std::mutex> lock(m_mutex);
    m_buffer += str;
    return *this;
  }

  // 拼接单个字符
  auto append(char c) -> StringBuilder& {
    std::lock_guard<std::mutex> lock(m_mutex);
    m_buffer.push_back(c);
    return *this;
  }

  auto append(int32_t value) -> StringBuilder& {
    return append(std::to_string(value));
  }

  auto append(uint32_t value) -> StringBuilder& {
    return append(std::to_string(value));
  }

  auto append(int64_t value) -> StringBuilder& {
    return append(std::to_string(value));
  }

  auto append(uint64_t value) -> StringBuilder& {
    return append(std::to_string(value));
  }

  auto append(float value) -> StringBuilder& {
    return append(std::to_string(value));
  }

  auto append(double value) -> StringBuilder& {
    return append(std::to_string(value));
  }

  // 预分配内存
  void reserve(size_t capacity) {
    std::lock_guard<std::mutex> lock(m_mutex);
    m_buffer.reserve(capacity);
  }

  // 清空缓冲区
  void clear() {
    std::lock_guard<std::mutex> lock(m_mutex);
    m_buffer.clear();
  }

  // 获取当前长度
  auto length() const -> size_t {
    std::lock_guard<std::mutex> lock(m_mutex);
    return m_buffer.length();
  }

  // 获取当前容量
  auto capacity() const -> size_t {
    std::lock_guard<std::mutex> lock(m_mutex);
    return m_buffer.capacity();
  }

  // 转换为std::string
  auto toString() const -> std::string {
    std::lock_guard<std::mutex> lock(m_mutex);
    return m_buffer;
  }

  // 重载+=运算符
  template <typename T>
  auto operator+=(const T& value) -> StringBuilder& {
    return append(value);
  }

  // 重载<<运算符
  template <typename T>
  friend auto operator<<(StringBuilder& builder, const T& value)
    -> StringBuilder& {
    return builder += value;
  }
};
}  // namespace Utils