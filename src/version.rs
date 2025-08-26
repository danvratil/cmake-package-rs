// SPDX-FileCopyrightText: 2024 Daniel Vrátil <dvratil@kde.org>
//
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, serde::Deserialize)]
#[serde(rename_all="UPPERCASE")]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VersionError {
    InvalidVersion,
    VersionTooOld(Version),
}

impl Version {
    pub fn parse(version: &str) -> Result<Version, VersionError> {
        let parts: Vec<&str> = version.split(&['.', '-']).collect();
        if parts.is_empty() {
            return Err(VersionError::InvalidVersion);
        }

        Ok(Version {
            major: parts[0].parse().or(Err(VersionError::InvalidVersion))?,
            minor: if parts.len() > 1 {
                parts[1].parse().or(Err(VersionError::InvalidVersion))?
            } else {
                0
            },
            patch: if parts.len() > 2 {
                parts[2].parse().or(Err(VersionError::InvalidVersion))?
            } else {
                0
            },
        })
    }
}

impl TryInto<Version> for &str {
    type Error = VersionError;

    fn try_into(self) -> Result<Version, Self::Error> {
        Version::parse(self)
    }
}

impl TryInto<Version> for String {
    type Error = VersionError;

    fn try_into(self) -> Result<Version, Self::Error> {
        Version::parse(&self)
    }
}

impl From<Version> for String {
    fn from(value: Version) -> Self {
        format!("{}.{}.{}", value.major, value.minor, value.patch)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod testing {
    use super::*;

    #[test]
    fn test_version_parse_valid() {
        assert_eq!(
            Version::parse("1.2.3").unwrap(),
            Version {
                major: 1,
                minor: 2,
                patch: 3
            }
        );
        assert_eq!(
            Version::parse("1.2").unwrap(),
            Version {
                major: 1,
                minor: 2,
                patch: 0
            }
        );
        assert_eq!(
            Version::parse("1").unwrap(),
            Version {
                major: 1,
                minor: 0,
                patch: 0
            }
        );
        assert_eq!(
            Version::parse("4.0.20250530-gab5866c").unwrap(),
            Version {
                major: 4,
                minor: 0,
                patch: 20250530
            }
        )
    }

    #[test]
    fn test_version_parse_invalid() {
        assert!(Version::parse("").is_err());
        assert!(Version::parse("1.2.3:4").is_err());
        assert!(Version::parse("a.b.c").is_err());
    }

    #[test]
    fn test_version_into_string() {
        let version = Version {
            major: 1,
            minor: 2,
            patch: 3,
        };
        let version_str: String = version.into();
        assert_eq!(version_str, "1.2.3");
    }

    #[test]
    fn test_version_partial_ord() {
        let v1 = Version {
            major: 1,
            minor: 0,
            patch: 0,
        };
        let v2 = Version {
            major: 1,
            minor: 1,
            patch: 0,
        };
        let v3 = Version {
            major: 1,
            minor: 1,
            patch: 1,
        };

        let v4 = Version {
            major: 2,
            minor: 0,
            patch: 0,
        };

        assert!(v1 < v2);
        assert!(v1 <= v2);
        assert!(v2 < v3);
        assert!(v2 <= v3);
        assert!(v1 < v3);
        assert!(v1 <= v3);
        assert!(v3 > v2);
        assert!(v3 >= v2);
        assert!(v2 > v1);
        assert!(v2 >= v1);
        assert!(v3 > v1);
        assert!(v3 >= v1);
        assert!(v1 < v4);
        assert!(v1 <= v4);
        assert!(v4 > v1);
        assert!(v4 >= v1);
        assert!(v2 < v4);
        assert!(v2 <= v4);
        assert!(v4 > v2);
        assert!(v4 >= v2);
        assert!(v3 < v4);
        assert!(v3 <= v4);
        assert!(v4 > v3);
        assert!(v4 >= v3);
    }

    #[test]
    fn test_version_partial_eq() {
        let v1 = Version {
            major: 1,
            minor: 0,
            patch: 0,
        };
        let v2 = Version {
            major: 1,
            minor: 0,
            patch: 0,
        };
        let v3 = Version {
            major: 1,
            minor: 1,
            patch: 0,
        };

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_version_try_into() {
        let version_str = "1.2.3";
        let version: Version = version_str.try_into().unwrap();
        assert_eq!(
            version,
            Version {
                major: 1,
                minor: 2,
                patch: 3
            }
        );

        let version_string = String::from("1.2.3");
        let version: Version = version_string.try_into().unwrap();
        assert_eq!(
            version,
            Version {
                major: 1,
                minor: 2,
                patch: 3
            }
        );
    }

    #[test]
    fn test_display() {
        let version = Version {
            major: 1,
            minor: 2,
            patch: 3,
        };
        assert_eq!(format!("{}", version), "1.2.3");
    }
}
