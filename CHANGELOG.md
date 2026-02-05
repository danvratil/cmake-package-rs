# Changelog

## 0.1.11 (2026-05-02)
* Add support for specifying CMAKE_PREFIX_PATH (#105)
* Update dependencies

## 0.1.10 (2025-12-10)
* Support absolute library path on Darwin (#95)
* Update dependencies

## 0.1.9 (2025-08-30)
* Avoid cmake version parsing in Rust (#83)
* Update dependencies

## 0.1.8 (2025-07-17)
* Fix how versions are compared, required for CMake 4.x.x (#76)
* Update dependencies

## 0.1.7 (2025-05-30)

* Fix parsing of cmake git versions (#70)
* Update dependencies

## 0.1.6 (2025-03-17)

* Fix build on MacOS (#61), add MacOS CI
* Update dependencies

## 0.1.5 (2025-01-15)

* Expose the LOCATION property of targets in public API (#48)
* Actually return None when failing to find target (#47)
* Updated dependencies

## 0.1.4 (2024-12-27)

* Set CLICOLOR to 0 when reading CMake version (#36) 
* Dependency updates

## 0.1.3 (2024-11-26)

* Fix library names being misinterpreted as linker args (#27)
* Extract rustc-link-search from library path (#25)

## 0.1.2 (2024-10-24)

* Fix arguments passed to the linker on both platforms

## 0.1.1 (2024-10-19)

* Fix typos and links in documentation

## 0.1.0 (2024-10-19)

* Initial release 🎉
