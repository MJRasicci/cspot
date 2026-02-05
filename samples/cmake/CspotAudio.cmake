include_guard(GLOBAL)

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
    target_link_libraries(
      ${target}
      PRIVATE
      ${CSPOT_COREAUDIO_FRAMEWORK}
      ${CSPOT_AUDIOTOOLBOX_FRAMEWORK}
      ${CSPOT_COREFOUNDATION_FRAMEWORK}
    )
  elseif (WIN32)
    target_link_libraries(${target} PRIVATE ole32 avrt uuid winmm)
  endif()
endfunction()
