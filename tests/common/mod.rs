use itertools::chain;
use scopeguard::{guard, ScopeGuard};
use std::{iter::once, path::PathBuf};

pub fn use_cmake(name: &str) -> ScopeGuard<(), impl FnOnce(())> {
    let path = std::env::var("PATH").expect("Path not set?");
    let cmake_path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("common")
        .join(name);
    assert!(cmake_path.exists());
    assert!(cmake_path.is_dir());

    // Prepend the custom cmake path to the PATH in platform-independent way
    let modified_path =
        std::env::join_paths(chain!(once(cmake_path), std::env::split_paths(&path))).unwrap();

    std::env::set_var("PATH", modified_path);

    guard((), |_| {
        std::env::set_var("PATH", path);
    })
}

pub fn set_outdir() -> ScopeGuard<(), impl FnOnce(())> {
    std::env::set_var("OUT_DIR", std::env::temp_dir());

    guard((), |_| {
        std::env::remove_var("OUT_DIR");
    })
}

pub enum Profile {
    Debug,
    Release,
}

pub fn set_profile(profile: Profile) -> ScopeGuard<(), impl FnOnce(())> {
    std::env::set_var(
        "PROFILE",
        match profile {
            Profile::Debug => "debug",
            Profile::Release => "release",
        },
    );

    guard((), |_| {
        std::env::remove_var("PROFILE");
    })
}
