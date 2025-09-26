cmake_minimum_required(VERSION 3.20)
project(lexer)
cmake_policy(SET CMP0177 NEW)
include(cmake/Settings.cmake)

file(GLOB_RECURSE lexer_files ${project_src_dir}/lexer/*.cpp)
file(GLOB_RECURSE parser_files ${project_src_dir}/parser/*.cpp)
file(GLOB_RECURSE utils_files ${project_src_dir}/utils/*.cpp)
add_executable(Demo src/main.cpp ${lexer_files} ${parser_files} ${utils_files})

file(GLOB_RECURSE json_files ${project_src_dir}/json/*.cpp)
add_executable(json src/json.cpp ${json_files})