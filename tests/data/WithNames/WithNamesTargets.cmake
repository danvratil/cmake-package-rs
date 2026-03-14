cmake_policy(PUSH)
cmake_policy(VERSION 2.8.12...3.29)

set(CMAKE_IMPORT_FILE_VERSION 1)

# Protect against multiple inclusion
set(_cmake_targets_defined "")
set(_cmake_targets_not_defined "")
set(_cmake_expected_targets "")
foreach(_cmake_expected_target IN ITEMS WithNames::WithNames)
  list(APPEND _cmake_expected_targets "${_cmake_expected_target}")
  if(TARGET "${_cmake_expected_target}")
    list(APPEND _cmake_targets_defined "${_cmake_expected_target}")
  else()
    list(APPEND _cmake_targets_not_defined "${_cmake_expected_target}")
  endif()
endforeach()
unset(_cmake_expected_target)
if(_cmake_targets_defined STREQUAL _cmake_expected_targets)
  unset(_cmake_targets_defined)
  unset(_cmake_targets_not_defined)
  unset(_cmake_expected_targets)
  unset(CMAKE_IMPORT_FILE_VERSION)
  cmake_policy(POP)
  return()
endif()
unset(_cmake_targets_defined)
unset(_cmake_targets_not_defined)
unset(_cmake_expected_targets)

# Create imported target WithNames::WithNames
add_library(WithNames::WithNames INTERFACE IMPORTED)

set_target_properties(WithNames::WithNames PROPERTIES
    INTERFACE_INCLUDE_DIRECTORIES "/usr/include/WithNames"
)

set(CMAKE_IMPORT_FILE_VERSION)
cmake_policy(POP)
