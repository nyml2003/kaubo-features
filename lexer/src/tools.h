#pragma once

#include <fstream>
#include <sstream>
#include <stdexcept>
#include <string>

inline auto read_file(const std::string& filename) -> std::string {
  std::ifstream file(filename);
  if (!file.is_open()) {
    throw std::runtime_error("无法打开文件: " + filename);
  }

  std::stringstream buffer;
  buffer << file.rdbuf();
  return buffer.str();
}