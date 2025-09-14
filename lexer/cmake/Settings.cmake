# 设置工作目录
set(project_dir ${CMAKE_CURRENT_SOURCE_DIR})
set(project_src_dir ${project_dir}/src)

# 设置 C++ 标准和编译器选项
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF) # 禁用编译器特定扩展

# 使用 UTF-8 编码
if(MSVC)
    list(APPEND project_cxx_flags /utf-8)
elseif(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    add_compile_options(-finput-charset=UTF-8)
endif()

include(cmake/BuildSystem.cmake)
include(cmake/Warnings.cmake)
include(cmake/Coverage.cmake)
include(cmake/Test.cmake)