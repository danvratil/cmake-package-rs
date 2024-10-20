// SPDX-FileCopyrightText: 2024 Daniel Vrátil <dvratil@kde.org>
//
// SPDX-License-Identifier: MIT

use crate::version::{Version, VersionError};
use crate::{CMakePackage, CMakeTarget};

use itertools::Itertools;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;
use which::which;

/// The minimum version of CMake required by this crate.
pub const CMAKE_MIN_VERSION: &str = "3.19";

/// A structure representing the CMake program found on the system.
#[derive(Debug, Clone)]
pub struct CMakeProgram {
    /// Path to the `cmake` executable to be used.
    pub path: PathBuf,
    /// Version of the `cmake` detected. Must be at least [`CMAKE_MIN_VERSION`].
    pub version: Version,
}

fn script_path(script: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cmake")
        .join(script)
}

/// Errors that can occur while working with CMake.
#[derive(Debug)]
pub enum Error {
    /// The `cmake` executable was not found in system `PATH` environment variable.
    CMakeNotFound,
    /// The available CMake version is too old (see [`CMAKE_MIN_VERSION`]).
    UnsupportedCMakeVersion,
    /// An internal error in the library.
    Internal,
    /// An I/O error while executing `cmake`
    IO(std::io::Error),
    /// An version-related error (e.g. the found package version is too old)
    Version(VersionError),
    /// The requested package was not found by CMake.
    PackageNotFound,
}

#[derive(Clone, Debug, Deserialize)]
struct PackageResult {
    name: Option<String>,
    version: Option<String>,
    components: Option<Vec<String>>,
}

/// Find the CMake program on the system and check version compatibility.
///
/// Tries to find the `cmake` executable in all paths listed in the `PATH` environment variable.
/// If found, it also checks that the version of CMake is at least [`CMAKE_MIN_VERSION`].
///
/// Returns [`CMakeProgram`] on success and [`Error::CMakeNotFound`] when the `cmake` executable
/// is not found or [`Error::UnsupportedCMakeVersion`] when the version is too low.
pub fn find_cmake() -> Result<CMakeProgram, Error> {
    let path = which("cmake").or(Err(Error::CMakeNotFound))?;

    let output = Command::new(&path)
        .arg("-P")
        .arg(script_path("cmake_version.cmake"))
        .output()
        .or(Err(Error::Internal))?;

    let version = String::from_utf8_lossy(&output.stderr)
        .trim()
        .to_string()
        .try_into()
        .or(Err(Error::UnsupportedCMakeVersion))?;

    if version
        < CMAKE_MIN_VERSION
            .try_into()
            .map_err(|_| Error::UnsupportedCMakeVersion)?
    {
        return Err(Error::UnsupportedCMakeVersion);
    }

    Ok(CMakeProgram { path, version })
}

fn get_temporary_working_directory() -> Result<TempDir, Error> {
    #[cfg(test)]
    let out_dir = std::env::temp_dir();
    #[cfg(not(test))]
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_else(|_| {
        panic!("OUT_DIR is not set, are you running the crate from build.rs?")
    }));

    // Make a unique directory inside
    tempfile::Builder::new()
        .prefix("cmake-package-rs")
        .tempdir_in(out_dir)
        .or(Err(Error::Internal))
}

fn setup_cmake_project(working_directory: &Path) -> Result<(), Error> {
    std::fs::copy(
        script_path("find_package.cmake"),
        working_directory.join("CMakeLists.txt"),
    )
    .map_err(Error::IO)?;
    Ok(())
}

fn stdio(verbose: bool) -> Stdio {
    if verbose {
        Stdio::inherit()
    } else {
        Stdio::null()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
enum CMakeBuildType {
    Debug,
    Release,
    RelWithDebInfo,
    MinSizeRel,
}

fn build_type() -> CMakeBuildType {
    // The PROFILE variable is set to "release" for release builds and to "debug" for any other build type.
    // This is fairly easy to map to CMake's build types...
    match std::env::var("PROFILE")
        .as_ref()
        .unwrap_or(&"debug".to_string())
        .as_str()
    {
        "release" => {
            // If the release profile is enabled, and also "s" or "z" optimimzation is set, meaning "optimize for binary size",
            // then we want to use MinSizeRel.
            // There's no way in CMake to combine MinSizeRel and RelWithDebInfo. Since those two options kinds contradict themselves,
            // we make the assumption here that if the user wants to optimize for binary size, they want that more than they want
            // debug info, so MinSizeRel is checked first.
            let opt_level = std::env::var("OPT_LEVEL").unwrap_or("0".to_string());
            if "sz".contains(&opt_level) {
                return CMakeBuildType::MinSizeRel;
            }

            // If DEBUG is set to anything other than "0", "false" or "none" (meaning to include /some/ kind of debug info),
            // then we want to use RelWithDebInfo.
            let debug = std::env::var("DEBUG").unwrap_or("0".to_string());
            if !["0", "false", "none"].contains(&debug.as_str()) {
                return CMakeBuildType::RelWithDebInfo;
            }

            // For everything else, there's Mastercard...I mean Release.
            CMakeBuildType::Release
        }
        // Any other profile (which really should only be "debug"), we map to Debug.
        _ => CMakeBuildType::Debug,
    }
}

/// Performs the actual `find_package()` operation with CMake
pub(crate) fn find_package(
    name: String,
    version: Option<Version>,
    components: Option<Vec<String>>,
    verbose: bool,
) -> Result<CMakePackage, Error> {
    // Find cmake or panic
    let cmake = find_cmake()?;

    let working_directory = get_temporary_working_directory()?;

    setup_cmake_project(working_directory.path())?;

    let output_file = working_directory.path().join("package.json");
    // Run the CMake - see the find_package.cmake script for docs
    let mut command = Command::new(&cmake.path);
    command
        .stdout(stdio(verbose))
        .stderr(stdio(verbose))
        .current_dir(&working_directory)
        .arg(".")
        .arg(format!("-DCMAKE_BUILD_TYPE={:?}", build_type()))
        .arg(format!("-DCMAKE_MIN_VERSION={CMAKE_MIN_VERSION}"))
        .arg(format!("-DPACKAGE={}", name))
        .arg(format!("-DOUTPUT_FILE={}", output_file.display()));
    if let Some(version) = version {
        command.arg(format!("-DVERSION={}", version));
    }
    if let Some(components) = components {
        command.arg(format!("-DCOMPONENTS={}", components.join(";")));
    }
    command.output().map_err(Error::IO)?;

    // Read from the generated JSON file
    let reader = std::fs::File::open(output_file).map_err(Error::IO)?;
    let package: PackageResult = serde_json::from_reader(reader).or(Err(Error::Internal))?;

    let package_name = match package.name {
        Some(name) => name,
        None => return Err(Error::PackageNotFound),
    };

    let package_version = match package.version {
        Some(version) => Some(version.try_into().map_err(Error::Version)?),
        None => None, // Missing version is not an error
    };

    if let Some(version) = version {
        if let Some(package_version) = package_version {
            if package_version < version {
                return Err(Error::Version(VersionError::VersionTooOld(package_version)));
            }
        }

        // It's not an error if the package did not provide a version.
    }

    Ok(CMakePackage::new(
        cmake,
        working_directory,
        package_name,
        package_version,
        package.components,
        verbose,
    ))
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum PropertyValue {
    String(String),
    Target(Target),
}

impl From<PropertyValue> for Vec<String> {
    fn from(value: PropertyValue) -> Self {
        match value {
            PropertyValue::String(value) => vec![value],
            PropertyValue::Target(target) => match target.location {
                Some(location) => vec![location],
                None => vec![],
            }
            .into_iter()
            .chain(
                target
                    .interface_link_libraries
                    .unwrap_or_default()
                    .into_iter()
                    .flat_map(Into::<Vec<String>>::into),
            )
            .collect(),
        }
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default, rename_all = "UPPERCASE")]
struct Target {
    name: String,
    location: Option<String>,
    #[serde(rename = "LOCATION_Release")]
    location_release: Option<String>,
    #[serde(rename = "LOCATION_Debug")]
    location_debug: Option<String>,
    #[serde(rename = "LOCATION_RelWithDebInfo")]
    location_relwithdebinfo: Option<String>,
    #[serde(rename = "LOCATION_MinSizeRel")]
    location_minsizerel: Option<String>,
    imported_implib: Option<String>,
    #[serde(rename = "IMPORTED_IMPLIB_Release")]
    imported_implib_release: Option<String>,
    #[serde(rename = "IMPORTED_IMPLIB_Debug")]
    imported_implib_debug: Option<String>,
    #[serde(rename = "IMPORTED_IMPLIB_RelWithDebInfo")]
    imported_implib_relwithdebinfo: Option<String>,
    #[serde(rename = "IMPORTED_IMPLIB_MinSizeRel")]
    imported_implib_minsizerel: Option<String>,
    interface_compile_definitions: Option<Vec<String>>,
    interface_compile_options: Option<Vec<String>>,
    interface_include_directories: Option<Vec<String>>,
    interface_link_directories: Option<Vec<String>>,
    interface_link_libraries: Option<Vec<PropertyValue>>,
    interface_link_options: Option<Vec<String>>,
}

/// Collects values from `property` of the current target and from `property` of
/// all targets linked in `interface_link_libraries` recursively.
///
/// This basically implements the CMake logic as described in the documentation
/// of e.g. [`INTERFACE_COMPILE_DEFINITIONS`][cmake_interface_compile_definitions] for
/// the target property:
///
/// > When target dependencies are specified using [`target_link_libraries()`][target_link_libraries],
/// > CMake will read this property from all target dependencies to determine the build properties of
/// > the consumer.
///
/// This function preserves the order of the values as they are found in the targets, the value of the
/// immediate `target` value is first, followed by all transitive properties of each linked target.
///
/// [cmake_interface_compile_definitions]: https://cmake.org/cmake/help/latest/prop_tgt/INTERFACE_COMPILE_DEFINITIONS.html
/// [target_link_libraries]: https://cmake.org/cmake/help/latest/command/target_link_libraries.html
fn collect_from_targets<'a>(
    target: &'a Target,
    property: impl Fn(&Target) -> &Option<Vec<String>> + 'a + Copy,
) -> Vec<String> {
    property(target)
        .as_ref()
        .map_or(Vec::new(), Clone::clone)
        .into_iter()
        .chain(
            target
                .interface_link_libraries
                .as_ref()
                .map_or(Vec::new(), Clone::clone)
                .iter()
                .filter_map(|value| match value {
                    PropertyValue::String(_) => None,
                    PropertyValue::Target(target) => Some(target),
                })
                .flat_map(|target| collect_from_targets(target, property)),
        )
        .collect()
}

/// Equivalent to `collect_from_target`, but it sorts and deduplicates the properties - use with
/// care, as the order of the properties might be important (e.g. for compile options).
fn collect_from_targets_unique<'a>(
    target: &'a Target,
    property: impl Fn(&Target) -> &Option<Vec<String>> + 'a + Copy,
) -> Vec<String> {
    collect_from_targets(target, property)
        .into_iter()
        .sorted()
        .dedup()
        .collect()
}

fn location_for_build_type(build_type: CMakeBuildType, target: &Target) -> Option<String> {
    if cfg!(target_os = "windows") {
        match build_type {
            CMakeBuildType::Debug => target.imported_implib_debug.clone().or(target.imported_implib.clone()),
            CMakeBuildType::Release => target.imported_implib_release.clone().or(target.imported_implib.clone()),
            CMakeBuildType::RelWithDebInfo => target
                .imported_implib_relwithdebinfo
                .clone()
                .or(target.imported_implib.clone()),
            CMakeBuildType::MinSizeRel => target
                .imported_implib_minsizerel
                .clone()
                .or(target.imported_implib.clone()),

        }
    } else {
        match build_type {
            CMakeBuildType::Debug => target.location_debug.clone().or(target.location.clone()),
            CMakeBuildType::Release => target.location_release.clone().or(target.location.clone()),
            CMakeBuildType::RelWithDebInfo => target
                .location_relwithdebinfo
                .clone()
                .or(target.location.clone()),
            CMakeBuildType::MinSizeRel => target
                .location_minsizerel
                .clone()
                .or(target.location.clone()),
        }
    }
}

impl Target {
    fn into_cmake_target(self, build_type: CMakeBuildType) -> CMakeTarget {
        CMakeTarget {
            compile_definitions: collect_from_targets_unique(&self, |target| {
                &target.interface_compile_definitions
            }),
            compile_options: collect_from_targets(&self, |target| {
                &target.interface_compile_options
            }),
            include_directories: collect_from_targets_unique(&self, |target| {
                &target.interface_include_directories
            }),
            link_directories: collect_from_targets_unique(&self, |target| {
                &target.interface_link_directories
            }),
            link_options: collect_from_targets(&self, |target| &target.interface_link_options),
            link_libraries: location_for_build_type(build_type, &self)
                .as_ref()
                .map_or(vec![], |location| vec![location.clone()])
                .into_iter()
                .chain(
                    self.interface_link_libraries
                        .as_ref()
                        .map_or(Vec::new(), Clone::clone)
                        .into_iter()
                        .flat_map(Into::<Vec<String>>::into),
                )
                .sorted() // FIXME: should we really do this for libraries? Linking order might be important...
                .dedup()
                .collect(),
            location: location_for_build_type(build_type, &self),
            name: self.name,
        }
    }
}

/// Finds the specified target in the CMake package and extracts its properties.
/// Returns `None` if the target was not found.
pub(crate) fn find_target(
    package: &CMakePackage,
    target: impl Into<String>,
) -> Option<CMakeTarget> {
    let target: String = target.into();

    // Run the CMake script
    let output_file = package.working_directory.path().join(format!(
        "target_{}.json",
        target.to_lowercase().replace(":", "_")
    ));
    let build_type = build_type();
    let mut command = Command::new(&package.cmake.path);
    command
        .stdout(stdio(package.verbose))
        .stderr(stdio(package.verbose))
        .current_dir(package.working_directory.path())
        .arg(".")
        .arg(format!("-DCMAKE_BUILD_TYPE={:?}", build_type))
        .arg(format!("-DCMAKE_MIN_VERSION={CMAKE_MIN_VERSION}"))
        .arg(format!("-DPACKAGE={}", package.name))
        .arg(format!("-DTARGET={}", target))
        .arg(format!("-DOUTPUT_FILE={}", output_file.display()));
    if let Some(version) = package.version {
        command.arg(format!("-DVERSION={}", version));
    }
    if let Some(components) = &package.components {
        command.arg(format!("-DCOMPONENTS={}", components.join(";")));
    }
    command.output().ok()?;

    // Read from the generated JSON file
    let reader = std::fs::File::open(&output_file).ok()?;
    let target: Target = serde_json::from_reader(reader)
        .map_err(|e| {
            eprintln!("Failed to parse target JSON: {:?}", e);
        })
        .ok()?;
    println!("Target: {:?}", target);
    Some(target.into_cmake_target(build_type))
}

#[cfg(test)]
mod testing {
    use scopeguard::{guard, ScopeGuard};
    use serial_test::serial;

    use super::*;

    #[test]
    fn from_target() {
        let target = Target {
            name: "my_target".to_string(),
            location: Some("/path/to/target.so".to_string()),
            interface_compile_definitions: Some(vec!["DEFINE1".to_string(), "DEFINE2".to_string()]),
            interface_compile_options: Some(vec!["-O2".to_string(), "-Wall".to_string()]),
            interface_include_directories: Some(vec!["/path/to/include".to_string()]),
            interface_link_directories: Some(vec!["/path/to/lib".to_string()]),
            interface_link_options: Some(vec!["-L/path/to/lib".to_string()]),
            interface_link_libraries: Some(vec![
                PropertyValue::String("library1".to_string()),
                PropertyValue::String("library2".to_string()),
                PropertyValue::Target(Target {
                    name: "dependency".to_string(),
                    location: Some("/path/to/dependency.so".to_string()),
                    interface_compile_definitions: Some(vec!["DEFINE3".to_string()]),
                    interface_compile_options: Some(vec!["-O3".to_string()]),
                    interface_include_directories: Some(vec![
                        "/path/to/dependency/include".to_string()
                    ]),
                    interface_link_directories: Some(vec!["/path/to/dependency/lib".to_string()]),
                    interface_link_options: Some(vec!["-L/path/to/dependency/lib".to_string()]),
                    interface_link_libraries: Some(vec![PropertyValue::String(
                        "dependency_library".to_string(),
                    )]),
                    ..Default::default()
                }),
            ]),
            ..Default::default()
        };

        let cmake_target: CMakeTarget = target.into_cmake_target(CMakeBuildType::Release);

        assert_eq!(cmake_target.name, "my_target");
        assert_eq!(cmake_target.location, Some("/path/to/target.so".into()));
        assert_eq!(
            cmake_target.compile_definitions,
            vec!["DEFINE1", "DEFINE2", "DEFINE3"]
        );
        assert_eq!(cmake_target.compile_options, vec!["-O2", "-Wall", "-O3"]);
        assert_eq!(
            cmake_target.include_directories,
            vec!["/path/to/dependency/include", "/path/to/include"]
        );
        assert_eq!(
            cmake_target.link_directories,
            vec!["/path/to/dependency/lib", "/path/to/lib"]
        );
        assert_eq!(
            cmake_target.link_options,
            vec!["-L/path/to/lib", "-L/path/to/dependency/lib"]
        );
        assert_eq!(
            cmake_target.link_libraries,
            vec![
                "/path/to/dependency.so",
                "/path/to/target.so",
                "dependency_library",
                "library1",
                "library2",
            ]
        );
    }

    #[test]
    fn from_debug_target() {
        let target = Target {
            name: "test_target".to_string(),
            location: Some("/path/to/target.so".to_string()),
            location_debug: Some("/path/to/target_debug.so".to_string()),
            ..Default::default()
        };

        let cmake_target = target.into_cmake_target(CMakeBuildType::Debug);
        assert_eq!(
            cmake_target.location,
            Some("/path/to/target_debug.so".to_string())
        );
    }

    #[test]
    fn from_json() {
        let json = r#"
{
  "INTERFACE_INCLUDE_DIRECTORIES" : [ "/usr/include" ],
  "INTERFACE_LINK_LIBRARIES" :
  [
    {
      "INTERFACE_INCLUDE_DIRECTORIES" : [ "/usr/include" ],
      "LOCATION" : "/usr/lib/libcrypto.so",
      "NAME" : "OpenSSL::Crypto"
    }
  ],
  "LOCATION" : "/usr/lib/libssl.so",
  "NAME" : "OpenSSL::SSL"
}
"#;
        let target: Target = serde_json::from_str(json).expect("Failed to parse JSON");
        assert_eq!(target.name, "OpenSSL::SSL");
        assert_eq!(target.location, Some("/usr/lib/libssl.so".to_string()));
        assert_eq!(
            target.interface_include_directories,
            Some(vec!["/usr/include".to_string()])
        );
        assert!(target.interface_link_libraries.is_some());
        assert_eq!(target.interface_link_libraries.as_ref().unwrap().len(), 1);
        let sub_target = target
            .interface_link_libraries
            .as_ref()
            .unwrap()
            .first()
            .unwrap();
        match sub_target {
            PropertyValue::Target(sub_target) => {
                assert_eq!(sub_target.name, "OpenSSL::Crypto");
                assert_eq!(
                    sub_target.location,
                    Some("/usr/lib/libcrypto.so".to_string())
                );
            }
            _ => panic!("Expected PropertyValue::Target"),
        }
    }

    fn clear_env(name: &'static str) -> ScopeGuard<(), impl FnOnce(())> {
        let value = std::env::var(name);
        std::env::remove_var(name);
        guard((), move |_| {
            if let Ok(value) = value {
                std::env::set_var(name, value);
            } else {
                std::env::remove_var(name);
            }
        })
    }

    #[test]
    #[serial]
    fn test_build_type() {
        let _profile = clear_env("PROFILE");
        let _debug = clear_env("DEBUG");
        let _opt_level = clear_env("OPT_LEVEL");

        assert_eq!(build_type(), CMakeBuildType::Debug);

        std::env::set_var("PROFILE", "release");
        assert_eq!(build_type(), CMakeBuildType::Release);

        std::env::set_var("DEBUG", "1");
        assert_eq!(build_type(), CMakeBuildType::RelWithDebInfo);

        std::env::set_var("DEBUG", "0");
        std::env::set_var("OPT_LEVEL", "s");
        assert_eq!(build_type(), CMakeBuildType::MinSizeRel);
    }
}
