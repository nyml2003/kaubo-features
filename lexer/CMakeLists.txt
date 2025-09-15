cmake_minimum_required(VERSION 3.20)
project(lexer)
cmake_policy(SET CMP0177 NEW)
include(cmake/Settings.cmake)

# file(GLOB_RECURSE project_files ${project_src_dir}/*.cpp)
# add_executable(Demo ${project_files})
add_executable(
    statemachine
    src/Lexer/main.cpp
)