include_guard(GLOBAL)

function(cspot_uses_static_library out_var)
  if (NOT TARGET librespot::cspot)
    set(${out_var} OFF PARENT_SCOPE)
    return()
  endif()

  get_target_property(_cspot_is_static_prop librespot::cspot CSPOT_IS_STATIC)
  if (NOT _cspot_is_static_prop STREQUAL "_cspot_is_static_prop-NOTFOUND")
    if (_cspot_is_static_prop)
      set(${out_var} ON PARENT_SCOPE)
    else()
      set(${out_var} OFF PARENT_SCOPE)
    endif()
    return()
  endif()

  get_target_property(_cspot_type librespot::cspot TYPE)
  if (_cspot_type STREQUAL "STATIC_LIBRARY")
    set(${out_var} ON PARENT_SCOPE)
    return()
  endif()

  if (_cspot_type STREQUAL "UNKNOWN_LIBRARY")
    get_target_property(_cspot_import_lib librespot::cspot IMPORTED_IMPLIB)
    if (_cspot_import_lib)
      set(${out_var} OFF PARENT_SCOPE)
      return()
    endif()

    get_target_property(_cspot_location librespot::cspot IMPORTED_LOCATION)
    if (NOT _cspot_location STREQUAL "_cspot_location-NOTFOUND")
      get_filename_component(_cspot_library_name "${_cspot_location}" NAME)
      string(TOLOWER "${_cspot_library_name}" _cspot_library_name_lower)

      if (WIN32 AND _cspot_library_name_lower MATCHES "\\.dll\\.lib$")
        set(${out_var} OFF PARENT_SCOPE)
        return()
      endif()

      get_filename_component(_cspot_library_ext "${_cspot_location}" EXT)
      string(TOLOWER "${_cspot_library_ext}" _cspot_library_ext_lower)
      if (_cspot_library_ext_lower STREQUAL ".a")
        set(${out_var} ON PARENT_SCOPE)
        return()
      endif()

      if (WIN32 AND _cspot_library_ext_lower STREQUAL ".lib")
        set(${out_var} ON PARENT_SCOPE)
        return()
      endif()
    endif()
  endif()

  set(${out_var} OFF PARENT_SCOPE)
endfunction()

function(cspot_link_common_deps target)
  if (NOT TARGET ${target})
    message(FATAL_ERROR "cspot_link_common_deps: target '${target}' does not exist")
  endif()

  if (CMAKE_SYSTEM_NAME STREQUAL "Linux")
    find_package(OpenSSL REQUIRED)
    target_link_libraries(${target} PRIVATE m OpenSSL::SSL OpenSSL::Crypto)
  endif()
endfunction()

function(cspot_link_platform_audio target)
  if (NOT TARGET ${target})
    message(FATAL_ERROR "cspot_link_platform_audio: target '${target}' does not exist")
  endif()

  if (CMAKE_SYSTEM_NAME STREQUAL "Linux")
    find_package(ALSA REQUIRED)
    if (TARGET ALSA::ALSA)
      target_link_libraries(${target} PRIVATE ALSA::ALSA)
    else()
      target_link_libraries(${target} PRIVATE ${ALSA_LIBRARIES})
      if (ALSA_INCLUDE_DIRS)
        target_include_directories(${target} PRIVATE ${ALSA_INCLUDE_DIRS})
      endif()
    endif()
  elseif (CMAKE_SYSTEM_NAME STREQUAL "Darwin")
    find_library(CSPOT_COREAUDIO_FRAMEWORK CoreAudio REQUIRED)
    find_library(CSPOT_AUDIOTOOLBOX_FRAMEWORK AudioToolbox REQUIRED)
    find_library(CSPOT_COREFOUNDATION_FRAMEWORK CoreFoundation REQUIRED)
    find_library(CSPOT_SECURITY_FRAMEWORK Security REQUIRED)
    find_library(CSPOT_SYSTEMCONFIGURATION_FRAMEWORK SystemConfiguration REQUIRED)
    find_library(CSPOT_IOKIT_FRAMEWORK IOKit REQUIRED)
    target_link_libraries(
      ${target}
      PRIVATE
      ${CSPOT_COREAUDIO_FRAMEWORK}
      ${CSPOT_AUDIOTOOLBOX_FRAMEWORK}
      ${CSPOT_COREFOUNDATION_FRAMEWORK}
      ${CSPOT_SECURITY_FRAMEWORK}
      ${CSPOT_SYSTEMCONFIGURATION_FRAMEWORK}
      ${CSPOT_IOKIT_FRAMEWORK}
    )
  elseif (WIN32)
    cspot_uses_static_library(_cspot_is_static)
    if (_cspot_is_static)
      target_link_libraries(
        ${target}
        PRIVATE
        ole32
        avrt
        uuid
        winmm
        iphlpapi
        propsys
        ntdll
      )
    endif()
  endif()
endfunction()

function(cspot_link_dependencies target)
  if (NOT TARGET ${target})
    message(FATAL_ERROR "cspot_link_dependencies: target '${target}' does not exist")
  endif()

  cspot_link_common_deps(${target})
  cspot_link_platform_audio(${target})
endfunction()
