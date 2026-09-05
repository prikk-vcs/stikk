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

/// RFC 009 decision 6: the floor moved to 0.28 — 0.27.x is dropped because `worktree-status` is the
/// UD-03 defect there and stikk already refuses to run it.
#[test]
fn supported_range_starts_at_0_28() {
    assert!(
        !Version {
            major: 0,
            minor: 27,
            patch: 1
        }
        .is_supported()
    );
    assert!(
        Version {
            major: 0,
            minor: 28,
            patch: 0
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
    // A 1.x prikk is a different format contract; not assumed supported.
    assert!(
        !Version {
            major: 1,
            minor: 0,
            patch: 0
        }
        .is_supported()
    );
}

/// RFC 009 decision 7 (ceiling raised to 0.31 by RFC 012 F-e): a prikk above the validated ceiling
/// still runs (`is_supported`), but `is_validated` says its output shapes have not actually been
/// checked.
#[test]
fn validated_ceiling_is_0_31_but_newer_still_runs() {
    let below_floor = Version {
        major: 0,
        minor: 27,
        patch: 1,
    };
    assert!(!below_floor.is_supported() && !below_floor.is_validated());

    let at_floor = Version {
        major: 0,
        minor: 28,
        patch: 0,
    };
    assert!(at_floor.is_supported() && at_floor.is_validated());

    let at_ceiling = Version {
        major: 0,
        minor: 31,
        patch: 0,
    };
    assert!(at_ceiling.is_supported() && at_ceiling.is_validated());

    let above_ceiling = Version {
        major: 0,
        minor: 32,
        patch: 0,
    };
    assert!(above_ceiling.is_supported() && !above_ceiling.is_validated());
}
