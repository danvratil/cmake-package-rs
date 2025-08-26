# SPDX-FileCopyrightText: 2024 Daniel Vrátil <dvratil@kde.org>
#
# SPDX-License-Identifier: MIT

cmake_minimum_required(VERSION ${CMAKE_MIN_VERSION})
if (NOT DEFINED OUTPUT_FILE)
    message(FATAL_ERROR "OUTPUT_FILE is not set")
endif()
set(version_json "{}")
set(version_variables "MAJOR" "MINOR" "PATCH")
foreach(x IN LISTS version_variables)
  string(JSON version_json SET ${version_json} "${x}" "${CMAKE_${x}_VERSION}")
endforeach()
file(WRITE ${OUTPUT_FILE} ${version_json})
