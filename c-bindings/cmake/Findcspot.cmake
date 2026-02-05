# Findcspot
#
# Usage:
#   find_package(cspot REQUIRED CONFIG PATHS "${PROJECT_SOURCE_DIR}/third_party/cspot")
#   target_link_libraries(your_target PRIVATE librespot::cspot)
#
# Variables that can be set by the caller:
#   cspot_ROOT / CSPOT_ROOT  - root prefix containing include/ and lib/
#
# Provides:
#   librespot::cspot  - imported target with include dirs and library location

if (TARGET librespot::cspot)
  return()
endif()

set(_lib_names cspot)

set(_roots "")
if (cspot_ROOT)
  list(APPEND _roots "${cspot_ROOT}")
endif()
if (CSPOT_ROOT)
  list(APPEND _roots "${CSPOT_ROOT}")
endif()

list(APPEND _roots
  "${CMAKE_INSTALL_PREFIX}"
  "${CMAKE_PREFIX_PATH}"
)

find_path(cspot_INCLUDE_DIR
  NAMES cspot.h
  PATH_SUFFIXES include
  PATHS ${_roots}
)

find_library(cspot_LIBRARY
  NAMES ${_lib_names}
  PATH_SUFFIXES lib lib64
  PATHS ${_roots}
)

include(FindPackageHandleStandardArgs)
find_package_handle_standard_args(cspot
  REQUIRED_VARS cspot_LIBRARY cspot_INCLUDE_DIR
)

if (cspot_FOUND)
  set(_cspot_is_static OFF)
  get_filename_component(_cspot_library_name "${cspot_LIBRARY}" NAME)
  string(TOLOWER "${_cspot_library_name}" _cspot_library_name_lower)

  if (WIN32)
    if (_cspot_library_name_lower MATCHES "\\.lib$" AND NOT _cspot_library_name_lower MATCHES "\\.dll\\.lib$")
      set(_cspot_is_static ON)
    endif()
  else()
    get_filename_component(_cspot_library_ext "${cspot_LIBRARY}" EXT)
    string(TOLOWER "${_cspot_library_ext}" _cspot_library_ext_lower)
    if (_cspot_library_ext_lower STREQUAL ".a")
      set(_cspot_is_static ON)
    endif()
  endif()

  add_library(librespot::cspot UNKNOWN IMPORTED)
  set_target_properties(librespot::cspot PROPERTIES
    IMPORTED_LOCATION "${cspot_LIBRARY}"
    INTERFACE_INCLUDE_DIRECTORIES "${cspot_INCLUDE_DIR}"
    CSPOT_IS_STATIC "${_cspot_is_static}"
  )
endif()

mark_as_advanced(cspot_LIBRARY cspot_INCLUDE_DIR)
