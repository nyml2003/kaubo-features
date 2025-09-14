#include "GC.h"
#include <iostream>
#include <vector>
#include "GCObject.h"

GC& GC::getInstance() {
  static GC instance;
  return instance;
}

GC::~GC() {
  // 清理所有剩余对象
  for (GCObject* obj : allObjects) {
    delete obj;
  }
  allObjects.clear();
  rootObjects.clear();
}

void GC::registerObject(GCObject* obj) {
  if (obj != nullptr) {
    allObjects.insert(obj);
  }
}

void GC::unregisterObject(GCObject* obj) {
  if (obj != nullptr) {
    allObjects.erase(obj);
    rootObjects.erase(obj);
  }
}

void GC::addRootObject(GCObject* obj) {
  if (obj != nullptr) {
    rootObjects.insert(obj);
  }
}

void GC::removeRootObject(GCObject* obj) {
  if (obj != nullptr) {
    rootObjects.erase(obj);
  }
}

void GC::mark() {
  // 遍历所有根对象，启动标记
  for (GCObject* root : rootObjects) {
    if (root != nullptr) {
      root->mark();
    }
  }
}

void GC::sweep() {
  std::vector<GCObject*> toDelete;

  // 收集所有未标记的对象
  for (GCObject* obj : allObjects) {
    if (!obj->isMarked()) {
      toDelete.push_back(obj);
    } else {
      obj->disableMark();
    }
  }

  // 删除未标记的对象并注销
  for (GCObject* obj : toDelete) {
    unregisterObject(obj);
    delete obj;
  }
}

void GC::collectGarbage() {
  std::cout << "Starting garbage collection..." << '\n';
  size_t before = allObjects.size();

  mark();
  sweep();

  size_t after = allObjects.size();
  std::cout << "Garbage collection completed. " << (before - after)
            << " objects collected." << '\n';
}

void GC::printStatus() {
  std::cout << "GC Status: " << allObjects.size() << " objects in memory."
            << '\n';
  std::cout << "Objects in memory: " << allObjects.size() << '\n';
  for (GCObject* obj : allObjects) {
    obj->print();
  }
  std::cout << '\n';
}