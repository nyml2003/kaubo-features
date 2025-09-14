# 设置工作目录
set(kaubo_dir ${CMAKE_CURRENT_SOURCE_DIR})
set(kaubo_src_dir ${kaubo_dir}/src)

# 设置 C++ 标准和编译器选项
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF) # 禁用编译器特定扩展


# 使用 UTF-8 编码
if(MSVC)
    list(APPEND kaubo_cxx_flags /utf-8)
elseif(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    add_compile_options(-finput-charset=UTF-8)
endif()

include(cmake/BuildSystem.cmake)
include(cmake/Warnings.cmake)