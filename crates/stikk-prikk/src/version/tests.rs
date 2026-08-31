//! Golden-fixture tests for version parsing (design TS-03).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn parses_prikk_version_line() {
    // Captured from `prikk --version` at the audited revision.
    let v = Version::parse_version_line("prikk 0.27.1").expect("valid version");
    assert_eq!(
        v,
        Version {
            major: 0,
            minor: 27,
            patch: 1
        }
    );
    assert_eq!(v.to_string(), "0.27.1");
}

#[test]
fn tolerates_whitespace_and_bare_triple() {
    assert_eq!(Version::parse_version_line("  0.27.1  ").unwrap().minor, 27);
}

#[test]
fn strips_a_prerelease_suffix_on_patch() {
    let v = Version::parse_version_line("prikk 0.28.0-rc1").unwrap();
    assert_eq!(v.patch, 0);
    assert_eq!(v.minor, 28);
}

#[test]
fn rejects_output_with_no_version() {
    assert!(Version::parse_version_line("prikk: command not found").is_err());
}

#[test]
fn supported_range_is_0_27_and_up_within_0_x() {
    assert!(
        Version {
            major: 0,
            minor: 27,
            patch: 1
        }
        .is_supported()
    );
    assert!(
        Version {
            major: 0,
            minor: 30,
            patch: 0
        }
        .is_supported()
    );
    // Below the floor: not validated.
    assert!(
        !Version {
            major: 0,
            minor: 21,
            patch: 0
        }
        .is_supported()
    );
    // A 1.x prikk is a different format contract; not assumed supported by v0.1.
    assert!(
        !Version {
            major: 1,
            minor: 0,
            patch: 0
        }
        .is_supported()
    );
}
