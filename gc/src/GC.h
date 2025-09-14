#pragma once
#include <unordered_set>

// 前置声明
class GCObject;

// 垃圾回收器类 - 实现标记-清除算法
class GC {
 private:
  // 所有被管理的对象
  std::unordered_set<GCObject*> allObjects;

  // 根对象集合
  std::unordered_set<GCObject*> rootObjects;

  // 标记阶段
  void mark();

  // 清除阶段
  void sweep();

  // 私有构造函数，确保单例
  GC() = default;

 public:
  // 单例模式
  static GC& getInstance();

  // 禁止拷贝和移动
  GC(const GC&) = delete;
  GC& operator=(const GC&) = delete;
  GC(GC&&) = delete;
  GC& operator=(GC&&) = delete;

  // 析构函数，清理所有对象
  ~GC();

  // 注册对象
  void registerObject(GCObject* obj);

  // 注销对象
  void unregisterObject(GCObject* obj);

  // 添加根对象
  void addRootObject(GCObject* obj);

  // 移除根对象
  void removeRootObject(GCObject* obj);

  // 执行垃圾回收
  void collectGarbage();

  // 打印垃圾回收器状态
  void printStatus();

  bool isRootObject(GCObject* obj) {
    return rootObjects.find(obj) != rootObjects.end();
  }
};
