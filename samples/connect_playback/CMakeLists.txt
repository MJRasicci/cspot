cmake_minimum_required(VERSION 3.15)
project(connect_playback C)

set(CMAKE_C_STANDARD 99)

# Adjust to your install/prefix containing include/cspot.h and libcspot.*
set(CSPOT_SOURCE_DIR "${CMAKE_CURRENT_LIST_DIR}/../../c-bindings")
if (NOT cspot_ROOT)
  set(cspot_ROOT "${CSPOT_SOURCE_DIR}")
endif()

list(APPEND CMAKE_MODULE_PATH "${CSPOT_SOURCE_DIR}/cmake")
find_package(cspot QUIET)

if (NOT cspot_FOUND)
  find_program(CBINDGEN_EXECUTABLE cbindgen REQUIRED)
  find_program(CARGO_EXECUTABLE cargo REQUIRED)
  set(CSPOT_CARGO_RELEASE_FLAG "")
  if (CMAKE_BUILD_TYPE MATCHES "^(Release|RelWithDebInfo|MinSizeRel)$")
    set(CSPOT_CARGO_RELEASE_FLAG "--release")
  endif()
  set(CSPOT_CARGO_BUILD_CMD "${CARGO_EXECUTABLE}" build -p cspot)
  if (CSPOT_CARGO_RELEASE_FLAG)
    list(APPEND CSPOT_CARGO_BUILD_CMD "${CSPOT_CARGO_RELEASE_FLAG}")
  endif()

  set(CSPOT_INCLUDE_DIR "${CSPOT_SOURCE_DIR}/include")
  set(CSPOT_LIBRARY_DEBUG
    "${CSPOT_SOURCE_DIR}/target/debug/${CMAKE_SHARED_LIBRARY_PREFIX}cspot${CMAKE_SHARED_LIBRARY_SUFFIX}"
  )
  set(CSPOT_LIBRARY_RELEASE
    "${CSPOT_SOURCE_DIR}/target/release/${CMAKE_SHARED_LIBRARY_PREFIX}cspot${CMAKE_SHARED_LIBRARY_SUFFIX}"
  )

  add_custom_target(cspot_prebuild
    COMMAND "${CMAKE_COMMAND}" -E make_directory "${CSPOT_INCLUDE_DIR}"
    COMMAND "${CBINDGEN_EXECUTABLE}" --config "${CSPOT_SOURCE_DIR}/cbindgen.toml" --crate cspot --output "${CSPOT_INCLUDE_DIR}/cspot.h"
    COMMAND ${CSPOT_CARGO_BUILD_CMD}
    WORKING_DIRECTORY "${CSPOT_SOURCE_DIR}"
    COMMENT "Generating cspot header and building libcspot"
    VERBATIM
  )

  add_library(librespot::cspot UNKNOWN IMPORTED)
  set_target_properties(librespot::cspot PROPERTIES
    IMPORTED_CONFIGURATIONS "Debug;Release;RelWithDebInfo;MinSizeRel"
    IMPORTED_LOCATION_DEBUG "${CSPOT_LIBRARY_DEBUG}"
    IMPORTED_LOCATION_RELEASE "${CSPOT_LIBRARY_RELEASE}"
    IMPORTED_LOCATION_RELWITHDEBINFO "${CSPOT_LIBRARY_RELEASE}"
    IMPORTED_LOCATION_MINSIZEREL "${CSPOT_LIBRARY_RELEASE}"
    INTERFACE_INCLUDE_DIRECTORIES "${CSPOT_INCLUDE_DIR}"
  )
endif()

add_executable(connect_playback src/connect_playback.c)
if (TARGET cspot_prebuild)
  add_dependencies(connect_playback cspot_prebuild)
endif()
target_link_libraries(connect_playback PRIVATE librespot::cspot m ssl crypto)
