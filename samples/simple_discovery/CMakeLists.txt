cmake_minimum_required(VERSION 3.15)
project(simple_discovery C)

set(CMAKE_C_STANDARD 99)

if (NOT DEFINED CSPOT_INSTALL_SAMPLES_DIR)
  set(CSPOT_INSTALL_SAMPLES_DIR "samples")
endif()

if (NOT TARGET librespot::cspot)
  # Adjust to your install/prefix containing include/cspot.h and libcspot.*
  set(CSPOT_SOURCE_DIR "${CMAKE_CURRENT_LIST_DIR}/../../c-bindings")
  if (NOT cspot_ROOT)
    set(cspot_ROOT "${CSPOT_SOURCE_DIR}")
  endif()
  list(APPEND CMAKE_MODULE_PATH "${CSPOT_SOURCE_DIR}/cmake")
  find_package(cspot REQUIRED)
endif()

add_executable(simple_discovery src/connect_playback.c)
if (TARGET cspot_prebuild)
  add_dependencies(simple_discovery cspot_prebuild)
endif()
target_link_libraries(simple_discovery PRIVATE librespot::cspot m ssl crypto)

install(TARGETS simple_discovery RUNTIME DESTINATION "${CSPOT_INSTALL_SAMPLES_DIR}")
