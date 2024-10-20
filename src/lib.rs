// SPDX-FileCopyrightText: 2024 Daniel Vrátil <dvratil@kde.org>
//
// SPDX-License-Identifier: MIT

//! A simple CMake package finder.
//!
//! This crate is intended to be used in [cargo build scripts][cargo_build_script] to obtain
//! information about and existing system [CMake package][cmake_package] and [CMake targets][cmake_target]
//! defined in the package, such as include directories and link libraries for individual
//! CMake targets defined in the package.
//!
//! The crate runs the `cmake` command in the background to query the system for the package
//! and to extract the necessary information, which means that in order for this crate to work,
//! the `cmake` executable must be located the system [`PATH`][wiki_path]. CMake version
//! [3.19][CMAKE_MIN_VERSION] or newer is required for this crate to work.
//!
//! The entry point for the crate is the [`find_package()`] function that returns a builder,
//! which you can use to specify further constraints on the package ([version][FindPackageBuilder::version]
//! or [components][FindPackageBuilder::components]). Once you call the [`find()`][FindPackageBuilder::find]
//! method on the builder, the crate will try to find the package on the system or return an
//! error. If the package is found, an instance of the [`CMakePackage`] struct is returned that
//! contains information about the package. Using its [`target()`][CMakePackage::target] method,
//! you can query information about individual CMake targets defined in the package.
//!
//! If you want to make your dependency on CMake optional, you can use the [`find_cmake()`]
//! function to check that a suitable version of CMake is found on the system and then decide
//! how to proceed yourself. It is not necessary to call the function before using [`find_package()`].
//!
//! # Example
//! ```no_run
//! use cmake_package::find_package;
//!
//! let package = find_package("OpenSSL").version("1.0").find();
//! let target = match package {
//!     Err(_) => panic!("OpenSSL>=1.0 not found"),
//!     Ok(package) => {
//!         package.target("OpenSSL::SSL").unwrap()
//!     }
//! };
//!
//! println!("Include directories: {:?}", target.include_directories);
//! target.link();
//! ```
//!
//! # How Does it Work?
//!
//! When you call [`FindPackageBuilder::find()`], the crate will create a temporary directory
//! with a `CMakeLists.txt` file that contains actual [`find_package()`][cmake_find_package]
//! command to search for the package. The crate will then run actual `cmake` command in the
//! temporary directory to let CMake find the package. The `CMakeLists.txt` then writes the
//! information about the package into a JSON file that is then read by this crate to produce
//! the [`CMakePackage`].
//!
//! When a target is queried using the [`CMakePackage::target()`] method, the crate runs the
//! CMake command again the same directory, but this time the `CMakeLists.txt` attempts to locate
//! the specified CMake target and list all its (relevant) properties and properties of all its
//! transitive dependencies. The result is again written into a JSON file that is then processed
//! by the crate to produce the [`CMakeTarget`] instance.
//!
//! # Known Limitations
//!
//! The crate currently supporst primarily linking against shared libraries. Linking against
//! static libraries is not tested and may not work as expected. The crate currently does not
//! support linking against MacOS frameworks.
//!
//! [CMake generator expressions][cmake_generator_expr] are not supported in property values
//! right now, because they are evaluated at later stage of the build, not during the "configure"
//! phase of CMake, which is what this crate does. Some generator expressions could be supported
//! by the crate in the future (e.g. by evaluating them ourselves).
//!
//! There's currently no way to customize the `CMakeLists.txt` file that is used to query the
//! package or the target in order to extract non-standard properties or variables set by
//! the CMake package. This may be addressed in the future.
//!
//! [wiki_path]: https://en.wikipedia.org/wiki/PATH_(variable)
//! [cmake_package]: https://cmake.org/cmake/help/latest/manual/cmake-packages.7.html
//! [cmake_target]: https://cmake.org/cmake/help/latest/manual/cmake-buildsystem.7.html#target-build-specification
//! [cargo_build_script]: https://doc.rust-lang.org/cargo/reference/build-scripts.html
//! [cmake_find_package]: https://cmake.org/cmake/help/latest/command/find_package.html
//! [cmake_generator_expr]: https://cmake.org/cmake/help/latest/manual/cmake-generator-expressions.7.html

use std::io::Write;

use regex::Regex;
use tempfile::TempDir;

mod cmake;
mod version;

pub use cmake::{find_cmake, CMakeProgram, Error, CMAKE_MIN_VERSION};
pub use version::{Version, VersionError};

/// A CMake package found on the system.
///
/// Represents a CMake package found on the system. To find a package, use the [`find_package()`] function.
/// The package can be queried for information about its individual CMake targets by [`CMakePackage::target()`].
///
/// # Example
/// ```no_run
/// use cmake_package::{CMakePackage, find_package};
///
/// let package: CMakePackage = find_package("OpenSSL").version("1.0").find().unwrap();
/// ```
#[derive(Debug)]
pub struct CMakePackage {
    cmake: CMakeProgram,
    working_directory: TempDir,
    verbose: bool,

    /// Name of the CMake package
    pub name: String,
    /// Version of the package found on the system
    pub version: Option<Version>,
    /// Components of the package, as requested by the user in [`find_package()`]
    pub components: Option<Vec<String>>,
}

impl CMakePackage {
    fn new(
        cmake: CMakeProgram,
        working_directory: TempDir,
        name: String,
        version: Option<Version>,
        components: Option<Vec<String>>,
        verbose: bool,
    ) -> Self {
        Self {
            cmake,
            working_directory,
            name,
            version,
            components,
            verbose,
        }
    }

    /// Queries the CMake package for information about a specific [CMake target][cmake_target].
    /// Returns `None` if the target is not found in the package.
    ///
    /// [cmake_target]: https://cmake.org/cmake/help/latest/manual/cmake-buildsystem.7.html#imported-targets
    pub fn target(&self, target: impl Into<String>) -> Option<CMakeTarget> {
        cmake::find_target(self, target)
    }
}

/// Describes a CMake target found in a CMake package.
///
/// The target can be obtained by calling the [`target()`][CMakePackage::target()] method on a [`CMakePackage`] instance.
///
/// Use [`link()`][Self::link()] method to instruct cargo to link the final binary against the target.
/// There's currently no way to automatically apply compiler arguments or include directories, since
/// that depends on how the C/C++ code in your project is compiled (e.g. using the [cc][cc_crate] crate).
/// Optional support for this may be added in the future.
///
/// # Example
/// ```no_run
/// use cmake_package::find_package;
///
/// let package = find_package("OpenSSL").version("1.0").find().unwrap();
/// let target = package.target("OpenSSL::SSL").unwrap();
/// println!("Include directories: {:?}", target.include_directories);
/// println!("Link libraries: {:?}", target.link_libraries);
/// target.link();
/// ```
///
/// [cc_crate]: https://crates.io/crates/cc
#[derive(Debug, Default, Clone)]
pub struct CMakeTarget {
    /// Name of the CMake target
    pub name: String,
    /// Location of the target's binary (library or executable)
    pub location: Option<String>,
    /// List of public compile definitions requirements for a library.
    ///
    /// Contains preprocessor definitions provided by the target and all its transitive dependencies
    /// via their [`INTERFACE_COMPILE_DEFINITIONS`][cmake_interface_compile_definitions] target properties.
    ///
    /// [cmake_interface_compile_definitions]: https://cmake.org/cmake/help/latest/prop_tgt/INTERFACE_COMPILE_DEFINITIONS.html
    pub compile_definitions: Vec<String>,
    /// List of options to pass to the compiler.
    ///
    /// Contains compiler options provided by the target and all its transitive dependencies via
    /// their [`INTERFACE_COMPILE_OPTIONS`][cmake_interface_compile_options] target properties.
    ///
    /// [cmake_interface_compile_options]: https://cmake.org/cmake/help/latest/prop_tgt/INTERFACE_COMPILE_OPTIONS.html
    pub compile_options: Vec<String>,
    /// List of include directories required to build the target.
    ///
    /// Contains include directories provided by the target and all its transitive dependencies via
    /// their [`INTERFACE_INCLUDE_DIRECTORIES`][cmake_interface_include_directories] target properties.
    ///
    /// [cmake_interface_include_directories]: https://cmake.org/cmake/help/latest/prop_tgt/INTERFACE_INCLUDE_DIRECTORIES.html
    pub include_directories: Vec<String>,
    /// List of directories to use for the link step of shared library, module and executable targets.
    ///
    /// Contains link directories provided by the target and all its transitive dependencies via
    /// their [`INTERFACE_LINK_DIRECTORIES`][cmake_interface_link_directories] target properties.
    ///
    /// [cmake_interface_link_directories]: https://cmake.org/cmake/help/latest/prop_tgt/INTERFACE_LINK_DIRECTORIES.html
    pub link_directories: Vec<String>,
    /// List of target's direct link dependencies, followed by indirect dependencies from the transitive closure of the direct
    /// dependencies' [`INTERFACE_LINK_LIBRARIES`][cmake_interface_link_libraries] properties
    ///
    /// [cmake_interface_link_libraries]: https://cmake.org/cmake/help/latest/prop_tgt/INTERFACE_LINK_LIBRARIES.html
    pub link_libraries: Vec<String>,
    /// List of options to use for the link step of shared library, module and executable targets as well as the device link step.
    ///
    /// Contains link options provided by the target and all its transitive dependencies via
    /// their [`INTERFACE_LINK_OPTIONS`][cmake_interface_link_options] target properties.
    ///
    /// [cmake_interface_link_options]: https://cmake.org/cmake/help/latest/prop_tgt/INTERFACE_LINK_OPTIONS.html
    pub link_options: Vec<String>,
}

/// Turns /usr/lib/libfoo.so.5 into foo, so that -lfoo rather than -l/usr/lib/libfoo.so.5
/// is passed to the linker.
#[cfg(target_os = "linux")]
fn link_name(lib: &str) -> Option<&str> {
    let regex = Regex::new(r"lib([^/]+)\.so.*").ok()?;
    regex.captures(lib)?.get(1).map(|f| f.as_str())
}


#[cfg(target_os = "windows")]
fn link_name(lib: &str) -> Option<&str> {
    Some(lib)
}

impl CMakeTarget {
    /// Instructs cargo to link the final binary against the target.
    ///
    /// This method prints the necessary [`cargo:rustc-link-search=native={}`][cargo_rustc_link_search],
    /// [`cargo:rustc-link-arg={}`][cargo_rustc_link_arg], and [`cargo:rustc-link-lib=dylib={}`][cargo_rustc_link_lib]
    /// directives to the standard output for each of the target's [`link_directories`][Self::link_directories],
    /// [`link_options`][Self::link_options], and [`link_libraries`][Self::link_libraries] respectively.
    ///
    /// [cargo_rustc_link_search]: https://doc.rust-lang.org/cargo/reference/build-scripts.html#rustc-link-search
    /// [cargo_rustc_link_arg]: https://doc.rust-lang.org/cargo/reference/build-scripts.html#rustc-link-arg
    /// [cargo_rustc_link_lib]: https://doc.rust-lang.org/cargo/reference/build-scripts.html#rustc-link-lib]
    pub fn link(&self) {
        self.link_write(&mut std::io::stdout());
    }

    fn link_write<W: Write>(&self, io: &mut W) {
        self.link_directories.iter().for_each(|dir| {
            writeln!(io, "cargo:rustc-link-search=native={}", dir).unwrap();
        });
        self.link_options.iter().for_each(|opt| {
            writeln!(io, "cargo:rustc-link-arg={}", opt).unwrap();
        });
        self.link_libraries.iter().for_each(|lib| {
            match link_name(lib) {
                Some(lib) => writeln!(io, "cargo:rustc-link-lib=dylib={}", lib).unwrap(),
                None => writeln!(io, "cargo:rustc-link-arg={}", lib).unwrap(),
            }
        });
    }
}

/// A builder for creating a [`CMakePackage`] instance. An instance of the builder is created by calling
/// the [`find_package()`] function. Once the package is configured, [`FindPackageBuilder::find()`] will actually
/// try to find the CMake package and return a [`CMakePackage`] instance (or error if the package is not found
/// or an error occurs during the search).
#[derive(Debug, Clone)]
pub struct FindPackageBuilder {
    name: String,
    version: Option<Version>,
    components: Option<Vec<String>>,
    verbose: bool,
}

impl FindPackageBuilder {
    fn new(name: String) -> Self {
        Self {
            name,
            version: None,
            components: None,
            verbose: false,
        }
    }

    /// Optionally specifies the minimum required version for the package to find.
    /// If the package is not found or the version is too low, the `find()` method will return
    /// [`Error::Version`] with the version of the package found on the system.
    pub fn version(self, version: impl TryInto<Version>) -> Self {
        Self {
            version: Some(
                version
                    .try_into()
                    .unwrap_or_else(|_| panic!("Invalid version specified!")),
            ),
            ..self
        }
    }

    /// Optionally specifies the required components to locate in the package.
    /// If the package is found, but any of the components is missing, the package is considered
    /// as not found and the `find()` method will return [`Error::PackageNotFound`].
    /// See the documentation on CMake's [`find_package()`][cmake_find_package] function and how it
    /// treats the `COMPONENTS` argument.
    ///
    /// [cmake_find_package]: https://cmake.org/cmake/help/latest/command/find_package.html
    pub fn components(self, components: impl Into<Vec<String>>) -> Self {
        Self {
            components: Some(components.into()),
            ..self
        }
    }

    /// Enable verbose output.
    /// This will redirect output from actual execution of the `cmake` command to the standard output
    /// and standard error of the build script.
    pub fn verbose(self) -> Self {
        Self {
            verbose: true,
            ..self
        }
    }

    /// Tries to find the CMake package on the system.
    /// Returns a [`CMakePackage`] instance if the package is found, otherwise an error.
    pub fn find(self) -> Result<CMakePackage, cmake::Error> {
        cmake::find_package(self.name, self.version, self.components, self.verbose)
    }
}

/// Find a CMake package on the system.
///
/// This function is the main entrypoint for the crate. It returns a builder object that you
/// can use to specify further constraints on the package to find, such as the [version][FindPackageBuilder::version]
/// or [components][FindPackageBuilder::components]. Once you call the [`find()`][FindPackageBuilder::find]
/// method on the builder, the crate will try to find the package on the system or return an
/// error if the package does not exist or does not satisfy some of the constraints. If the package
/// is found, an instance of the [`CMakePackage`] struct is returned that can be used to further
/// query the package for information about its individual CMake targets.
///
/// See the documentation for [`FindPackageBuilder`], [`CMakePackage`], and [`CMakeTarget`] for more
/// information and the example in the crate documentation for a simple usage example.
pub fn find_package(name: impl Into<String>) -> FindPackageBuilder {
    FindPackageBuilder::new(name.into())
}

#[cfg(test)]
mod testing {
    use super::*;

    // Note: requires cmake to be installed on the system
    #[test]
    fn test_find_package() {
        let package = find_package("totallynonexistentpackage").find();
        match package {
            Ok(_) => panic!("Package should not be found"),
            Err(cmake::Error::PackageNotFound) => (),
            Err(err) => panic!("Unexpected error: {:?}", err),
        }
    }

    // Note: requires cmake to be installed on the system
    #[test]
    fn test_find_package_with_version() {
        let package = find_package("foo").version("1.0").find();
        match package {
            Ok(_) => panic!("Package should not be found"),
            Err(cmake::Error::PackageNotFound) => (),
            Err(err) => panic!("Unexpected error: {:?}", err),
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_link_to() {
        let target = CMakeTarget {
            name: "foo".into(),
            location: None,
            compile_definitions: vec![],
            compile_options: vec![],
            include_directories: vec![],
            link_directories: vec!["/usr/lib64".into()],
            link_libraries: vec!["/usr/lib/libbar.so".into(), "/usr/lib64/libfoo.so.5".into()],
            link_options: vec![],
        };

        let mut buf = Vec::new();
        target.link_write(&mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert_eq!(
            output.lines().collect::<Vec<&str>>(),
            vec![
                "cargo:rustc-link-search=native=/usr/lib64",
                "cargo:rustc-link-lib=dylib=bar",
                "cargo:rustc-link-lib=dylib=foo"
            ]
        );
    }
}
