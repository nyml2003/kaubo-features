#include "Value.h"
#include "Utils/Overloaded.h"
#include "Utils/StringBuilder.h"

#include <format>

namespace Json::Value {
using ::Utils::StringBuilder;

auto Value::to_string() const -> std::string {
  return std::visit(
    overloaded{
      // 处理 Null 类型
      [](const std::shared_ptr<Null>&) -> std::string { return "null"; },
      // 处理布尔类型
      [](const std::shared_ptr<True>&) -> std::string { return "true"; },
      [](const std::shared_ptr<False>&) -> std::string { return "false"; },
      // 处理数字类型
      [](const std::shared_ptr<Number>& n) -> std::string {
        return std::to_string(n->value);
      },
      [](const std::shared_ptr<String>& s) -> std::string {
        return std::format("\"{}\"", s->value);
      },
      // 处理数组类型（递归调用 to_string）
      [](const std::shared_ptr<Array>& arr) -> std::string {
        if (arr->value.empty()) {
          return "[]";  // 空指针安全处理
        }
        StringBuilder sb;
        sb << "[";
        // 遍历数组元素，拼接每个元素的字符串
        for (size_t i = 0; i < arr->value.size(); ++i) {
          sb << arr->value.at(i)->to_string();  // 递归调用元素的 to_string
          if (i != arr->value.size() - 1) {
            sb << ", ";
          }
        }
        sb << "]";
        return sb.toString();
      },
      // 处理对象类型（递归调用 to_string）
      [](const std::shared_ptr<Object>& obj) -> std::string {
        if (!obj) {
          return "{}";  // 空指针安全处理
        }
        StringBuilder sb;
        sb << "{";
        // 遍历键值对，拼接每个键值对的字符串
        size_t count = 0;
        for (const auto& [key, value] : obj->value) {
          sb << "\"" << key
             << "\": " << value->to_string();  // 键加双引号，值递归转换
          if (count != obj->value.size() - 1) {
            sb << ", ";
          }
          ++count;
        }
        sb << "}";
        return sb.toString();
      }
    },
    m_value
  );
}

auto Value::get(const std::string& key) -> Result<ValuePtr, std::string> {
  return std::visit(
    overloaded{
      [&](std::shared_ptr<Object>& obj) -> Result<ValuePtr, std::string> {
        auto it = obj->value.find(key);
        if (it != obj->value.end()) {
          return Ok(it->second);
        }
        return Err(std::format("Key not found: {}", key));
      },
      [this](auto&&) -> Result<ValuePtr, std::string> {
        return Err(std::format("Not an object: {}", to_string()));
      },
    },
    m_value
  );
}

auto Value::set(const std::string& key, ValuePtr value)
  -> Result<ValuePtr, std::string> {
  return std::visit(
    overloaded{
      [&](std::shared_ptr<Object>& obj) -> Result<ValuePtr, std::string> {
        obj->value[key] = value;
        return Ok(value);
      },
      [this](auto&&) -> Result<ValuePtr, std::string> {
        return Err(std::format("Not an object: {}", to_string()));
      }
    },
    m_value
  );
}

}  // namespace Json::Value