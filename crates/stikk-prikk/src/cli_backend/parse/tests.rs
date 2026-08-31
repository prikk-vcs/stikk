//! Golden-fixture tests for prikk output parsing (design TS-03).
//!
//! The fixture below is captured verbatim from `prikk status` at the audited revision. A parser
//! change that would misread it fails here.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

const STATUS_FIXTURE: &str = "\
prikk repository: /tmp/sample/.prikk
active WAL records: 0
trailing partial WAL bytes: 0
heads/main RefState: b8586806457aaab97190c7ccfeb3a8c152f9e140613b70cbb09ff2afd8a56c8e
queued patches: 0
status: multi-operation text diff minimization and plugins not yet implemented
";

#[test]
fn parses_a_clean_status() {
    let o = orientation(STATUS_FIXTURE).expect("status parses");
    assert_eq!(o.queued_patches, 0);
    assert_eq!(o.trailing_partial_wal_bytes, 0);
    assert_eq!(
        o.main_ref_state.as_deref(),
        Some("b8586806457aaab97190c7ccfeb3a8c152f9e140613b70cbb09ff2afd8a56c8e")
    );
}

#[test]
fn parses_queued_and_partial_counts() {
    let text = "\
queued patches: 3
trailing partial WAL bytes: 7
heads/main RefState: <none>
";
    let o = orientation(text).expect("parses");
    assert_eq!(o.queued_patches, 3);
    assert_eq!(o.trailing_partial_wal_bytes, 7);
    assert_eq!(o.main_ref_state, None);
}

#[test]
fn refuses_rather_than_guesses_on_a_missing_field() {
    // UD-02: an unrecognized shape is an environment fault, never a fabricated default.
    let text = "some unexpected prikk output with no queued patches line\n";
    let err = orientation(text).expect_err("must refuse");
    assert_eq!(err.class(), "environment");
}

#[test]
fn refuses_a_non_numeric_count() {
    let text = "queued patches: lots\ntrailing partial WAL bytes: 0\n";
    assert_eq!(orientation(text).unwrap_err().class(), "environment");
}
