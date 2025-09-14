#pragma once

#include <vector>
#include "GCPtr.h"
#include "Klass.h"
#include "Object.h"

// 前向声明
template <typename T>
class List;

class ListKlass : public Klass {
 public:
  // 构造函数
  ListKlass() : Klass("List") {}
  ListKlass(const ListKlass&) = delete;
  ListKlass& operator=(const ListKlass&) = delete;
  ListKlass(ListKlass&&) = delete;
  ListKlass& operator=(ListKlass&&) = delete;

  // 析构函数
  ~ListKlass() override = default;
};

template <typename T>
class List : public Object {
 private:
  std::vector<GCPtr<T>> m_elements;  // 存储列表元素

 public:
  // 构造函数
  explicit List() : Object(getKlass()) {}

  explicit List(std::vector<GCPtr<T>> elements)
    : Object(getKlass()), m_elements(std::move(elements)) {}

  // 析构函数
  ~List() override = default;

  List(const List&) = delete;
  List& operator=(const List&) = delete;
  List(List&&) = delete;
  List& operator=(List&&) = delete;

  // 获取列表中的元素数量
  [[nodiscard]] size_t size() const { return m_elements.size(); }

  // 检查列表是否为空
  [[nodiscard]] bool isEmpty() const { return m_elements.empty(); }

  // 添加元素到列表末尾
  void add(GCPtr<T> element) { m_elements.push_back(std::move(element)); }

  // 在指定位置插入元素
  void insert(size_t index, GCPtr<T> element) {
    if (index <= m_elements.size()) {
      m_elements.insert(m_elements.begin() + index, std::move(element));
    }
  }

  // 移除指定位置的元素
  void remove(size_t index) {
    if (index < m_elements.size()) {
      m_elements.erase(m_elements.begin() + index);
    }
  }

  // 获取指定位置的元素
  GCPtr<T> get(size_t index) const {
    if (index < m_elements.size()) {
      return m_elements[index];
    }
    return nullptr;
  }

  // 设置指定位置的元素
  void set(size_t index, GCPtr<T> element) {
    if (index < m_elements.size()) {
      m_elements[index] = std::move(element);
    }
  }

  // 清除列表中的所有元素
  void clear() { m_elements.clear(); }

  // 获取该对象引用的所有其他GC对象
  std::unordered_set<GCObject*> getReferences() override {
    std::unordered_set<GCObject*> refs = Object::getReferences();

    // 添加对所有元素的引用
    for (const auto& element : m_elements) {
      if (element.get() != nullptr) {
        refs.insert(element.get());
      }
    }

    return refs;
  }

  // 转换为字符串表示
  [[nodiscard]] std::string toString() const {
    std::string result = "List[";
    for (size_t i = 0; i < m_elements.size(); ++i) {
      if (i > 0) {
        result += ", ";
      }
      if (m_elements[i] != nullptr) {
        result += m_elements[i]->toString();
      } else {
        result += "null";
      }
    }
    result += "]";
    return result;
  }
};
