set(test_name "TEST_UTILS")

add_executable(
    ${test_name}
    ${project_dir}/test/unittest/utils.cpp
)
set_target_properties(${test_name} PROPERTIES COMPILE_FLAGS ${project_cxx_flags})

# 添加头文件搜索目录（针对当前测试目标）
target_include_directories(${test_name}
    PRIVATE
    ${project_dir}/src
)

# gtest
target_link_libraries(${test_name} gtest gtest_main)
add_test(NAME ${test_name} COMMAND ${test_name})
add_coverage_target(${test_name})
add_coverage_report(${test_name} ${project_dir}/src)