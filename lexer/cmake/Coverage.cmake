if(CMAKE_BUILD_TYPE MATCHES "Debug" AND CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    # 检查是否有LLVM工具链
    find_program(LLVM_PROFDATA llvm-profdata)
    find_program(LLVM_COV llvm-cov)

    if(NOT LLVM_PROFDATA OR NOT LLVM_COV)
        message(WARNING "LLVM工具链未找到，覆盖率报告生成将被禁用")
        set(COVERAGE_AVAILABLE FALSE)
    else()
        set(COVERAGE_AVAILABLE TRUE)
        message(STATUS "找到LLVM工具链: ${LLVM_PROFDATA}, ${LLVM_COV}")
    endif()

    # 1. 定义覆盖率核心编译/链接标志（Clang专属）
    set(COVERAGE_COMPILE_FLAGS
        -fprofile-instr-generate
        -fcoverage-mapping
        -O0 # 禁用优化，确保覆盖率准确
        -g # 生成调试信息
    )
    set(COVERAGE_LINK_FLAGS
        -fprofile-instr-generate
        -fcoverage-mapping
    )

    # 2. 手动为目标添加覆盖率标志的函数
    function(add_coverage_target target)
        if(TARGET ${target} AND COVERAGE_AVAILABLE)
            # 确保这些标志被添加
            target_compile_options(${target}
                PRIVATE
                ${COVERAGE_COMPILE_FLAGS}
            )
            target_link_options(${target}
                PRIVATE
                ${COVERAGE_LINK_FLAGS}
            )

            # 验证标志是否已添加
            get_target_property(COMPILE_OPTS ${target} COMPILE_OPTIONS)
            get_target_property(LINK_OPTS ${target} LINK_OPTIONS)
            message(STATUS "[Coverage] 目标 ${target} 编译选项: ${COMPILE_OPTS}")
            message(STATUS "[Coverage] 目标 ${target} 链接选项: ${LINK_OPTS}")

            message(STATUS "[Coverage] 已为目标 ${target} 添加覆盖率标志")
        else()
            message(WARNING "[Coverage] 目标 ${target} 不存在或覆盖率不可用，无法添加覆盖率标志")
        endif()
    endfunction()

    # 3. 清理覆盖率数据/报告的目标
    add_custom_target(coverage_clean
        COMMAND ${CMAKE_COMMAND} -E remove -f ${CMAKE_CURRENT_BINARY_DIR}/*.profraw
        COMMAND ${CMAKE_COMMAND} -E remove -f ${CMAKE_CURRENT_BINARY_DIR}/*.profdata
        COMMAND ${CMAKE_COMMAND} -E remove_directory ${CMAKE_CURRENT_BINARY_DIR}/coverage_report
        COMMAND ${CMAKE_COMMAND} -E remove_directory ${CMAKE_CURRENT_BINARY_DIR}/coverage_html
        COMMENT "[Coverage] 清理覆盖率数据和报告"
    )

    # 4. 核心函数：添加覆盖率全流程目标
    function(add_coverage_report test_target src_dir)
        if(NOT TARGET ${test_target} OR NOT COVERAGE_AVAILABLE)
            message(WARNING "[Coverage] 测试目标 ${test_target} 不存在或覆盖率不可用，无法生成报告")
            return()
        endif()

        # 定义Python脚本路径（根据实际存放位置调整）
        set(COVERAGE_SCRIPT "${CMAKE_CURRENT_SOURCE_DIR}/cmake/coverage_runner.py")

        if(NOT EXISTS ${COVERAGE_SCRIPT})
            message(FATAL_ERROR "[Coverage] 未找到Python脚本: ${COVERAGE_SCRIPT}")
        endif()

        # 构建Python脚本调用命令（跨平台统一）
        set(COVERAGE_FULL_CMD
            ${Python3_EXECUTABLE} "${COVERAGE_SCRIPT}"
            --test-exe "$<TARGET_FILE:${test_target}>" # 测试程序路径
            --coverage-name "${test_target}" # 覆盖率数据前缀
            --src-dir "${src_dir}" # 源代码目录
            --output-dir "${CMAKE_CURRENT_BINARY_DIR}" # 输出目录（build目录）
            --llvm-profdata "${LLVM_PROFDATA}"
            --llvm-cov "${LLVM_COV}"
        )

        # 定义全流程目标（依赖测试程序构建和运行）
        add_custom_target(coverage_${test_target}
            COMMAND ${COVERAGE_FULL_CMD}
            DEPENDS ${test_target} # 确保先构建测试程序
            WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
            COMMENT "[Coverage] 执行 ${test_target} 覆盖率全流程"
        )

        # 确保测试被执行
        add_dependencies(coverage_${test_target} ${test_target})

        message(STATUS "[Coverage] 已添加全流程目标：coverage_${test_target}")
    endfunction()

    # 确保找到Python3
    find_package(Python3 REQUIRED)
    message(STATUS "[Coverage] 模块加载成功（Debug+Clang），使用Python: ${Python3_EXECUTABLE}")
else()
    # 非Debug/非Clang环境的空实现
    function(add_coverage_target target)
        message(STATUS "[Coverage] 已禁用（非Debug模式或非Clang编译器）")
    endfunction()

    function(add_coverage_report test_target src_dir)
        message(STATUS "[Coverage] 已禁用（非Debug模式或非Clang编译器）")
    endfunction()

    add_custom_target(coverage_clean
        COMMENT "[Coverage] 已禁用，无需清理"
    )
endif()