//! Tests for the Queue view-model (RFC 028 Handoff A §4 and §6): every line, byte-exact.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use stikk_prikk::{
    NullBackend, Queue, QueueReport, QueueTarget, QueueThreshold, QueuedMessage, QueuedOperation,
    QueuedPatch, QueuedPath, ThresholdStatus,
};

use super::*;

const PATCH_A: &str = "493e58168d7759ee72dd98f21f428a6b3f82023e519830a1ad9fa76f684c1a1a";
const PATCH_B: &str = "71c62d0a05e58564356ee39573c0baba7abac83f87553c0b7f00c417089d2478";
const NODE: &str = "751ae57a14ec0a61402a5118108c41dedd8e5d409c716faaa79a584c2edea6e1";

fn view_at(minor: u32, report: QueueReport) -> QueueView {
    let backend = NullBackend::supported()
        .with_version(0, minor, 0)
        .with_queue(report);
    queue_view(&backend, std::path::Path::new("/repo")).expect("reads")
}

fn threshold(status: ThresholdStatus) -> Option<QueueThreshold> {
    Some(QueueThreshold {
        status,
        warn: 800,
        hard_limit: 1000,
    })
}

/// `STATUS_QUEUE_TWO_PATCH_0_42`'s queue, as the reader produces it, with `message` as given.
fn two_patches(message: impl Fn(&str) -> QueuedMessage) -> Queue {
    Queue {
        count: 2,
        target: QueueTarget::Ref("heads/main".to_string()),
        threshold: threshold(ThresholdStatus::None),
        patches: vec![
            QueuedPatch {
                patch_id: PATCH_A.to_string(),
                message: message("add b"),
                operations: vec![QueuedOperation {
                    kind: "create-file".to_string(),
                    paths: vec![QueuedPath::Path("b.txt".to_string())],
                    author_key_id: None,
                }],
            },
            QueuedPatch {
                patch_id: PATCH_B.to_string(),
                message: message("rename a to c"),
                operations: vec![QueuedOperation {
                    kind: "rename-path".to_string(),
                    paths: vec![
                        QueuedPath::Path("a.txt".to_string()),
                        QueuedPath::Path("c.txt".to_string()),
                    ],
                    author_key_id: Some("author".to_string()),
                }],
            },
        ],
    }
}

#[test]
fn at_0_42_each_patch_shows_its_message_and_operations_and_the_foot_names_patch_detail() {
    let view = view_at(
        42,
        QueueReport::Listed(two_patches(|m| QueuedMessage::Text(m.to_string()))),
    );
    assert_eq!(
        view,
        QueueView {
            heading: "2 patch(es) queued for heads/main".to_string(),
            thresholds: vec![
                "warning at 800 · hard limit at 1000".to_string(),
                "below the warning threshold".to_string(),
            ],
            patches: vec![
                QueuedPatchView {
                    id: PATCH_A.to_string(),
                    message: Some("add b".to_string()),
                    operations: vec!["create-file b.txt".to_string()],
                },
                QueuedPatchView {
                    id: PATCH_B.to_string(),
                    message: Some("rename a to c".to_string()),
                    operations: vec!["rename-path a.txt → c.txt · asserted by author".to_string()],
                },
            ],
            foot: Some(
                "A queued patch's content is not shown here; stikk's Patch detail view is not built yet."
                    .to_string()
            ),
        }
    );
}

#[test]
fn at_0_41_there_is_no_message_line_and_the_foot_says_why_once() {
    let view = view_at(
        41,
        QueueReport::Listed(two_patches(|_| QueuedMessage::NotReported)),
    );
    for patch in &view.patches {
        assert_eq!(patch.message, None, "{patch:?}");
    }
    assert_eq!(
        view.foot.as_deref(),
        Some(
            "prikk 0.41 does not report a queued patch's message or content; both appear in History \
             once it is sealed."
        )
    );
}

#[test]
fn a_null_message_reads_no_message_and_a_long_one_shows_its_first_line() {
    let mut queue = two_patches(|_| QueuedMessage::None);
    queue.patches[1].message =
        QueuedMessage::Text("rename a to c\n\nbecause c is clearer".to_string());
    let view = view_at(42, QueueReport::Listed(queue));
    assert_eq!(view.patches[0].message.as_deref(), Some("(no message)"));
    assert_eq!(view.patches[1].message.as_deref(), Some("rename a to c…"));
}

#[test]
fn an_unresolved_node_is_named_as_one() {
    let mut queue = two_patches(|m| QueuedMessage::Text(m.to_string()));
    queue.patches[0].operations = vec![QueuedOperation {
        kind: "edit-text".to_string(),
        paths: vec![QueuedPath::UnresolvedNode(NODE.to_string())],
        author_key_id: None,
    }];
    let view = view_at(42, QueueReport::Listed(queue));
    assert_eq!(
        view.patches[0].operations,
        vec![format!(
            "edit-text unresolved node {NODE} (no longer in the baseline)"
        )]
    );
}

#[test]
fn each_threshold_state_has_its_words() {
    for (status, words) in [
        (ThresholdStatus::None, "below the warning threshold"),
        (ThresholdStatus::Warn, "at or above the warning threshold"),
        (ThresholdStatus::HardLimit, "at or above the hard limit"),
    ] {
        let mut queue = two_patches(|m| QueuedMessage::Text(m.to_string()));
        queue.threshold = threshold(status);
        let view = view_at(42, QueueReport::Listed(queue));
        assert_eq!(
            view.thresholds,
            vec![
                "warning at 800 · hard limit at 1000".to_string(),
                words.to_string()
            ]
        );
    }
}

#[test]
fn each_target_state_has_its_heading() {
    for (target, heading) in [
        (
            QueueTarget::MissingMetadata,
            "2 patch(es) queued; prikk reports the target ref's metadata as missing",
        ),
        (
            QueueTarget::MalformedMetadata,
            "2 patch(es) queued; prikk reports the target ref's metadata as malformed",
        ),
        (
            QueueTarget::NotReported,
            "2 patch(es) queued; prikk reports no target ref",
        ),
    ] {
        let mut queue = two_patches(|m| QueuedMessage::Text(m.to_string()));
        queue.target = target;
        assert_eq!(view_at(42, QueueReport::Listed(queue)).heading, heading);
    }
}

#[test]
fn an_empty_queue_says_nothing_is_queued_and_nothing_else() {
    let nothing = QueueView {
        heading: "Nothing is queued.".to_string(),
        thresholds: Vec::new(),
        patches: Vec::new(),
        foot: None,
    };
    let listed = QueueReport::Listed(Queue {
        count: 0,
        target: QueueTarget::NotReported,
        threshold: None,
        patches: Vec::new(),
    });
    assert_eq!(view_at(42, listed), nothing);
    assert_eq!(
        view_at(
            28,
            QueueReport::Unreported {
                count: 0,
                target: None
            }
        ),
        nothing
    );
}

#[test]
fn below_0_39_the_list_is_unreported_never_empty() {
    let view = view_at(
        28,
        QueueReport::Unreported {
            count: 2,
            target: Some("heads/main".to_string()),
        },
    );
    assert_eq!(
        view,
        QueueView {
            heading: "2 patch(es) queued for heads/main".to_string(),
            thresholds: Vec::new(),
            patches: Vec::new(),
            foot: Some("prikk 0.28 does not list queued patches.".to_string()),
        }
    );
    let no_target = view_at(
        38,
        QueueReport::Unreported {
            count: 1,
            target: None,
        },
    );
    assert_eq!(
        no_target.heading,
        "1 patch(es) queued; prikk reports no target ref"
    );
    assert_eq!(
        no_target.foot.as_deref(),
        Some("prikk 0.38 does not list queued patches.")
    );
}
