#pragma once
#include "Common.h"
#include "GCObject.h"
#include "GCPtr.h"
#include "Klass.h"

// 所有对象的基类
class Object : public GCObject {
 private:
  GCPtr<Klass> m_klass;  // 使用GCPtr管理对Klass的引用

 public:
  explicit Object(GCPtr<Klass> klass);
  ~Object() override = default;
  Object(const Object&) = delete;
  Object& operator=(const Object&) = delete;
  Object(Object&&) = delete;
  Object& operator=(Object&&) = delete;

  [[nodiscard]] const GCPtr<Klass>& getKlass() const;

  // 实现GCObject接口
  std::unordered_set<GCObject*> getReferences() override;
};
