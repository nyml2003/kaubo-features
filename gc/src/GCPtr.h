#pragma once

#include "GC.h"

// GC指针 - 用于对象之间的引用，不作为根对象
template <typename T>
class GCPtr {
 private:
  T* m_ptr;

 public:
  // 构造函数
  explicit GCPtr(T* ptr = nullptr) noexcept : m_ptr(ptr) {
    GC::getInstance().registerObject(m_ptr);
  }

  // 析构函数
  ~GCPtr() = default;

  // 禁止拷贝
  GCPtr(const GCPtr&) = default;
  GCPtr& operator=(const GCPtr&) = default;

  // 允许移动
  GCPtr(GCPtr&& other) = default;
  GCPtr& operator=(GCPtr&& other) = default;

  // 指针操作符
  T& operator*() const noexcept { return *m_ptr; }
  T* operator->() const noexcept { return m_ptr; }
  T* get() const noexcept { return m_ptr; }

  // 赋值操作
  GCPtr& operator=(T* ptr) noexcept {
    m_ptr = ptr;
    return *this;
  }

  // 创建对象并自动注册到GC
  template <typename... Args>
  static GCPtr<T> create(Args&&... args) {
    return GCPtr<T>(new T(std::forward<Args>(args)...));
  }
};
