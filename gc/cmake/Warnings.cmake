set(kaubo_cxx_flags)
list(APPEND kaubo_cxx_flags 
    -Wall -Wextra -Werror -pedantic -pedantic-errors
    -Wconversion -Wsign-conversion -Wshadow 
    -Wdouble-promotion -Wformat=2 -Wnull-dereference
    -Wold-style-cast -Woverloaded-virtual -Wunused -Wunreachable-code
    -Wdeprecated -Winvalid-pch -Wstrict-aliasing -Wstrict-overflow=5 -Wcast-align
    -Wunused
    -Wmissing-declarations -fstack-protector-strong -D_FORTIFY_SOURCE=2
)

list(APPEND kaubo_cxx_flags -fcolor-diagnostics)
list(APPEND kaubo_cxx_flags 
    -Wall -Wextra -Werror -pedantic -pedantic-errors
    -Wconversion -Wsign-conversion -Wshadow 
    -Wdouble-promotion -Wformat=2 -Wnull-dereference
    -Wold-style-cast -Woverloaded-virtual -Wunused -Wunreachable-code
    -Wdeprecated -Winvalid-pch -Wstrict-aliasing -Wstrict-overflow=5 -Wcast-align
    -Wunused
    -Wmissing-declarations -fstack-protector-strong -D_FORTIFY_SOURCE=2)


list(APPEND kaubo_cxx_flags
    -fstack-protector-strong
    -D_FORTIFY_SOURCE=2
)

# set(CMAKE_CXX_CLANG_TIDY "clang-tidy")

# 生成 compile_commands.json 文件
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

message(STATUS "PROJECT_SOURCE_DIR ${PROJECT_SOURCE_DIR}")
message(STATUS "CMAKE_BINARY_DIR ${CMAKE_BINARY_DIR}")

add_custom_command(
    OUTPUT ${PROJECT_SOURCE_DIR}/compile_commands.json
    COMMAND ${CMAKE_COMMAND} -E copy
        ${CMAKE_BINARY_DIR}/compile_commands.json
        ${PROJECT_SOURCE_DIR}/compile_commands.json
    DEPENDS ${CMAKE_BINARY_DIR}/compile_commands.json
)

add_custom_target(copy_compile_commands
    DEPENDS ${PROJECT_SOURCE_DIR}/compile_commands.json
)