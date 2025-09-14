#include "GC.h"
#include "GCPtr.h"
#include "Klass.h"
#include "List.h"
#include "Object.h"

// 示例：创建一个带引用的类
class MyObject : public Object {
 private:
  GCPtr<MyObject> m_child;  // 使用GCPtr管理子对象引用

 public:
  explicit MyObject(GCPtr<Klass> klass) : Object(klass) {}

  // 实现引用获取方法
  std::unordered_set<GCObject*> getReferences() override {
    std::unordered_set<GCObject*> refs = Object::getReferences();
    if (m_child.get() != nullptr) {
      refs.insert(m_child.get());
    }
    return refs;
  }

  void setChild(GCPtr<MyObject> child) { m_child = child; }
};

int main() {
  // 创建类元信息（根对象）
  auto myObjectKlass = GCPtr<Klass>::create("MyObject");

  GC::getInstance().collectGarbage();  // 此时所有根对象都不会被回收

  // 创建对象树
  auto obj1 = GCPtr<MyObject>::create(myObjectKlass);

  auto obj2 = GCPtr<MyObject>::create(myObjectKlass);

  auto obj3 = GCPtr<MyObject>::create(myObjectKlass);

  auto list = GCPtr<List<MyObject>>::create(std::vector<GCPtr<MyObject>>{obj3});
  list->add(obj1);
  list->add(obj2);

  GC::getInstance().addRootObject(list.get());

  GC::getInstance().collectGarbage();
  return 0;
}
