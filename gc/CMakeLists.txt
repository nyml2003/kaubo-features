cmake_minimum_required(VERSION 3.20)
project(gc)
cmake_policy(SET CMP0177 NEW)
include(cmake/Settings.cmake)


file(GLOB_RECURSE gc_files ${kaubo_src_dir}/*.cpp ${kaubo_src_dir}/*.h )
add_executable(Demo ${gc_files})
