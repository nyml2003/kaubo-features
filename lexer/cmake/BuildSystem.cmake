set(CMAKE_GENERATOR Ninja)
set(BUILD_COMMAND "ninja -j${NUM_PROCESSORS}")
message(STATUS "Build command: ${BUILD_COMMAND}")