cmake_minimum_required(VERSION 3.15)

if (CMAKE_SOURCE_DIR STREQUAL CMAKE_CURRENT_LIST_DIR)
  project(cspot C)
endif()

set(CMAKE_C_STANDARD 99)

set(CSPOT_SOURCE_DIR "${CMAKE_CURRENT_LIST_DIR}")
set(CSPOT_INCLUDE_DIR "${CSPOT_SOURCE_DIR}/include")
file(MAKE_DIRECTORY "${CSPOT_INCLUDE_DIR}")

if (NOT DEFINED CSPOT_CARGO_TARGET_DIR)
  set(CSPOT_CARGO_TARGET_DIR "${CMAKE_SOURCE_DIR}/artifacts/target")
endif()
if (NOT DEFINED CSPOT_INSTALL_INCLUDE_DIR)
  set(CSPOT_INSTALL_INCLUDE_DIR "include")
endif()
if (NOT DEFINED CSPOT_INSTALL_LIB_DIR)
  set(CSPOT_INSTALL_LIB_DIR "lib")
endif()

set(CSPOT_CARGO_TARGET_TRIPLE "" CACHE STRING "Cargo target triple used to cross-compile cspot")
set(CSPOT_CARGO_FEATURES "" CACHE STRING "Cargo feature list passed to --features")
option(CSPOT_CARGO_NO_DEFAULT_FEATURES "Pass --no-default-features when building cspot with cargo" OFF)
set(CSPOT_CARGO_LINKER "" CACHE FILEPATH "Explicit linker for the configured Cargo target")
set(CSPOT_CARGO_AR "" CACHE FILEPATH "Explicit archiver for the configured Cargo target")

find_program(CBINDGEN_EXECUTABLE cbindgen REQUIRED)
find_program(CARGO_EXECUTABLE cargo REQUIRED)

set(CSPOT_CARGO_RELEASE_FLAG "")
if (CMAKE_BUILD_TYPE MATCHES "^(Release|RelWithDebInfo|MinSizeRel)$")
  set(CSPOT_CARGO_RELEASE_FLAG "--release")
endif()

set(CSPOT_CARGO_BUILD_CMD "${CARGO_EXECUTABLE}" build -p cspot --target-dir "${CSPOT_CARGO_TARGET_DIR}")
if (CSPOT_CARGO_TARGET_TRIPLE)
  list(APPEND CSPOT_CARGO_BUILD_CMD --target "${CSPOT_CARGO_TARGET_TRIPLE}")
endif()
if (CSPOT_CARGO_RELEASE_FLAG)
  list(APPEND CSPOT_CARGO_BUILD_CMD "${CSPOT_CARGO_RELEASE_FLAG}")
endif()
if (CSPOT_CARGO_NO_DEFAULT_FEATURES)
  list(APPEND CSPOT_CARGO_BUILD_CMD --no-default-features)
endif()
if (CSPOT_CARGO_FEATURES)
  list(APPEND CSPOT_CARGO_BUILD_CMD --features "${CSPOT_CARGO_FEATURES}")
endif()

set(CSPOT_CARGO_ENV_CMD "${CMAKE_COMMAND}" -E env)
if (CSPOT_CARGO_TARGET_TRIPLE)
  string(TOUPPER "${CSPOT_CARGO_TARGET_TRIPLE}" CSPOT_CARGO_TARGET_ENV_SUFFIX)
  string(REPLACE "-" "_" CSPOT_CARGO_TARGET_ENV_SUFFIX "${CSPOT_CARGO_TARGET_ENV_SUFFIX}")
  set(CSPOT_CARGO_TARGET_ENV_SUFFIX_LOWER "${CSPOT_CARGO_TARGET_TRIPLE}")
  string(REPLACE "-" "_" CSPOT_CARGO_TARGET_ENV_SUFFIX_LOWER "${CSPOT_CARGO_TARGET_ENV_SUFFIX_LOWER}")

  set(_cspot_cargo_linker "${CSPOT_CARGO_LINKER}")
  if (ANDROID AND NOT _cspot_cargo_linker)
    get_filename_component(_cspot_android_toolchain_bin "${CMAKE_C_COMPILER}" DIRECTORY)
    set(_cspot_android_api "26")
    if (DEFINED ANDROID_PLATFORM AND NOT ANDROID_PLATFORM STREQUAL "")
      set(_cspot_android_platform_value "${ANDROID_PLATFORM}")
      string(REGEX REPLACE "^android-" "" _cspot_android_api "${_cspot_android_platform_value}")
    elseif (DEFINED CMAKE_ANDROID_API AND NOT CMAKE_ANDROID_API STREQUAL "")
      set(_cspot_android_api "${CMAKE_ANDROID_API}")
    endif()

    set(_cspot_android_clang_prefix "")
    if (CSPOT_CARGO_TARGET_TRIPLE STREQUAL "i686-linux-android")
      set(_cspot_android_clang_prefix "i686-linux-android")
    elseif (CSPOT_CARGO_TARGET_TRIPLE STREQUAL "x86_64-linux-android")
      set(_cspot_android_clang_prefix "x86_64-linux-android")
    elseif (CSPOT_CARGO_TARGET_TRIPLE STREQUAL "armv7-linux-androideabi")
      set(_cspot_android_clang_prefix "armv7a-linux-androideabi")
    elseif (CSPOT_CARGO_TARGET_TRIPLE STREQUAL "aarch64-linux-android")
      set(_cspot_android_clang_prefix "aarch64-linux-android")
    endif()

    if (_cspot_android_clang_prefix)
      set(_cspot_android_linker
        "${_cspot_android_toolchain_bin}/${_cspot_android_clang_prefix}${_cspot_android_api}-clang"
      )
      if (EXISTS "${_cspot_android_linker}")
        set(_cspot_cargo_linker "${_cspot_android_linker}")
      endif()
    endif()
  endif()
  if (NOT _cspot_cargo_linker AND CMAKE_C_COMPILER)
    set(_cspot_cargo_linker "${CMAKE_C_COMPILER}")
  endif()
  if (_cspot_cargo_linker)
    list(APPEND CSPOT_CARGO_ENV_CMD "CARGO_TARGET_${CSPOT_CARGO_TARGET_ENV_SUFFIX}_LINKER=${_cspot_cargo_linker}")
    list(APPEND CSPOT_CARGO_ENV_CMD "CC_${CSPOT_CARGO_TARGET_ENV_SUFFIX_LOWER}=${_cspot_cargo_linker}")
    list(APPEND CSPOT_CARGO_ENV_CMD "CXX_${CSPOT_CARGO_TARGET_ENV_SUFFIX_LOWER}=${_cspot_cargo_linker}")
  endif()

  set(_cspot_cargo_ar "${CSPOT_CARGO_AR}")
  if (ANDROID AND NOT _cspot_cargo_ar)
    get_filename_component(_cspot_android_toolchain_bin "${CMAKE_C_COMPILER}" DIRECTORY)
    if (EXISTS "${_cspot_android_toolchain_bin}/llvm-ar")
      set(_cspot_cargo_ar "${_cspot_android_toolchain_bin}/llvm-ar")
    endif()
  endif()
  if (NOT _cspot_cargo_ar AND CMAKE_AR)
    set(_cspot_cargo_ar "${CMAKE_AR}")
  endif()
  if (_cspot_cargo_ar)
    list(APPEND CSPOT_CARGO_ENV_CMD "CARGO_TARGET_${CSPOT_CARGO_TARGET_ENV_SUFFIX}_AR=${_cspot_cargo_ar}")
    list(APPEND CSPOT_CARGO_ENV_CMD "AR_${CSPOT_CARGO_TARGET_ENV_SUFFIX_LOWER}=${_cspot_cargo_ar}")
  endif()
endif()

if (CSPOT_CARGO_RELEASE_FLAG)
  set(CSPOT_CARGO_PROFILE "release")
else()
  set(CSPOT_CARGO_PROFILE "debug")
endif()

if (CSPOT_CARGO_TARGET_TRIPLE)
  set(CSPOT_CARGO_OUTPUT_ROOT "${CSPOT_CARGO_TARGET_DIR}/${CSPOT_CARGO_TARGET_TRIPLE}")
else()
  set(CSPOT_CARGO_OUTPUT_ROOT "${CSPOT_CARGO_TARGET_DIR}")
endif()

set(CSPOT_LIBRARY_DEBUG
  "${CSPOT_CARGO_OUTPUT_ROOT}/debug/${CMAKE_SHARED_LIBRARY_PREFIX}cspot${CMAKE_SHARED_LIBRARY_SUFFIX}"
)
set(CSPOT_LIBRARY_RELEASE
  "${CSPOT_CARGO_OUTPUT_ROOT}/release/${CMAKE_SHARED_LIBRARY_PREFIX}cspot${CMAKE_SHARED_LIBRARY_SUFFIX}"
)
set(CSPOT_STATIC_LIBRARY_DEBUG
  "${CSPOT_CARGO_OUTPUT_ROOT}/debug/${CMAKE_STATIC_LIBRARY_PREFIX}cspot${CMAKE_STATIC_LIBRARY_SUFFIX}"
)
set(CSPOT_STATIC_LIBRARY_RELEASE
  "${CSPOT_CARGO_OUTPUT_ROOT}/release/${CMAKE_STATIC_LIBRARY_PREFIX}cspot${CMAKE_STATIC_LIBRARY_SUFFIX}"
)
set(CSPOT_LIBRARY_OUTPUT
  "${CSPOT_CARGO_OUTPUT_ROOT}/${CSPOT_CARGO_PROFILE}/${CMAKE_SHARED_LIBRARY_PREFIX}cspot${CMAKE_SHARED_LIBRARY_SUFFIX}"
)
set(CSPOT_STATIC_LIBRARY_OUTPUT
  "${CSPOT_CARGO_OUTPUT_ROOT}/${CSPOT_CARGO_PROFILE}/${CMAKE_STATIC_LIBRARY_PREFIX}cspot${CMAKE_STATIC_LIBRARY_SUFFIX}"
)

file(GLOB_RECURSE CSPOT_RUST_SOURCES CONFIGURE_DEPENDS
  "${CSPOT_SOURCE_DIR}/src/*.rs"
)
set(CSPOT_HEADER_DEPENDS
  "${CSPOT_SOURCE_DIR}/Cargo.toml"
  "${CSPOT_SOURCE_DIR}/Cargo.lock"
  "${CSPOT_SOURCE_DIR}/cbindgen.toml"
  ${CSPOT_RUST_SOURCES}
)

add_custom_command(
  OUTPUT
    "${CSPOT_INCLUDE_DIR}/cspot.h"
    "${CSPOT_LIBRARY_OUTPUT}"
    "${CSPOT_STATIC_LIBRARY_OUTPUT}"
  COMMAND "${CMAKE_COMMAND}" -E make_directory "${CSPOT_INCLUDE_DIR}"
  COMMAND "${CBINDGEN_EXECUTABLE}" --config "${CSPOT_SOURCE_DIR}/cbindgen.toml" --crate cspot --output "${CSPOT_INCLUDE_DIR}/cspot.h"
  COMMAND ${CSPOT_CARGO_ENV_CMD} ${CSPOT_CARGO_BUILD_CMD}
  DEPENDS ${CSPOT_HEADER_DEPENDS}
  WORKING_DIRECTORY "${CSPOT_SOURCE_DIR}"
  COMMENT "Generating cspot header and building libcspot"
  VERBATIM
  USES_TERMINAL
)
add_custom_target(cspot_prebuild ALL DEPENDS
  "${CSPOT_INCLUDE_DIR}/cspot.h"
  "${CSPOT_LIBRARY_OUTPUT}"
  "${CSPOT_STATIC_LIBRARY_OUTPUT}"
)

add_library(librespot::cspot STATIC IMPORTED GLOBAL)
set_target_properties(librespot::cspot PROPERTIES
  IMPORTED_CONFIGURATIONS "Debug;Release;RelWithDebInfo;MinSizeRel"
  IMPORTED_LOCATION_DEBUG "${CSPOT_STATIC_LIBRARY_DEBUG}"
  IMPORTED_LOCATION_RELEASE "${CSPOT_STATIC_LIBRARY_RELEASE}"
  IMPORTED_LOCATION_RELWITHDEBINFO "${CSPOT_STATIC_LIBRARY_RELEASE}"
  IMPORTED_LOCATION_MINSIZEREL "${CSPOT_STATIC_LIBRARY_RELEASE}"
  INTERFACE_INCLUDE_DIRECTORIES "${CSPOT_INCLUDE_DIR}"
  CSPOT_IS_STATIC ON
)

set(CSPOT_INSTALL_LIB_DESTINATION "${CSPOT_INSTALL_LIB_DIR}")
if (ANDROID AND CMAKE_ANDROID_ARCH_ABI)
  set(CSPOT_INSTALL_LIB_DESTINATION "${CSPOT_INSTALL_LIB_DIR}/${CMAKE_ANDROID_ARCH_ABI}")
endif()

install(FILES "${CSPOT_INCLUDE_DIR}/cspot.h" DESTINATION "${CSPOT_INSTALL_INCLUDE_DIR}")
install(FILES "${CSPOT_LIBRARY_DEBUG}" DESTINATION "${CSPOT_INSTALL_LIB_DESTINATION}" CONFIGURATIONS Debug)
install(FILES "${CSPOT_LIBRARY_RELEASE}" DESTINATION "${CSPOT_INSTALL_LIB_DESTINATION}" CONFIGURATIONS Release RelWithDebInfo MinSizeRel)
install(FILES "${CSPOT_STATIC_LIBRARY_DEBUG}" DESTINATION "${CSPOT_INSTALL_LIB_DESTINATION}" CONFIGURATIONS Debug OPTIONAL)
install(FILES "${CSPOT_STATIC_LIBRARY_RELEASE}" DESTINATION "${CSPOT_INSTALL_LIB_DESTINATION}" CONFIGURATIONS Release RelWithDebInfo MinSizeRel OPTIONAL)
