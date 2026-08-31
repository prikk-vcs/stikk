//! Tests for the error taxonomy (design `stikk-04` TS-01/TS-05: the class → presentation contract).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn every_class_has_a_stable_name() {
    let cases: [(StikkError, &str); 7] = [
        (
            StikkError::Refusal {
                message: "m".into(),
            },
            "refusal",
        ),
        (
            StikkError::LockConflict {
                message: "m".into(),
            },
            "lock-conflict",
        ),
        (StikkError::NotReady { detail: "m".into() }, "not-ready"),
        (
            StikkError::IntegrityFinding {
                message: "m".into(),
            },
            "integrity-finding",
        ),
        (
            StikkError::Limits {
                message: "m".into(),
            },
            "limits",
        ),
        (StikkError::environment_msg("m"), "environment"),
        (
            StikkError::Internal { detail: "m".into() },
            "stikk-internal",
        ),
    ];
    for (err, name) in cases {
        assert_eq!(err.class(), name);
    }
}

#[test]
fn refusal_message_is_preserved_verbatim_in_display() {
    // NFR-I03 / FR-110: prikk's message must survive unmodified so the refusal overlay can show it.
    let prikk_said =
        "merge refused: heads/topic is not confluent with heads/main from baseline abc";
    let err = StikkError::Refusal {
        message: prikk_said.to_string(),
    };
    assert!(
        err.to_string().contains(prikk_said),
        "the verbatim prikk message must appear in Display output"
    );
}

#[test]
fn environment_error_preserves_its_source_cause() {
    // ER-01: source() is implemented, unlike prikk's own PrikkError (an audit finding applied here).
    let io = std::io::Error::new(std::io::ErrorKind::NotFound, "prikk binary not found");
    let err = StikkError::environment("could not launch prikk", io);
    let source = std::error::Error::source(&err).expect("environment error must expose its cause");
    assert!(source.to_string().contains("prikk binary not found"));
}

#[test]
fn user_resolved_classes_are_never_auto_retried() {
    // NFR-S04: a refusal, lock conflict, or not-ready is the user's to resolve.
    assert!(
        StikkError::Refusal {
            message: "x".into()
        }
        .is_user_resolved()
    );
    assert!(
        StikkError::LockConflict {
            message: "x".into()
        }
        .is_user_resolved()
    );
    assert!(StikkError::NotReady { detail: "x".into() }.is_user_resolved());
    assert!(!StikkError::environment_msg("x").is_user_resolved());
    assert!(!StikkError::Internal { detail: "x".into() }.is_user_resolved());
}
