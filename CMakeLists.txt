cmake_minimum_required(VERSION 3.15)
project(cspot VERSION 1.0.0 LANGUAGES C)

set(CMAKE_C_STANDARD 99)

set(CSPOT_INSTALL_INCLUDE_DIR "include")
set(CSPOT_INSTALL_LIB_DIR "lib")
set(CSPOT_INSTALL_SAMPLES_DIR "samples")

if (ANDROID)
  option(CSPOT_BUILD_NATIVE_SAMPLES "Build native C sample executables" OFF)
  option(CSPOT_BUILD_ANDROID_CLIENT "Build the Android sample application" ON)
else()
  option(CSPOT_BUILD_NATIVE_SAMPLES "Build native C sample executables" ON)
  option(CSPOT_BUILD_ANDROID_CLIENT "Build the Android sample application" OFF)
endif()

set(CSPOT_CARGO_TARGET_DIR "${CMAKE_CURRENT_LIST_DIR}/artifacts/target")
add_subdirectory(c-bindings)
if (CSPOT_BUILD_NATIVE_SAMPLES)
  add_subdirectory(samples/simple_discovery)
  add_subdirectory(samples/discovery_playback)
  add_subdirectory(samples/repl_app)
endif()
if (CSPOT_BUILD_ANDROID_CLIENT)
  add_subdirectory(samples/android-client)
endif()

add_custom_target(android_package_all
  COMMAND "${CMAKE_CURRENT_LIST_DIR}/scripts/package-android-all.sh"
  WORKING_DIRECTORY "${CMAKE_CURRENT_LIST_DIR}"
  COMMENT "Building all Android ABIs and producing a consolidated package"
  USES_TERMINAL
)

include(CPack)
