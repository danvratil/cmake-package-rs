#----------------------------------------------------------------
# Generated CMake target import file for configuration "Debug".
#----------------------------------------------------------------

# Commands may need to know the format version.
set(CMAKE_IMPORT_FILE_VERSION 1)

# Import target "WithAsan::WithAsan" for configuration "Debug"
set_property(TARGET WithAsan::WithAsan APPEND PROPERTY IMPORTED_CONFIGURATIONS DEBUG)
set_target_properties(WithAsan::WithAsan PROPERTIES
    IMPORTED_LINK_DEPENDENT_LIBRARIES_DEBUG "OpenSSL::Crypto;OpenSSL::SSL"
    IMPORTED_LOCATION_DEBUG "${_IMPORT_PREFIX}/lib/libWithAsan.so"
    IMPORTED_SONAME_DEBUG "libWithAsan.so.1"
  )

  list(APPEND _cmake_import_check_targets WithAsan::WithAsan)
  list(APPEND _cmake_import_check_files_for_WithAsan::WithAsan)

# Commands beyond this point should not need to know the version.
set(CMAKE_IMPORT_FILE_VERSION)
