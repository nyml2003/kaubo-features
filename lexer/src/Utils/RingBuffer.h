#pragma once
#include <condition_variable>
#include <cstddef>
#include <mutex>
#include <optional>
#include <stdexcept>
#include <vector>

namespace Utils {
// 线程安全环形缓冲区，适配流式数据（如文件）的生产者-消费者模型
class RingBuffer {
 public:
  // 构造函数：初始化缓冲区容量，底层容器预分配空间
  explicit RingBuffer(size_t capacity)
    : m_capacity(capacity), m_buffer(capacity) {}

  // 禁止拷贝构造与赋值（避免线程安全问题）
  RingBuffer(const RingBuffer&) = delete;
  auto operator=(const RingBuffer&) -> RingBuffer& = delete;

  // 允许移动构造与赋值（实际使用中需谨慎，确保线程安全）
  RingBuffer(RingBuffer&&) noexcept = delete;
  auto operator=(RingBuffer&&) noexcept -> RingBuffer& = delete;

  ~RingBuffer() = default;

  // 向缓冲区添加数据（阻塞式）：满时阻塞，关闭后禁止写入
  void push(char item) {
    std::unique_lock<std::mutex> lock(m_mutex);
    // 阻塞条件：缓冲区满且未关闭（使用无锁版本检查）
    m_not_full.wait(lock, [this]() { return !is_full_unlocked() || m_closed; });

    if (m_closed) {
      throw std::runtime_error("cannot push to closed ring buffer");
    }

    // 写入数据并更新尾指针与大小
    m_buffer[m_tail] = item;
    m_tail = (m_tail + 1) % m_capacity;
    m_size++;

    lock.unlock();
    m_not_empty.notify_one();  // 通知消费者有新数据
  }

  // 从缓冲区获取数据（阻塞式）：空时阻塞，关闭且空时抛异常
  auto pop() -> char {
    std::unique_lock<std::mutex> lock(m_mutex);
    // 阻塞条件：缓冲区空且未关闭（使用无锁版本检查）
    m_not_empty.wait(lock, [this]() {
      return !is_empty_unlocked() || m_closed;
    });

    if (is_empty_unlocked() && m_closed) {
      throw std::runtime_error("cannot pop from empty and closed ring buffer");
    }

    // 读取数据并更新头指针与大小
    char item = m_buffer[m_head];
    m_head = (m_head + 1) % m_capacity;
    m_size--;

    lock.unlock();
    m_not_full.notify_one();  // 通知生产者有空闲空间
    return item;
  }

  // 尝试获取数据（非阻塞式）：空时返回std::nullopt，不阻塞
  auto try_pop() -> std::optional<char> {
    std::lock_guard<std::mutex> lock(m_mutex);

    if (is_empty_unlocked()) {
      return std::nullopt;
    }

    char item = m_buffer[m_head];
    m_head = (m_head + 1) % m_capacity;
    m_size--;

    m_not_full.notify_one();  // 唤醒可能阻塞的生产者
    return item;
  }

  // 尝试观察缓冲区头部数据（非阻塞式）：空时返回std::nullopt，不阻塞
  auto try_peek() -> std::optional<char> {
    std::lock_guard<std::mutex> lock(m_mutex);

    if (is_empty_unlocked()) {
      return std::nullopt;
    }

    return m_buffer[m_head];
  }

  // 尝试观察缓冲区头部数据（非阻塞式）：空时返回std::nullopt，不阻塞
  auto try_peek(size_t k) -> std::optional<char> {
    std::lock_guard<std::mutex> lock(m_mutex);

    if (is_empty_unlocked()) {
      return std::nullopt;
    }

    return m_buffer[m_head + k];
  }

  // 关闭缓冲区：不再接受新数据，唤醒所有阻塞的线程
  void close() {
    std::lock_guard<std::mutex> lock(m_mutex);
    m_closed = true;
    m_not_full.notify_all();   // 唤醒阻塞的生产者（避免死等）
    m_not_empty.notify_all();  // 唤醒阻塞的消费者（告知关闭状态）
  }

  // 检查缓冲区是否已关闭
  [[nodiscard]] auto is_closed() const -> bool {
    std::lock_guard<std::mutex> lock(m_mutex);
    return m_closed;
  }

  // 检查缓冲区是否为空
  [[nodiscard]] auto is_empty() const -> bool {
    std::lock_guard<std::mutex> lock(m_mutex);
    return is_empty_unlocked();
  }

  // 检查缓冲区是否已满
  [[nodiscard]] auto is_full() const -> bool {
    std::lock_guard<std::mutex> lock(m_mutex);
    return is_full_unlocked();
  }

  // 获取当前缓冲区中的数据量
  [[nodiscard]] auto get_size() const -> size_t {
    std::lock_guard<std::mutex> lock(m_mutex);
    return m_size;
  }

  // 获取当前缓冲区中的数据量是否大于等于指定值
  [[nodiscard]] auto is_size_at_least(size_t size) const -> bool {
    std::lock_guard<std::mutex> lock(m_mutex);
    return m_size >= size;
  }

  // 获取缓冲区的总容量（常量，无需加锁）
  [[nodiscard]] auto get_capacity() const -> size_t { return m_capacity; }

 private:
  // 无锁版本：检查缓冲区是否为空（仅内部已加锁时使用）
  [[nodiscard]] auto is_empty_unlocked() const -> bool { return m_size == 0; }

  // 无锁版本：检查缓冲区是否已满（仅内部已加锁时使用）
  [[nodiscard]] auto is_full_unlocked() const -> bool {
    return m_size == m_capacity;
  }

  size_t m_capacity;           // 缓冲区总容量（不可修改）
  std::vector<char> m_buffer;  // 底层存储容器
  size_t m_head{};             // 数据读取指针（头指针）
  size_t m_tail{};             // 数据写入指针（尾指针）
  size_t m_size{};             // 当前存储的数据量
  bool m_closed{};             // 缓冲区关闭标记

  mutable std::mutex m_mutex;           // 互斥锁（保护所有共享状态）
  std::condition_variable m_not_full;   // 非满条件变量（生产者等待）
  std::condition_variable m_not_empty;  // 非空条件变量（消费者等待）
};
}  // namespace Utils
