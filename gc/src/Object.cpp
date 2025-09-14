#include "Object.h"
#include "Klass.h"

Object::Object(GCPtr<Klass> klass) : m_klass(klass) {}

const GCPtr<Klass>& Object::getKlass() const {
  return m_klass;
}

std::unordered_set<GCObject*> Object::getReferences() {
  return {};
}
