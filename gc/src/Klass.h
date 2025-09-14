#pragma once
#include <string>
#include "Common.h"
#include "GCObject.h"

// 类元信息类，用于描述对象的类型信息
class Klass : public GCObject {
 private:
  std::string className;

 public:
  explicit Klass(std::string name);
  ~Klass() override = default;
  Klass(const Klass&) = delete;
  Klass& operator=(const Klass&) = delete;
  Klass(Klass&&) noexcept ;
  Klass& operator=(Klass&&)  noexcept ;

  [[nodiscard]] std::string getClassName() const;

  // 实现GCObject接口
  std::unordered_set<GCObject*> getReferences() override;

 
};
