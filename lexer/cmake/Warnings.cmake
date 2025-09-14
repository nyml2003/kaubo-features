set(project_cxx_flags)
list(APPEND project_cxx_flags
    -Wall -Wextra -Werror -pedantic -pedantic-errors
    -Wconversion -Wsign-conversion -Wshadow
    -Wdouble-promotion -Wformat=2 -Wnull-dereference
    -Wold-style-cast -Woverloaded-virtual -Wunused -Wunreachable-code
    -Wdeprecated -Winvalid-pch -Wstrict-aliasing -Wstrict-overflow=5 -Wcast-align
    -Wunused
    -Wmissing-declarations -fstack-protector-strong -D_FORTIFY_SOURCE=2
)

list(APPEND project_cxx_flags -fcolor-diagnostics)

# 生成 compile_commands.json 文件
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

message(STATUS "PROJECT_SOURCE_DIR ${PROJECT_SOURCE_DIR}")
message(STATUS "CMAKE_BINARY_DIR ${CMAKE_BINARY_DIR}")
