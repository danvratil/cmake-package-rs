use cmake_package::{find_cmake, find_package, Error, Version};
use scopeguard::defer;
use serial_test::serial;

mod common;

// Note: since the tests manipulate the PATH environment variable in order to fake the presence
// of cmake, they must be executed serially, otherwise, since they are all executed within the
// same process, they would interfere with each other.

#[test]
#[serial]
fn test_find_cmake() {
    let cmake = find_cmake().expect("Failed to find cmake");
    assert!(cmake.path.exists());
    assert!(cmake.path.is_file());
    assert!(cmake.version >= Version::parse("3.19").unwrap());
}

#[test]
#[serial]
fn test_cmake_not_in_path() {
    let path = std::env::var("PATH").expect("Path not set?");
    defer! {
        std::env::set_var("PATH", path);
    }

    std::env::set_var("PATH", "");
    match find_cmake().expect_err("cmake should not be found when PATH is empty") {
        Error::CMakeNotFound => {}
        err => panic!("Unexpected error: expected CMakeNotFound, got {:?}", err),
    }
}

#[test]
#[serial]
fn test_cmake_version_too_old() {
    let _path_guard = common::use_cmake("cmake_old");
    match find_cmake().expect_err("cmake should be too old") {
        Error::UnsupportedCMakeVersion => {}
        err => panic!(
            "Unexpected error: expected UnsupportedCMakeVersion, got {:?}",
            err
        ),
    }
}

#[test]
#[serial]
fn test_missing_find_package() {
    let _tmpdir = common::set_outdir();

    match find_package("ThisPackageDefinitelyDoesNotExist")
        .verbose()
        .find()
        .expect_err("Found a package that possibly cannot exist")
    {
        Error::PackageNotFound => {}
        err => panic!("Unexpected error: expected PackageNotFound, got {:?}", err),
    }
}

#[test]
#[serial]
#[cfg_attr(target_os = "windows", ignore = "Requires OpenSSL installed")]
fn test_find_openssl_release() {
    let _tmpdir = common::set_outdir();
    let _profile = common::set_profile(common::Profile::Release);

    let package = find_package("OpenSSL")
        .verbose()
        .find()
        .expect("Failed to find OpenSSL");
    assert_eq!(package.name, "OpenSSL");
    assert!(package.version.is_none()); // OpenSSL version detection simply doesn't work for some reason

    let target = package
        .target("OpenSSL::SSL")
        .expect("Failed to find OpenSSL::SSL target");
    assert_eq!(target.name, "OpenSSL::SSL");
    if cfg!(target_os = "linux") {
        assert_eq!(target.include_directories, ["/usr/include"]);
        assert_eq!(target.link_libraries.len(), 2);
        // The actual path will vary depending on the system, and the library may itself may have a
        // soname, so we just check for the presence of the library name.
        assert!(target
            .link_libraries
            .iter()
            .any(|lib| lib.contains("libcrypto.so")));
        assert!(target
            .link_libraries
            .iter()
            .any(|lib| lib.contains("libssl.so")));
    } else if cfg!(target_os = "windows") {
        assert_eq!(
            target.include_directories,
            ["C:/Program Files/OpenSSL-Win64/include"]
        );
        assert!(target.link_libraries.len() >= 2);
        assert!(target
            .link_libraries
            .iter()
            .any(|lib| lib.contains("libcrypto64MD.lib")));
        assert!(target
            .link_libraries
            .iter()
            .any(|lib| lib.contains("libssl64MD.lib")));
    }
}

#[test]
#[serial]
#[ignore = "Requires Qt installed"]
fn test_find_qt_release() {
    let _tmpdir = common::set_outdir();
    let _profile = common::set_profile(common::Profile::Release);

    let package = find_package("Qt6")
        .components(["Core".into(), "Gui".into(), "Widgets".into()])
        .version("6.2")
        .verbose()
        .find()
        .expect("Failed to find Qt6");
    assert_eq!(package.name, "Qt6");
    assert!(package.version.unwrap() >= Version::parse("6.2").unwrap());
    assert_eq!(
        package.components,
        Some(vec!["Core".into(), "Gui".into(), "Widgets".into()])
    );

    let core = package
        .target("Qt6::Core")
        .expect("Failed to find Qt6::Core target");
    assert_eq!(core.name, "Qt6::Core");
    if cfg!(target_os = "linux") {
        assert_eq!(
            core.include_directories,
            vec![
                "/usr/include/qt6".to_string(),
                "/usr/include/qt6/QtCore".to_string(),
                "/usr/lib/qt6/mkspecs/linux-g++".to_string()
            ]
        );
    } else if cfg!(target_os = "windows") {
        for def in ["QT_CORE_LIB", "WIN32", "WIN64"] {
            assert!(core.compile_definitions.contains(&def.to_string()));
        }
        for lib in ["Qt6Core.lib", "mpr", "userenv"] {
            assert!(core.link_libraries.iter().any(|l| l.contains(lib)))
        }
        assert_eq!(
            core.include_directories,
            [
                "C:/Qt/6.6.1/msvc2019_64/include".to_string(),
                "C:/Qt/6.6.1/msvc2019_64/include/QtCore".to_string(),
                "C:/Qt/6.6.1/msvc2019_64/mkspecs/win32-msvc".to_string()
            ]
        );
    }

    let gui = package
        .target("Qt6::Gui")
        .expect("Failed to find Qt6::Gui target");
    println!("gui: {:?}", gui);
    assert_eq!(gui.name, "Qt6::Gui");
    if cfg!(target_os = "linux") {
        assert_eq!(
            gui.compile_definitions,
            ["QT_CORE_LIB".to_string(), "QT_GUI_LIB".to_string()]
        );
        for lib in ["libQt6Core.so", "libQt6Gui.so", "libOpenGL.so", "libGLX.so"] {
            assert!(gui.link_libraries.iter().any(|l| l.contains(lib)))
        }
        assert_eq!(
            gui.include_directories,
            vec![
                "/usr/include".to_string(),
                "/usr/include/qt6".to_string(),
                "/usr/include/qt6/QtCore".to_string(),
                "/usr/include/qt6/QtGui".to_string(),
                "/usr/lib/qt6/mkspecs/linux-g++".to_string()
            ]
        );
    } else if cfg!(target_os = "windows") {
        for def in ["QT_CORE_LIB", "QT_GUI_LIB", "WIN32", "WIN64"] {
            assert!(gui.compile_definitions.contains(&def.to_string()));
        }
        for lib in ["Qt6Gui.lib", "Qt6Core.lib", "mpr", "userenv"] {
            assert!(gui.link_libraries.iter().any(|l| l.contains(lib)))
        }
        assert_eq!(
            gui.include_directories,
            [
                "C:/Qt/6.6.1/msvc2019_64/include".to_string(),
                "C:/Qt/6.6.1/msvc2019_64/include/QtCore".to_string(),
                "C:/Qt/6.6.1/msvc2019_64/include/QtGui".to_string(),
                "C:/Qt/6.6.1/msvc2019_64/mkspecs/win32-msvc".to_string()
            ]
        )
    }
}

#[test]
#[serial]
#[ignore = "Requires Qt installed"]
fn test_find_qt_debug() {
    let _tmpdir = common::set_outdir();
    let _profile = common::set_profile(common::Profile::Debug);

    let package = find_package("Qt6")
        .components(["Core".into()])
        .version("6.2")
        .verbose()
        .find()
        .expect("Failed to find Qt6");
    assert_eq!(package.name, "Qt6");
    assert!(package.version.unwrap() >= Version::parse("6.2").unwrap());

    let core = package
        .target("Qt6::Core")
        .expect("Failed to find Qt6::Core target");
    assert_eq!(core.name, "Qt6::Core");
    if cfg!(target_os = "windows") {
        for def in ["QT_CORE_LIB", "WIN32", "WIN64"] {
            assert!(core.compile_definitions.contains(&def.to_string()));
        }
        for lib in ["Qt6Cored.lib", "mpr", "userenv"] {
            assert!(core.link_libraries.iter().any(|l| l.contains(lib)))
        }
        assert_eq!(
            core.include_directories,
            vec![
                "C:/Qt/6.6.1/msvc2019_64/include".to_string(),
                "C:/Qt/6.6.1/msvc2019_64/include/QtCore".to_string(),
                "C:/Qt/6.6.1/msvc2019_64/mkspecs/win32-msvc".to_string()
            ]
        );
    } else {
        // Really only makes sense on Windows, on Linux the debug symbols are
        // usually in a separate debug file.
    }
}
