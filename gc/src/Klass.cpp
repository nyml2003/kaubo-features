#include "Klass.h"
#include "GC.h"

Klass::Klass(std::string name) : className(std::move(name)) {
  GC::getInstance().addRootObject(this);
}

std::string Klass::getClassName() const {
  return className;
}

std::unordered_set<GCObject*> Klass::getReferences() {
  // Klass对象通常不引用其他GCObject
  return {};
}

Klass::Klass(Klass&& other) noexcept : className(std::move(other.className)) {}

Klass& Klass::operator=(Klass&& other) noexcept {
  if (this != &other) {
    className = std::move(other.className);
  }
  return *this;
}
