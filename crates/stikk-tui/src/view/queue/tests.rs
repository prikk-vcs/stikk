//! Render tests for the Queue view (RFC 028 Handoff A §6): the six captures at 80 columns.
//!
//! Each view is built by `stikk_core::queue_view` from a `QueueReport` holding exactly what the reader
//! produces from the matching capture in `stikk-prikk`'s `parse_json/tests.rs` (same ids, paths, messages
//! and numbers). Every capture asserts that each of the view's lines is on screen **whole**, wrapped or not
//! — nothing clipped (RFC 024).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_core::QueueView;
use stikk_prikk::{
    NullBackend, Queue, QueueReport, QueueTarget, QueueThreshold, QueuedMessage, QueuedOperation,
    QueuedPatch, QueuedPath, ThresholdStatus,
};

use super::*;
use crate::test_util::buffer_text;

fn view_at(minor: u32, report: QueueReport) -> QueueView {
    let backend = NullBackend::supported()
        .with_version(0, minor, 0)
        .with_queue(report);
    stikk_core::queue_view(&backend, std::path::Path::new("/repo")).expect("reads")
}

fn draw(view: &QueueView) -> String {
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|f| {
            render(
                view,
                &std::cell::Cell::new(0),
                0,
                &Palette::default(),
                f,
                f.area(),
            )
        })
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

/// The screen's text with the borders and all runs of whitespace collapsed, so a wrapped line matches whole.
fn joined(text: &str) -> String {
    text.lines()
        .map(|line| line.trim_matches(|c: char| c == '│' || c.is_whitespace()))
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Print the capture, and assert every line of `view` is on screen whole.
fn capture(label: &str, view: &QueueView) -> String {
    let text = draw(view);
    println!("--- Queue, {label}, 80×24\n{text}");
    let screen = joined(&text);
    let mut expected = vec![view.heading.clone()];
    expected.extend(view.thresholds.iter().cloned());
    for patch in &view.patches {
        expected.push(patch.id.clone());
        expected.extend(patch.message.iter().cloned());
        expected.extend(patch.operations.iter().cloned());
    }
    expected.extend(view.foot.iter().cloned());
    for line in expected {
        let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            screen.contains(&line),
            "{label}: {line:?} is not whole on screen:\n{text}"
        );
    }
    assert!(text.contains("Queue"), "{label}: the title");
    text
}

fn op(kind: &str, paths: Vec<QueuedPath>, key: Option<&str>) -> QueuedOperation {
    QueuedOperation {
        kind: kind.to_string(),
        paths,
        author_key_id: key.map(str::to_string),
    }
}

fn path(p: &str) -> QueuedPath {
    QueuedPath::Path(p.to_string())
}

fn threshold(status: ThresholdStatus, warn: u64) -> Option<QueueThreshold> {
    Some(QueueThreshold {
        status,
        warn,
        hard_limit: 1000,
    })
}

/// `STATUS_QUEUE_TWO_PATCH_0_42` / `_0_41`, as the reader produces them.
fn two_patch(ids: [&str; 2], message: impl Fn(&str) -> QueuedMessage) -> QueueReport {
    QueueReport::Listed(Queue {
        count: 2,
        target: QueueTarget::Ref("heads/main".to_string()),
        threshold: threshold(ThresholdStatus::None, 800),
        patches: vec![
            QueuedPatch {
                patch_id: ids[0].to_string(),
                message: message("add b"),
                operations: vec![op("create-file", vec![path("b.txt")], None)],
            },
            QueuedPatch {
                patch_id: ids[1].to_string(),
                message: message("rename a to c"),
                operations: vec![op(
                    "rename-path",
                    vec![path("a.txt"), path("c.txt")],
                    Some("author"),
                )],
            },
        ],
    })
}

#[test]
fn the_six_queue_captures_render_whole_at_80_columns() {
    // 1. 0.42, two patches, one a rename.
    let text = capture(
        "0.42 two patches with a rename",
        &view_at(
            42,
            two_patch(
                [
                    "493e58168d7759ee72dd98f21f428a6b3f82023e519830a1ad9fa76f684c1a1a",
                    "71c62d0a05e58564356ee39573c0baba7abac83f87553c0b7f00c417089d2478",
                ],
                |m| QueuedMessage::Text(m.to_string()),
            ),
        ),
    );
    assert!(
        text.contains("rename-path a.txt → c.txt · asserted by author"),
        "{text}"
    );

    // 2. 0.41, the same queue without messages.
    let text = capture(
        "0.41 without messages",
        &view_at(
            41,
            two_patch(
                [
                    "49d234ae86c48f9126a84acc26d847836b361d8a923081b16188bcc16293e15f",
                    "80ed9246a501996bfc41b1d9ff0d3f6a7e36d636e47e3d8be08a7c6e73e0ae14",
                ],
                |_| QueuedMessage::NotReported,
            ),
        ),
    );
    assert!(
        !text.contains("add b") && !text.contains("(no message)"),
        "{text}"
    );

    // 3. 0.42, an unresolved node beside a path.
    capture(
        "0.42 unresolved node",
        &view_at(
            42,
            QueueReport::Listed(Queue {
                count: 2,
                target: QueueTarget::Ref("heads/main".to_string()),
                threshold: threshold(ThresholdStatus::None, 800),
                patches: vec![
                    QueuedPatch {
                        patch_id:
                            "af0fb3f2d7c2b509c7383a23a1c387a2134fdad26f825f3ad446c0c5a7c59029"
                                .to_string(),
                        message: QueuedMessage::Text("edit doc".to_string()),
                        operations: vec![op(
                            "edit-text",
                            vec![QueuedPath::UnresolvedNode(
                                "751ae57a14ec0a61402a5118108c41dedd8e5d409c716faaa79a584c2edea6e1"
                                    .to_string(),
                            )],
                            None,
                        )],
                    },
                    QueuedPatch {
                        patch_id:
                            "1cab8c21fe1639c54e2f8521ecbfcd53f057bfce9567e0a3991b4a40f222048e"
                                .to_string(),
                        message: QueuedMessage::Text("delete doc".to_string()),
                        operations: vec![op("delete-node", vec![path("doc.txt")], None)],
                    },
                ],
            }),
        ),
    );

    // 4. 0.42, a warn threshold.
    let text = capture(
        "0.42 warn threshold",
        &view_at(
            42,
            QueueReport::Listed(Queue {
                count: 1,
                target: QueueTarget::Ref("heads/main".to_string()),
                threshold: threshold(ThresholdStatus::Warn, 1),
                patches: vec![QueuedPatch {
                    patch_id: "3829ccfb080c07011497119384f429bbcf32395695bfc6030dc3fe20da56f321"
                        .to_string(),
                    message: QueuedMessage::Text("one patch".to_string()),
                    operations: vec![op("create-file", vec![path("w.txt")], None)],
                }],
            }),
        ),
    );
    assert!(text.contains("at or above the warning threshold"), "{text}");

    // 5. Below 0.39.
    let text = capture(
        "below 0.39",
        &view_at(
            28,
            QueueReport::Unreported {
                count: 2,
                target: Some("heads/main".to_string()),
            },
        ),
    );
    assert!(
        text.contains("prikk 0.28 does not list queued patches."),
        "{text}"
    );

    // 6. Empty.
    let text = capture(
        "empty",
        &view_at(
            42,
            QueueReport::Listed(Queue {
                count: 0,
                target: QueueTarget::NotReported,
                threshold: None,
                patches: Vec::new(),
            }),
        ),
    );
    assert!(text.contains("Nothing is queued."), "{text}");
    assert!(
        !text.contains("patch"),
        "nothing else below the heading: {text}"
    );
}

#[test]
fn a_hostile_message_is_rendered_inert() {
    let view = QueueView {
        heading: "1 patch(es) queued for heads/main".to_string(),
        thresholds: Vec::new(),
        patches: vec![stikk_core::QueuedPatchView {
            id: "3829ccfb080c07011497119384f429bbcf32395695bfc6030dc3fe20da56f321".to_string(),
            message: Some("evil\u{1b}[2Jmessage".to_string()),
            detail_message: stikk_core::PatchMessageState::Text("evil\u{1b}[2Jmessage".to_string()),
            operations: Vec::new(),
        }],
        foot: None,
    };
    let text = draw(&view);
    assert!(!text.contains('\u{1b}'), "{text:?}");
}

// ---------------------------------------------------------------------------------------------
// Review v1 §2.1: a tall queue scrolls, in the Glossary's idiom.
// ---------------------------------------------------------------------------------------------

/// Twelve one-operation patches at 0.42: 3 heading rows, 4 rows per patch, and a blank row and a two-row
/// foot — 54 rows against a 22-row viewport at 80×24.
fn tall_queue() -> QueueView {
    let patches = (1..=12)
        .map(|i| QueuedPatch {
            patch_id: format!("{i:064x}"),
            message: QueuedMessage::Text(format!("patch {i}")),
            operations: vec![op("create-file", vec![path(&format!("p{i}.txt"))], None)],
        })
        .collect();
    view_at(
        42,
        QueueReport::Listed(Queue {
            count: 12,
            target: QueueTarget::Ref("heads/main".to_string()),
            threshold: threshold(ThresholdStatus::None, 800),
            patches,
        }),
    )
}

fn draw_scrolled(view: &QueueView, offset: &std::cell::Cell<u16>) -> String {
    draw_at(view, offset, 0)
}

/// The same, with `cursor` selecting a patch — which is what moves the scroll since RFC 035 §4.2.
fn draw_at(view: &QueueView, offset: &std::cell::Cell<u16>, cursor: usize) -> String {
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|f| render(view, offset, cursor, &Palette::default(), f, f.area()))
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

#[test]
fn a_tall_queue_scrolls_to_follow_the_selected_patch_and_says_where_it_is() {
    let view = tall_queue();
    let offset = std::cell::Cell::new(0);

    let top = draw_at(&view, &offset, 0);
    println!("--- Queue, twelve patches, first selected, 80×24\n{top}");
    assert!(
        top.contains(" Queue — ↑/↓ to scroll · lines 1–22 of 53 "),
        "{top}"
    );
    assert!(top.contains("12 patch(es) queued for heads/main"), "{top}");
    assert!(
        top.contains("▸ 00000000000"),
        "the first patch is marked: {top}"
    );
    assert_eq!(offset.get(), 0, "nothing to scroll to yet");

    // RFC 035 §4.2: the scroll follows the **cursor**, not a key of its own. Selecting the last patch
    // brings its rows on screen, and no further.
    let end = draw_at(&view, &offset, 11);
    println!("--- Queue, twelve patches, last selected, 80×24\n{end}");
    assert!(end.contains("patch 12"), "{end}");
    assert!(end.contains("create-file p12.txt"), "{end}");
    assert!(
        !end.contains("12 patch(es) queued"),
        "the heading scrolled away: {end}"
    );
    // The foot is *not* asserted here: the scroll stops as soon as the selected patch's last row is
    // on screen, which is the whole point of following the cursor by the least it can — the rows below
    // it stay below it.
    // And the offset the renderer wrote back is the least that shows it, not the end of the list.
    let followed = offset.get();
    assert!(followed > 0 && followed <= 53 - 22, "{followed}");

    // Selecting back upwards scrolls back up by the least it can, too.
    let back = draw_at(&view, &offset, 0);
    assert_eq!(offset.get(), 0, "{back}");
    assert!(
        back.contains("12 patch(es) queued for heads/main"),
        "{back}"
    );
}

#[test]
fn a_refresh_that_shrinks_the_queue_clamps_the_offset() {
    let offset = std::cell::Cell::new(0);
    offset.set(u16::MAX);
    // The last patch selected, so the clamp is tested against a scroll the cursor justifies.
    draw_at(&tall_queue(), &offset, 11);
    assert_eq!(offset.get(), 53 - 22);

    // The refreshed queue fits: the kept offset clamps to zero, and the title is plain.
    let small = view_at(
        28,
        QueueReport::Unreported {
            count: 1,
            target: Some("heads/main".to_string()),
        },
    );
    let text = draw_scrolled(&small, &offset);
    assert_eq!(offset.get(), 0, "{text}");
    assert!(text.contains("1 patch(es) queued for heads/main"), "{text}");
}

#[test]
fn a_queue_that_fits_shows_the_plain_title_and_does_not_scroll() {
    let view = view_at(
        42,
        two_patch(
            [
                "493e58168d7759ee72dd98f21f428a6b3f82023e519830a1ad9fa76f684c1a1a",
                "71c62d0a05e58564356ee39573c0baba7abac83f87553c0b7f00c417089d2478",
            ],
            |m| QueuedMessage::Text(m.to_string()),
        ),
    );
    let offset = std::cell::Cell::new(5);
    let text = draw_scrolled(&view, &offset);
    assert_eq!(offset.get(), 0, "nothing to scroll to");
    assert!(text.contains("┌ Queue ─"), "{text}");
    assert!(!text.contains("to scroll"), "{text}");
    assert!(text.contains("2 patch(es) queued for heads/main"), "{text}");
}

/// **RFC 035 §6's capture: the Queue with its selection**, at 80 columns. The marker is on exactly one
/// patch, and the foot says what `Enter` does.
#[test]
fn rfc035_the_queue_marks_the_selected_patch_at_80_columns() {
    let view = view_at(
        42,
        two_patch(
            [
                "493e58168d7759ee72dd98f21f428a6b3f82023e519830a1ad9fa76f684c1a1a",
                "71c62d0a05e58564356ee39573c0baba7abac83f87553c0b7f00c417089d2478",
            ],
            |m| QueuedMessage::Text(m.to_string()),
        ),
    );
    let text = draw_at(&view, &std::cell::Cell::new(0), 1);
    println!("--- RFC 035 §6: the Queue with its selection, 80 columns\n{text}");
    let marked: Vec<&str> = text.lines().filter(|line| line.contains('▸')).collect();
    assert_eq!(marked.len(), 1, "one patch marked: {text}");
    assert!(
        marked[0].contains("71c62d0a05e58564356ee39573c0baba7abac83f87553c0b7f00c417089d2478"),
        "the second patch: {text}"
    );
    assert!(
        joined(&text).contains("Enter opens a patch's operations."),
        "{text}"
    );
    // And the patch that is not selected is still listed, unmarked.
    assert!(
        text.contains("  493e58168d7759ee72dd98f21f428a6b3f82023e519830a1ad9fa76f684c1a1a"),
        "{text}"
    );
}
