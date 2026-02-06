cmake_minimum_required(VERSION 3.15)
project(repl_app C)

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

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_LIST_DIR}/../cmake")
include(CspotDependencies)

add_executable(repl_app src/repl_app.c)
if (TARGET cspot_prebuild)
  add_dependencies(repl_app cspot_prebuild)
endif()
target_link_libraries(repl_app PRIVATE librespot::cspot)
cspot_link_dependencies(repl_app)

if (NOT WIN32)
  find_package(Threads REQUIRED)
  target_link_libraries(repl_app PRIVATE Threads::Threads)
endif()

install(TARGETS repl_app RUNTIME DESTINATION "${CSPOT_INSTALL_SAMPLES_DIR}")
