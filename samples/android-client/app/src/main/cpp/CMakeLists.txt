cmake_minimum_required(VERSION 3.22)
project(android_client_jni LANGUAGES CXX)

if (NOT DEFINED CSPOT_HEADER_DIR)
  message(FATAL_ERROR "CSPOT_HEADER_DIR must be provided")
endif()
if (NOT DEFINED CSPOT_LIB_DIR)
  message(FATAL_ERROR "CSPOT_LIB_DIR must be provided")
endif()
if (NOT EXISTS "${CSPOT_LIB_DIR}/libcspot.so")
  message(FATAL_ERROR "Expected libcspot.so at ${CSPOT_LIB_DIR}/libcspot.so")
endif()

add_library(android_client_jni SHARED android_client_jni.cpp)

target_include_directories(android_client_jni PRIVATE "${CSPOT_HEADER_DIR}")
target_link_directories(android_client_jni PRIVATE "${CSPOT_LIB_DIR}")

target_link_libraries(android_client_jni PRIVATE cspot log android)

target_compile_features(android_client_jni PRIVATE cxx_std_17)
