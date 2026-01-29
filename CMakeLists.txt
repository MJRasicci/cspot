cmake_minimum_required(VERSION 3.15)
project(cspot C)

set(CMAKE_C_STANDARD 99)

set(CSPOT_INSTALL_INCLUDE_DIR "include")
set(CSPOT_INSTALL_LIB_DIR "lib")
set(CSPOT_INSTALL_SAMPLES_DIR "samples")

set(CSPOT_CARGO_TARGET_DIR "${CMAKE_CURRENT_LIST_DIR}/artifacts/target")
add_subdirectory(c-bindings)
add_subdirectory(samples/connect_playback)

include(CPack)
