cmake_minimum_required(VERSION 3.16)
project(audio_engine LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# 查找所需的包
find_package(OpenSSL REQUIRED)
find_package(PkgConfig REQUIRED)

# hiredis: 尝试 pkg-config，失败则回退到手动路径
pkg_check_modules(HIREDIS hiredis)
if(NOT HIREDIS_FOUND)
    # 手动搜索 hiredis
    find_path(HIREDIS_INCLUDE_DIR hiredis/hiredis.h
        PATHS /usr/include /usr/local/include)
    find_library(HIREDIS_LIBRARY hiredis
        PATHS /usr/lib /usr/local/lib)
    if(HIREDIS_INCLUDE_DIR AND HIREDIS_LIBRARY)
        set(HIREDIS_FOUND TRUE)
        set(HIREDIS_INCLUDE_DIRS ${HIREDIS_INCLUDE_DIR})
        set(HIREDIS_LIBRARIES ${HIREDIS_LIBRARY})
        message(STATUS "手动找到 hiredis: ${HIREDIS_LIBRARIES}")
    endif()
endif()

if(NOT HIREDIS_FOUND)
    message(FATAL_ERROR "未找到 hiredis。请安装: apt-get install libhiredis-dev")
endif()

# 源文件
set(SOURCES
    ../radioserver.cpp
    ../metadata.cpp
)

# Crow 是仅头文件库，只需包含目录
include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/..
    ${HIREDIS_INCLUDE_DIRS}
)

# 编译器标志
set(CMAKE_CXX_FLAGS_RELEASE "-O3 -flto -march=native -DNDEBUG")
set(CMAKE_CXX_FLAGS_DEBUG "-g -O0")

# 链接
add_executable(audio_engine ${SOURCES})

target_link_libraries(audio_engine
    pthread
    OpenSSL::SSL
    OpenSSL::Crypto
    ${HIREDIS_LIBRARIES}
)

# 安装
install(TARGETS audio_engine RUNTIME DESTINATION /usr/local/bin)
