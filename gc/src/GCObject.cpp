#include "GCObject.h"
#include <iostream>
#include "GC.h"
void GCObject::mark() {
  if (!marked) {
    marked = true;
    // 遍历所有引用的对象并标记
    for (GCObject* ref : getReferences()) {
      if (ref != nullptr) {
        ref->mark();
      }
    }
  }
}

void GCObject::print(int depth) {
  std::cout << std::string(depth, ' ') << "GCObject: " << this;
  if (GC::getInstance().isRootObject(this)) {
    std::cout << " (Root)";
  }
  std::cout << '\n';
  for (GCObject* ref : getReferences()) {
    if (ref != nullptr) {
      ref->print(depth + 2);
    }
  }
}