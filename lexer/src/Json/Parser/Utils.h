#pragma once
#include <memory>

namespace Json::Utils {
template <typename T>
inline auto create(T&& obj) -> std::shared_ptr<T> {
  return std::make_shared<T>(std::forward<T>(obj));
}

template <typename T, typename... Args>
inline auto create(Args&&... args) -> std::shared_ptr<T> {
  return std::make_shared<T>(std::forward<Args>(args)...);
}

}  // namespace Json::Utils