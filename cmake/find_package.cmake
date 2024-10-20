# SPDX-FileCopyrightText: 2024 Daniel Vrátil <dvratil@kde.org>
#
# SPDX-License-Identifier: MIT

#[===================================================================================[.rst
Find Package Cmake Script
--------------------------

This is a helper CMake script that is used to execute find_package() and collect
the results into a JSON file. The JSON file is then read and interpreted by the
Rust code.

This is basically a regular project CMakeLists.txt script (except it has documentation :)),
so it needs to be compiled as CMakeLists.txt into some temporary directory and then invoked
by running `cmake .` in that directory. Some additional arguments must be passed to cmake
in order to specify the package to find, the output file and optionally the version and
components to find.

Parameters
~~~~~~~~~~

``PACKAGE``
  The package name to find (required)
``OUTPUT_FILE``
  The file to write the JSON output to (required)
``VERSION``
  Minimum version of the package to find (optional)
``COMPONENTS``
  Semicolon-separated list of components to find (optional)
``TARGET``
  The target to resolve (optional)

To invoke the script, first copy it into a temporary directory and then run:

.. code-block:: bash

  cmake -DPACKAGE=foo \
        -DVERSION=1.2.3 \
        -DCOMPONENTS=bar;baz \
        -DOUTPUT_FILE=/path/to/output.json
        -B /path/to/tmp/dir/build
        /path/to/tmp/dir

When `TARGET` is not specified, the script will only call ``find_package()`` and write
a JSON file with the package name, discovered version and components. When ``TARGET``
is set, the script will find all the following properties for the target, and also for
recursively for all nested targets referenced by e.g. ``INTERFACE_LINK_LIBRARIES``
target property:

``NAME``
``LOCATION``
``LOCATION_Release``
``LOCATION_RelWithDebInfo``
``LOCATION_MinSizeRel``
``LOCATION_Debug``
``INTERFACE_COMPILE_DEFINITIONS``
``INTERFACE_COMPILE_OPTIONS``
``INTERFACE_INCLUDE_DIRECTORIES``
``INTERFACE_LINK_DIRECTORIES``
``INTERFACE_LINK_LIBRARIES``
``INTERFACE_LINK_OPTIONS``

Note that due to usage of ``find_package()`` it is not possible to run the script in CMake script
mode. It must be run in the standard "configure" mode.

#]===================================================================================]

cmake_minimum_required(VERSION ${CMAKE_MIN_VERSION})
# TODO: Make it possible to disable check for compilers (by passing `LANGUAGES NONE`)
# so that users do not need to have a C/C++ compiler installed.
# C compiler is required by FindThreads.cmake that is often used inside other package
# scripts.
project(cmake-package)


###################################################################################
# Invokes find_package() and writes the result into a JSON file.
# Parameters:
#   PACKAGE: The package name to find (required)
#   OUTPUT_FILE: The file to write the JSON output to (required)
#   VERSION: The minimum version of the package to find (optional)
#   COMPONENTS: The components to find (optional)
###################################################################################
function(find_package_wrapper)
    cmake_parse_arguments(FP "" "PACKAGE;VERSION;OUTPUT_FILE" "COMPONENTS" ${ARGN})
    if (NOT FP_PACKAGE)
        message(FATAL_ERROR "PACKAGE is not set")
    endif()
    if (NOT FP_OUTPUT_FILE)
        message(FATAL_ERROR "OUTPUT_FILE is not set")
    endif()

    # Don't specify the version here, even if FP_VERSION is set - we want to find the package
    # even if the version is too old in order to be able to return the found version back to
    # the Rust code.
    find_package(${FP_PACKAGE} COMPONENTS ${FP_COMPONENTS})
    # Package found?
    if (${FP_PACKAGE}_FOUND)
        # Write its name into the JSON
        string(JSON json SET "{ }" "name" "\"${FP_PACKAGE}\"")
        # If we also found a version, write its version
        if (${FP_PACKAGE}_VERSION)
            string(JSON json SET ${json} "version" "\"${${FP_PACKAGE}_VERSION}\"")
        endif()
        if (FP_COMPONENTS)
            string(REPLACE ";" "\",\"" component_array "${FP_COMPONENTS}")
            string(JSON json SET ${json} "components" "[\"${component_array}\"]")
        endif()

        file(WRITE ${FP_OUTPUT_FILE} ${json})
    else()
        # If not found, just output an empty JSON object, the rust code will interpret it as not found
        file(WRITE ${FP_OUTPUT_FILE} "{ }")
    endif()
endfunction()

###################################################################################
# For given target and a target property this function resolves the value of the
# property. It checks each value and if the value is in fact another target, it
# calls `resolve_deps_recursively()` on it to obtain all properties of the target,
# otherwise it just keeps the value. Generator expressions are ignored since they
# cannot be resolved at configuration time.
#
# The result is a list of either strings or JSON objects (as a string). It is stored
# in the provided `OUT_VAR` variable.
#
#
# Parameters:
#   TARGET: The target to resolve (required)
#   PROPERTY: The property to resolve (required)
#   OUT_VAR: The variable to store the result in (required)
###################################################################################
function(resolve_target_prop)
    cmake_parse_arguments(ARG "" "TARGET;PROPERTY;OUT_VAR" "" ${ARGN})
    # Read the property value
    get_target_property(prop_values ${ARG_TARGET} ${ARG_PROPERTY})
    # Check each value
    message(STATUS "${ARG_TARGET}: ${ARG_PROPERTY} = ${prop_values}")
    set(result)
    foreach(value ${prop_values})
        # If the value is actually another imported target, then recursive into it and obtain
        # all properties of the target. Don't recurse into ourselves.
        if (TARGET ${value} AND NOT ("${ARG_TARGET}" STREQUAL "${value}"))
            set(var)
            resolve_deps_recursively(TARGET ${value} OUTPUT_JSON var)
            list(APPEND result ${var})
        elseif(value AND NOT ("${value}" MATCHES "^\\$\\<.*")) # Ignore generator expressions
            # Otherwise just append the value to output the list
            list(APPEND result ${value})
        endif()
    endforeach()
    set(${ARG_OUT_VAR} ${result} PARENT_SCOPE)
endfunction()

###################################################################################
# Recursively resolves all properties of the given target and all targets that may
# be referenced by any of the properties of the target (see `resolve_target_prop()`).
# The result is a JSON object with all properties of the target.
#
# Parameters:
#   TARGET: The target to resolve (required)
#   OUTPUT_JSON: The variable to store the result in (required)
###################################################################################
function(resolve_deps_recursively)
    cmake_parse_arguments(ARG "" "TARGET;OUTPUT_JSON" "" ${ARGN})
    set(single_value_props
        NAME
        LOCATION
        IMPORTED_IMPLIB
        IMPORTED_NO_SONAME
    )
    set(multi_value_props
        INTERFACE_COMPILE_DEFINITIONS
        INTERFACE_COMPILE_OPTIONS
        INTERFACE_INCLUDE_DIRECTORIES
        INTERFACE_LINK_DIRECTORIES
        INTERFACE_LINK_LIBRARIES
        INTERFACE_LINK_LIBRARIES_DIRECT
        INTERFACE_LINK_DEPENDENT_LIBRARIES
        INTERFACE_LINK_OPTIONS
    )
    set(cfg_props
        LOCATION
        IMPORTED_IMPLIB
    )
    set(cfg_types
        Release
        RelWithDebInfo
        MinSizeRel
        Debug
    )
    foreach(cfg_prop ${cfg_props})
        foreach(config ${cfg_types})
            list(APPEND single_value_props "${cfg_prop}_${config}")
        endforeach()
    endforeach()

    set(json "{}")
    foreach(prop ${single_value_props})
        set(value)
        get_target_property(value ${ARG_TARGET} ${prop})
        message(STATUS "${ARG_TARGET}: ${prop} = ${value}")
        if (value)
            string(JSON json SET ${json} ${prop} "\"${value}\"")
        endif()
    endforeach()

    foreach(prop ${multi_value_props})
        set(value)
        resolve_target_prop(TARGET ${ARG_TARGET} PROPERTY ${prop} OUT_VAR value)
        if (value)
            list_to_json(json ${json} ${prop} value)
        endif()
    endforeach()
    set(${ARG_OUTPUT_JSON} ${json} PARENT_SCOPE)

endfunction()

###################################################################################
# Converts a list of strings into a JSON array and stores it in the provided JSON
# object.
#
# Parameters:
#   json_var: Output variable to store the resulting JSON into
#   json: String with a JSON object to append the array to
#   member: The member name of the array in the JSON object
#   list_var: Name of the variable containing the list of strings to convert to JSON
###################################################################################
function(list_to_json json_var json member list_var)
    set(i 0)
    string(JSON json SET ${json} "${member}" "[]")
    foreach(elem ${${list_var}})
        # Super-simple check if "elem" contains a JSON object, otherwise assume string
        if (elem MATCHES "^{.*")
            string(JSON json SET ${json} "${member}" "${i}" "${elem}")
        else()
            string(JSON json SET ${json} "${member}" "${i}" "\"${elem}\"")
        endif()
        math(EXPR i "${i} + 1")
    endforeach()

    set(${json_var} ${json} PARENT_SCOPE)
endfunction()

###################################################################################
# Invokes find_package(), locates the specified target and returns all relevant
# properties of the target and all targets that may be referenced by any of the
# properties (recursively).
#
# Parameters:
#   PACKAGE: The package name to find (required)
#   TARGET: The target to resolve (required)
#   OUTPUT_FILE: The file to write the JSON output to (required)
#   COMPONENTS: The components to find (optional)
#   VERSION: The minimum version of the package to find (optional)
###################################################################################
function (find_package_target)
    cmake_parse_arguments(ARG "" "PACKAGE;VERSION;TARGET;OUTPUT_FILE" "COMPONENTS" ${ARGN})
    if (NOT ARG_PACKAGE)
        message(FATAL_ERROR "PACKAGE argument is not set")
    endif()
    if (NOT ARG_TARGET)
        message(FATAL_ERROR "TARGET argument is not set")
    endif()
    if (NOT ARG_OUTPUT_FILE)
        message(FATAL_ERROR "OUTPUT_FILE argument is not set")
    endif()

    # It's safe to require the version here, we already found the package before and established
    # the version is recent enough.
    find_package(${ARG_PACKAGE} ${ARG_VERSION} COMPONENTS ${ARG_COMPONENTS})
    if (${ARG_PACKAGE}_FOUND)
        resolve_deps_recursively(
            TARGET ${ARG_TARGET}
            OUTPUT_JSON json
        )
        file(WRITE ${ARG_OUTPUT_FILE} ${json})
        message(STATUS "Target details written to ${ARG_OUTPUT_FILE}")
    else()
        # We found the package before, how come we did not find it this time?!
        message(FATAL_ERROR "Package ${FP_PACKAGE} not found")
    endif()
endfunction()





if (NOT DEFINED PACKAGE)
    message(FATAL_ERROR "PACKAGE is not set")
endif()

if (NOT DEFINED OUTPUT_FILE)
    message(FATAL_ERROR "OUTPUT_FILE is not set")
endif()

message(STATUS "CMAKE_BUILD_TYPE=${CMAKE_BUILD_TYPE}")

if (NOT DEFINED TARGET)
    find_package_wrapper(
        PACKAGE ${PACKAGE}
        COMPONENTS "${COMPONENTS}"
        VERSION ${VERSION}
        OUTPUT_FILE ${OUTPUT_FILE}
    )
else()
    find_package_target(
        PACKAGE ${PACKAGE}
        COMPONENTS "${COMPONENTS}"
        VERSION ${VERSION}
        TARGET ${TARGET}
        OUTPUT_FILE ${OUTPUT_FILE}
    )
endif()

include(FeatureSummary)

feature_summary(WHAT PACKAGES_FOUND PACKAGES_NOT_FOUND)
