#pragma once
#include <unordered_set>

// 前置声明
template <typename T>
class GCPtr;

// 所有可被垃圾回收的对象的基类
class GCObject {
 private:
  bool marked = false;  // 标记是否可达
 public:
  GCObject() = default;
  virtual ~GCObject() = default;
  [[nodiscard]] bool isMarked() const { return marked; }
  void enableMark() { marked = true; }
  void disableMark() { marked = false; }

  // 禁止拷贝
  GCObject(const GCObject&) = delete;
  GCObject& operator=(const GCObject&) = delete;
  // 禁止移动
  GCObject(GCObject&&) = delete;
  GCObject& operator=(GCObject&&) = delete;

  // 虚函数：返回当前对象引用的所有其他GCObject
  virtual std::unordered_set<GCObject*> getReferences() = 0;

  // 递归标记自身及所有引用的对象
  void mark();

  void print(int depth = 0);
};
