//! **Every frame stikk draws, pinned** (RFC 067 decision 8; Handoff A §1).
//!
//! RFC 067 moves stikk's program out of the terminal front end. The claim that comes with it is that
//! **a user sees no change**, and a claim about what a user sees can only be proved by what is drawn.
//! So before anything moves, this module records every view and every overlay, at four sizes, in
//! every palette, and again after the text has been read to its end — and the recording is committed
//! (`golden/frames.txt`). Every later commit must leave it **byte for byte** as it is.
//!
//! # What a frame is
//!
//! A scene drives the **real `App`** and the **real [`crate::shell::render`]** into a `TestBackend`
//! buffer, as [`crate::readme_visuals`] does, from invented data and nothing else. The frame is then
//! every cell's **symbol and style** — foreground, background and modifiers — row by row.
//! [`crate::test_util::buffer_text`] would not do: it drops the style, and four of the five palettes
//! differ from each other in style alone.
//!
//! # Coverage is the compiler's to check
//!
//! [`scene_for_overlay`] and [`scene_for_focus`] map **every** variant of [`Overlay`] and [`Focus`]
//! to the scene that shows it, with no `_` arm, and they return the scene's own builder. A variant
//! added later therefore does not compile until someone writes a scene for it, and an arm cannot name
//! a scene that does not exist. Each scene then asserts that it reaches the variant it claims
//! ([`every_scene_reaches_what_it_claims`]).
//!
//! # When a frame differs
//!
//! [`every_frame_is_what_it_was`] names every frame that differs and writes each one's full text to
//! `target/golden-frames/<scene>-<size>-<palette>-<state>.txt`, so the difference can be read rather
//! than guessed at.
//!
//! **The manifest is written once.** `STIKK_GOLDEN_WRITE=1 cargo test -p stikk-tui golden` rewrites
//! it; that was done in RFC 067 Handoff A's first commit and must not be done to make a later commit
//! pass. If a frame changes, the change is the defect (Handoff A trap 1).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent};

use stikk_core::{ConfirmationSummary, NextStep, NextTarget, OperationContext, RefusalCard};
use stikk_model::{Capability, Readiness, Tier};

use crate::app::{App, Focus, Operation, OperationStatus, OrientationState};
use crate::overlay::{Overlay, RefNameKind, RefWriteKind, SavePathKind};
use crate::theme::{Depth, Palette, Scheme};
use crate::worker::{Request, Response, ResponseKind};

/// Where the committed manifest lives, relative to this crate.
const MANIFEST: &str = "src/golden/frames.txt";

/// Where a differing frame's full text is written, relative to the workspace target directory.
const DIFF_DIR: &str = "../../target/golden-frames";

// The invented world. **Nothing here is copied from any real repository** (the same rule
// `readme_visuals` keeps, for the same reason).
const REPO: &str = "example-project";
const MAIN: &str = "heads/main";
const OTHER: &str = "heads/feature";

/// An invented object id: visibly a placeholder, so no reader mistakes it for a real one.
fn id(tag: char) -> String {
    format!("{}0badc0de", tag.to_string().repeat(56))
}

// -------------------------------------------------------------------------------------------
// The digest.

/// **64-bit FNV-1a, written out** (Handoff A §1.5). `DefaultHasher` is not promised to be stable
/// across Rust releases, and a hash crate is more than a regression check needs — so the ten lines
/// are here, where they cannot change under the manifest.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

// -------------------------------------------------------------------------------------------
// The frame.

/// Every cell's symbol **and** style, row by row — what a user sees, and nothing about the machine.
fn serialize(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut out = String::new();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = buffer.cell((x, y)).expect("a cell inside the area");
            out.push_str(cell.symbol());
            out.push('\u{1f}');
            out.push_str(&format!(
                "{:?}|{:?}|{:?}|{:?}",
                cell.fg, cell.bg, cell.modifier, cell.underline_color
            ));
            out.push('\u{1e}');
        }
        out.push('\n');
    }
    out
}

/// Draw one `App` through the same `shell::render` the run loop calls, at `width` x `height`.
fn draw(app: &App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
    terminal
        .draw(|frame| crate::shell::render(app, frame))
        .expect("a draw");
    terminal.backend().buffer().clone()
}

/// **One `↓`, through the key path** — never a handler call (RFC 067 decision 3's rule, and RFC 060
/// Handoff B's lesson: only a test that presses a key proves the wiring).
fn press_down(app: &mut App) {
    app.handle_key(KeyEvent::from(KeyCode::Down));
}

/// Draw, then press `↓`, until the frame stops changing — the path `Seen` depends on (§1.2).
///
/// **The stop condition is the frame itself**, not any internal field: a card scrolls to its end and
/// then `↓` moves into its actions, a view scrolls to its end and then `↓` clamps, and either way the
/// frame settles. The bound is a safety net, not the expected exit.
fn scrolled_to_rest(app: &mut App, width: u16, height: u16) -> Buffer {
    let mut frame = draw(app, width, height);
    for _ in 0..64 {
        press_down(app);
        let next = draw(app, width, height);
        if next == frame {
            return frame;
        }
        frame = next;
    }
    frame
}

// -------------------------------------------------------------------------------------------
// The palettes.

/// The palette every frame is drawn in unless the matrix asks for another.
fn dark() -> Palette {
    Scheme::dark().for_depth(Depth::TrueColour)
}

/// The five palettes a frame can be drawn in, named as the manifest names them.
fn palettes() -> Vec<(&'static str, Palette)> {
    vec![
        ("dark", Scheme::dark().for_depth(Depth::TrueColour)),
        ("light", Scheme::light().for_depth(Depth::TrueColour)),
        (
            "high-contrast-dark",
            Scheme::high_contrast(false).for_depth(Depth::TrueColour),
        ),
        (
            "high-contrast-light",
            Scheme::high_contrast(true).for_depth(Depth::TrueColour),
        ),
        ("plain", Scheme::plain().for_depth(Depth::TrueColour)),
    ]
}

/// The four sizes, smallest first: the floor RFC 060 set, the project's own capture size, and two
/// larger terminals.
const SIZES: [(u16, u16); 4] = [(60, 15), (80, 24), (120, 40), (200, 60)];

// -------------------------------------------------------------------------------------------
// The invented repository.

fn orientation() -> stikk_core::OrientationView {
    let readiness = Readiness::none();
    stikk_core::OrientationView {
        interrupted_materialization: None,
        prikk_version: "prikk 0.58.0".to_string(),
        prikk_supported: true,
        prikk_validated: true,
        validated_through: "0.58.0".to_string(),
        prikk_persists_messages: true,
        prikk_minor: 58,
        queued_patches: 2,
        queued_target: Some(MAIN.to_string()),
        trailing_partial_wal_bytes: 0,
        main_ref_state: Some(id('4')),
        capability: Capability::derive(readiness),
        readiness,
        stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
        current_branch: stikk_model::CurrentBranch::NotReported,
    }
}

fn block_row(tag: char, state: char, seq: u64, kind: &str, patches: u64) -> stikk_prikk::BlockRow {
    stikk_prikk::BlockRow {
        block_id: id(tag),
        ref_state_id: id(state),
        update_seq: seq,
        kind: kind.to_string(),
        rollback_block: false,
        parents: u64::from(seq > 1),
        patches,
        rollback_patches: 0,
        required_attestations: 0,
        messages: vec![stikk_prikk::PatchMessage {
            patch_id: id(tag),
            message: format!("a change, number {seq}"),
        }],
        previous_ref_state: (seq > 1).then(|| id('9')),
    }
}

/// A lineage long enough to overflow 15 rows, so the scrolled frames have somewhere to go.
fn history() -> stikk_core::HistoryView {
    let rows = [
        ('1', 'd', 14, "Merge", 4),
        ('2', 'e', 13, "Normal", 2),
        ('3', 'f', 12, "Normal", 7),
        ('4', '1', 11, "Normal", 1),
        ('5', '2', 10, "Normal", 3),
        ('6', '3', 9, "Normal", 1),
        ('7', '4', 8, "Merge", 6),
        ('8', '5', 7, "Normal", 2),
        ('9', '6', 6, "Normal", 5),
        ('a', '7', 5, "Normal", 1),
        ('b', '8', 4, "Normal", 3),
        ('c', '9', 3, "Normal", 2),
        ('d', 'a', 2, "Normal", 1),
        ('e', 'b', 1, "Root", 1),
    ];
    stikk_core::HistoryView {
        reff: MAIN.to_string(),
        queued: 2,
        queued_target: Some(MAIN.to_string()),
        blocks: rows
            .iter()
            .map(|(tag, state, seq, kind, patches)| block_row(*tag, *state, *seq, kind, *patches))
            .collect(),
    }
}

fn block_detail() -> stikk_core::BlockDetailView {
    stikk_core::BlockDetailView {
        row: block_row('1', 'd', 14, "Merge", 4),
        is_tip: true,
        state: stikk_core::BlockState::Unreported(46),
        patches: stikk_core::BlockPatches::Unreported(39),
    }
}

fn change_entry(path: &str, kind: stikk_core::ChangeKind) -> stikk_core::ChangeEntry {
    stikk_core::ChangeEntry {
        kind,
        path: path.to_string(),
        note: String::new(),
        authoring: stikk_prikk::Authoring::Authored,
        rename: None,
    }
}

fn changes() -> stikk_core::ChangesView {
    use stikk_core::ChangeKind;
    stikk_core::ChangesView {
        reff: MAIN.to_string(),
        clean: false,
        tracked: 9,
        unchanged: 5,
        missing: 1,
        modified: 2,
        untracked: 1,
        unsupported: 0,
        refused: None,
        refused_declarations: None,
        entries: vec![
            change_entry("src/main.rs", ChangeKind::Modified),
            change_entry("src/lib.rs", ChangeKind::Modified),
            change_entry("docs/old.md", ChangeKind::Missing),
            change_entry("notes.txt", ChangeKind::Untracked),
        ],
        queued_elsewhere: None,
        declarations: Vec::new(),
        declared_renames: Vec::new(),
        renames: 0,
    }
}

fn queue() -> stikk_core::QueueView {
    stikk_core::QueueView {
        heading: format!("2 patches queued for {MAIN}"),
        thresholds: vec!["seal at 64 patches".to_string()],
        patches: (1..=2)
            .map(|n| stikk_core::QueuedPatchView {
                id: id(char::from_digit(n, 16).unwrap()),
                message: Some(format!("a queued change, number {n}")),
                detail_message: stikk_core::PatchMessageState::Text(format!("message {n}")),
                operations: vec![format!("modified src/file{n}.rs")],
            })
            .collect(),
        foot: Some("Enter opens a queued patch.".to_string()),
        head: None,
    }
}

fn patch_detail() -> stikk_core::PatchDetailView {
    stikk_core::PatchDetailView {
        patch_id: id('2'),
        heading: vec!["modified one file".to_string()],
        operations: (1..=6)
            .map(|n| stikk_core::OperationView {
                title: vec![
                    stikk_core::Piece::Said("text edit in ".to_string()),
                    stikk_core::Piece::Name(format!("src/file{n}.rs")),
                ],
                rows: vec![
                    stikk_core::OperationRow::Said(format!("span {n}, 12 bytes replaced")),
                    stikk_core::OperationRow::Content(format!("\"    let n = {n};\\n\"")),
                ],
                raw: vec![format!("{{\"op\":\"edit\",\"n\":{n}}}")],
            })
            .collect(),
        unreported: false,
        foot: Some("f shows prikk's own fields.".to_string()),
    }
}

fn trust() -> stikk_core::TrustView {
    stikk_core::TrustView::Listed {
        rows: (1..=3)
            .map(|n| stikk_core::TrustRow {
                key_id: format!("maintainer-{n}"),
                public_key: id(char::from_digit(n, 16).unwrap()),
                yours: n == 1,
            })
            .collect(),
        own_key_unknown: None,
    }
}

fn merge_plan() -> stikk_core::MergePlanView {
    stikk_core::MergePlanView {
        from: OTHER.to_string(),
        into: MAIN.to_string(),
        baseline: id('b'),
        lines: (1..=12)
            .map(|n| format!("prikk says: pair {n} is confluent"))
            .collect(),
        outcome: stikk_prikk::MergeOutcome::Confluent,
    }
}

fn verify() -> stikk_core::VerifyView {
    stikk_core::VerifyView::Checked(stikk_core::Checked {
        ok: true,
        conditions: Vec::new(),
        stages: [
            "objects",
            "refs",
            "wal-replay",
            "wal-persistence",
            "commit-witness",
            "commit-index",
            "wal-ordering",
            "received-refs",
            "object-connectivity",
        ]
        .into_iter()
        .map(|stage| stikk_core::StageRow {
            stage: stage.to_string(),
            status: "ok".to_string(),
            detail: None,
        })
        .collect(),
        sessions: Some(stikk_prikk::ActiveSessions {
            count: 1,
            verified_count: 1,
        }),
        raw: vec!["{\"verdict\":{\"ok\":true}}".to_string()],
    })
}

fn compare() -> stikk_core::CompareView {
    let side = |tag: char| stikk_core::ComparedSide {
        point: id(tag),
        target: stikk_prikk::DiffTarget::Block(id(tag)),
        queued: stikk_prikk::QueuedOnSide::NotApplicable,
    };
    stikk_core::CompareView::Listed(stikk_core::Compared {
        from: side('2'),
        to: side('1'),
        entries: (1..=3)
            .map(|n| stikk_core::ComparedEntry {
                status: "modified".to_string(),
                title: vec![stikk_core::Piece::Name(format!("src/file{n}.rs"))],
                facts: vec![vec![stikk_core::Piece::Said(
                    "text, 96 -> 98 bytes".to_string(),
                )]],
                minimal_note: None,
                rows: vec![
                    stikk_core::HunkRow::Header(format!("@@ -{n} +{n} @@")),
                    stikk_core::HunkRow::Line {
                        marker: '-',
                        content: "    println!(\"hello\");".to_string(),
                    },
                    stikk_core::HunkRow::Line {
                        marker: '+',
                        content: "    println!(\"hello\");  ".to_string(),
                    },
                ],
                raw: vec![format!("{{\"path\":\"src/file{n}.rs\"}}")],
            })
            .collect(),
        unsupported: Vec::new(),
    })
}

fn bundle() -> stikk_core::BundleOpened {
    stikk_core::BundleOpened::Refused {
        path: "shared/feature.bundle".to_string(),
        message: "malformed persisted data: unexpected end of bundle".to_string(),
    }
}

// -------------------------------------------------------------------------------------------
// The scenes.

/// A staged `App`, with the request channel kept alive so nothing it sends is lost.
struct Staged {
    app: App,
    _rx: mpsc::Receiver<Request>,
}

/// How a scene is built. **The scene maps return one of these**, so an arm cannot name a scene that
/// does not exist (§1.3).
type Build = fn(Palette) -> Staged;

/// An `App` focused on the invented repository, with nothing loaded yet.
fn staged(palette: Palette) -> (App, mpsc::Receiver<Request>) {
    let (tx, rx) = mpsc::channel();
    let app = App::from_state(
        PathBuf::from(REPO),
        OrientationState::Loaded(orientation()),
        palette,
        tx,
    );
    (app, rx)
}

/// Apply one response to the request the app has just sent.
///
/// **[`mpsc::Receiver::try_recv`], never `recv`**: the app queues its request before returning, so
/// the request is already there. A blocking read would buy nothing and would turn a scene that asks
/// for one response too many into a test that hangs for ever instead of one that fails and says so.
fn answer(app: &mut App, rx: &mpsc::Receiver<Request>, kind: impl FnOnce(u64) -> ResponseKind) {
    let seq = rx
        .try_recv()
        .expect("the app to have sent a request to answer")
        .seq;
    let kind = kind(seq);
    app.apply(Response { seq, kind });
}

fn with(palette: Palette, build: impl FnOnce(&mut App, &mpsc::Receiver<Request>)) -> Staged {
    let (mut app, rx) = staged(palette);
    build(&mut app, &rx);
    Staged { app, _rx: rx }
}

// The twelve `Focus` variants.

fn scene_orientation(palette: Palette) -> Staged {
    with(palette, |_app, _rx| {})
}

fn scene_loading(palette: Palette) -> Staged {
    // A pending screen, with no answer applied: `Focus::Loading`.
    with(palette, |app, _rx| app.open_history())
}

fn scene_history(palette: Palette) -> Staged {
    with(palette, |app, rx| {
        app.open_history();
        answer(app, rx, |_| ResponseKind::History(Ok(history())));
    })
}

fn scene_block_detail(palette: Palette) -> Staged {
    with(palette, |app, rx| {
        app.open_history();
        answer(app, rx, |_| ResponseKind::History(Ok(history())));
        // `→`, not Enter: Enter opens the row's action list (RFC 061 decision 3), and it is
        // the drill-in that reads the block.
        app.select_screen_or_row();
        answer(app, rx, |_| ResponseKind::BlockState {
            reff: MAIN.to_string(),
            result: Ok(block_detail()),
        });
    })
}

fn scene_changes(palette: Palette) -> Staged {
    with(palette, |app, rx| {
        app.open_changes();
        answer(app, rx, |_| {
            ResponseKind::Changes(Ok(stikk_core::ChangesRead {
                view: changes(),
                history: stikk_core::RefHistory::Published,
            }))
        });
    })
}

fn scene_queue(palette: Palette) -> Staged {
    with(palette, |app, rx| {
        app.open_queue();
        answer(app, rx, |_| ResponseKind::Queue(Ok(queue())));
    })
}

fn scene_patch_detail(palette: Palette) -> Staged {
    with(palette, |app, rx| {
        app.open_queue();
        answer(app, rx, |_| ResponseKind::Queue(Ok(queue())));
        app.select_screen_or_row();
        answer(app, rx, |_| ResponseKind::PatchDetail(Ok(patch_detail())));
    })
}

fn scene_compare(palette: Palette) -> Staged {
    with(palette, |app, rx| {
        app.open_history();
        answer(app, rx, |_| ResponseKind::History(Ok(history())));
        app.compare();
        answer(app, rx, |_| ResponseKind::Compare {
            opened: crate::app::CompareSource::PreviousInHistory,
            result: Ok(compare()),
        });
    })
}

fn scene_merge_plan(palette: Palette) -> Staged {
    with(palette, |app, rx| {
        app.open_ref_picker();
        answer(app, rx, |_| {
            ResponseKind::Refs(Ok(stikk_core::RefList {
                refs: [(MAIN, '4'), (OTHER, '7')]
                    .into_iter()
                    .map(|(name, tag)| stikk_prikk::RefEntry {
                        name: name.to_string(),
                        id: id(tag),
                        closed: false,
                        received: false,
                    })
                    .collect(),
                unpublished_current: stikk_prikk::UnpublishedBranch::None,
            }))
        });
        app.mark_block();
        app.nav_down();
        app.begin_merge_plan();
        answer(app, rx, |_| {
            ResponseKind::MergePlan(Ok(stikk_core::MergePlanRead::Read(Box::new(merge_plan()))))
        });
    })
}

fn scene_bundle(palette: Palette) -> Staged {
    with(palette, |app, rx| {
        app.begin_open_bundle();
        for ch in "shared/feature.bundle".chars() {
            app.input_char(ch);
        }
        app.select();
        answer(app, rx, |_| ResponseKind::OpenBundle(Ok(bundle())));
    })
}

fn scene_verify(palette: Palette) -> Staged {
    with(palette, |app, rx| {
        app.open_verify();
        answer(app, rx, |_| ResponseKind::Verify(Ok(verify())));
    })
}

fn scene_trust(palette: Palette) -> Staged {
    with(palette, |app, rx| {
        app.open_trust();
        answer(app, rx, |_| ResponseKind::TrustRead(Ok(trust())));
    })
}

// The thirty-four `Overlay` variants. Each is pushed onto a loaded History, so the frame behind the
// card is the same one in every scene and only the card differs.

fn over(palette: Palette, overlay: Overlay) -> Staged {
    with(palette, |app, rx| {
        app.open_history();
        answer(app, rx, |_| ResponseKind::History(Ok(history())));
        app.push_overlay(overlay);
    })
}

/// A card long enough to need scrolling at 60 x 15: the state `Seen` is made from.
fn long_consequence() -> String {
    (1..=14)
        .map(|n| format!("Consequence line {n}: what this will do, said once and in full."))
        .collect::<Vec<_>>()
        .join(" ")
}

fn summary() -> ConfirmationSummary {
    ConfirmationSummary {
        operation: "Commit worktree changes".to_string(),
        preamble: Some("What a commit will author:".to_string()),
        target_ids: vec![stikk_core::TargetId::bare(id('4'))],
        counts: vec![("files", 9), ("renames", 0)],
        capability: Some(Capability::Author),
        consequence: long_consequence(),
        target_name: Some(MAIN.to_string()),
        signing_key_id: Some("author-1".to_string()),
        signing_key_claim: stikk_core::KeyClaim::None,
        signing_key_is_published_example: false,
        branch_notice: None,
        freezes: None,
        history_notice: None,
        rename_note: None,
        declaration_notices: Vec::new(),
        notices: Vec::new(),
        message_note: None,
        materializes: None,
        undo_plan: None,
        merge_plan: None,
        asides: Vec::new(),
    }
}

fn refusal_card() -> RefusalCard {
    RefusalCard {
        verbatim: "lock conflict: another prikk is writing this repository".to_string(),
        gloss: Some(
            "prikk takes a lock while it writes. Another prikk holds it now, so this read was \
             refused rather than taken half-written."
                .to_string(),
        ),
        next_steps: vec![NextStep {
            label: "Refresh, once the other prikk has finished".to_string(),
            target: NextTarget::Refresh,
        }],
        glossary_codes: vec!["lock".to_string()],
    }
}

fn warning() -> stikk_core::SecurityWarning {
    stikk_core::two_keys_warning(&stikk_core::TwoKeysOneId {
        key_id: "maintainer-1".to_string(),
        adopted: id('1'),
        given: id('2'),
    })
}

fn commit_result() -> stikk_prikk::CommitResult {
    stikk_prikk::CommitResult {
        baseline_ref: MAIN.to_string(),
        patch_id: id('2'),
        wal_sequence: 3,
        operations: 2,
        referenced_blobs: 1,
        text_edits: 1,
        changes: Vec::new(),
        notes: vec!["sealing is the next step".to_string()],
    }
}

fn scene_glossary(palette: Palette) -> Staged {
    over(palette, Overlay::Glossary)
}
fn scene_loading_overlay(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::Loading {
            what: "the Queue",
            seq: 7,
        },
    )
}
fn scene_operations(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::Operations {
            operations: vec![
                Operation {
                    seq: 1,
                    label: "Orientation",
                    status: OperationStatus::Finished { ok: true },
                },
                Operation {
                    seq: 2,
                    label: "History",
                    status: OperationStatus::Running,
                },
            ],
        },
    )
}
fn scene_ref_picker(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::RefPicker {
            refs: vec![
                MAIN.to_string(),
                OTHER.to_string(),
                "tags/0.1.0".to_string(),
            ],
            cursor: 0,
            unpublished_main: false,
            unpublished_current: false,
        },
    )
}
fn scene_refusal(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::Refusal {
            card: refusal_card(),
            cursor: 0,
        },
    )
}
fn scene_stale(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::Stale {
            operation: "Commit worktree changes".to_string(),
            cause: stikk_model::StaleCause::Repository,
            headline: "The repository changed while this was open".to_string(),
            gloss: "What you confirmed was built for a repository that has since moved on."
                .to_string(),
            next_steps: vec![NextStep {
                label: "Start again from a fresh preview".to_string(),
                target: NextTarget::Refresh,
            }],
            cursor: 0,
        },
    )
}
fn scene_commit_would_refuse(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::CommitWouldRefuse {
            paths: vec![stikk_core::RefusedPath {
                kind: stikk_core::ChangeKind::Unsupported,
                path: "build/socket".to_string(),
                reason: "unsupported entry kind: socket".to_string(),
            }],
        },
    )
}
fn scene_commit_would_refuse_declarations(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::CommitWouldRefuseDeclarations {
            declarations: vec![stikk_core::RefusedDeclaration {
                old_path: "docs/old.md".to_string(),
                new_path: "docs/new.md".to_string(),
                reason: "the destination is already tracked".to_string(),
            }],
        },
    )
}
fn scene_action_list(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::ActionList {
            subject: "this block".to_string(),
            entries: stikk_core::actions::here(
                Some(stikk_core::RowKind::Block),
                Some(stikk_core::ScreenKind::History),
            ),
            cursor: 0,
            readiness: Readiness::none(),
            ref_focused: true,
        },
    )
}
fn scene_palette(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::Palette {
            filter: String::new(),
            cursor: 0,
            readiness: Readiness::none(),
            ref_focused: true,
        },
    )
}
fn scene_refusals(palette: Palette) -> Staged {
    let mut history = stikk_core::RefusalHistory::new();
    history.record(
        "lock conflict: another prikk is writing this repository",
        "lock",
        OperationContext::LoadHistory,
    );
    over(
        palette,
        Overlay::Refusals {
            records: history.recent().into_iter().cloned().collect(),
            cursor: 0,
        },
    )
}
fn scene_confirmation(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::Confirmation {
            summary: Box::new(summary()),
            tier: Tier::Two,
            typed: String::new(),
            error: None,
        },
    )
}
fn scene_commit_message(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::CommitMessage {
            reff: MAIN.to_string(),
            typed: "a change worth recording".to_string(),
            messages_persist: true,
        },
    )
}
fn scene_undo_message(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::UndoMessage {
            block: id('1'),
            typed: "put back what that block changed".to_string(),
        },
    )
}
fn scene_undo_recorded(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::UndoRecorded {
            result: Box::new(commit_result()),
        },
    )
}
fn scene_undo_stopped(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::UndoStopped {
            changed: vec!["src/main.rs".to_string()],
            text: "Nothing was committed. One file had already been put back.".to_string(),
        },
    )
}
fn scene_commit_result(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::CommitResult {
            result: commit_result(),
        },
    )
}
fn scene_seal_consent(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::SealConsent {
            reff: MAIN.to_string(),
            acknowledged: false,
        },
    )
}
fn scene_seal_result(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::SealResult {
            result: stikk_prikk::SealResult {
                patches: 2,
                block_id: id('1'),
                reff: MAIN.to_string(),
                ref_state: id('d'),
                notes: Vec::new(),
            },
        },
    )
}
fn scene_ref_name(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::RefName {
            kind: RefNameKind::Branch,
            source: MAIN.to_string(),
            typed: "feature".to_string(),
        },
    )
}
fn scene_export_path(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::ExportPath {
            reff: MAIN.to_string(),
            typed: "shared/main.bundle".to_string(),
        },
    )
}
fn scene_save_path(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::SavePath {
            what: SavePathKind::File {
                block: id('1'),
                path: "src/main.rs".to_string(),
            },
            heading: "Save this file, as it was here".to_string(),
            typed: "saved/main.rs".to_string(),
        },
    )
}
fn scene_save_result(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::SaveResult {
            lines: vec!["saved src/main.rs to saved/main.rs".to_string()],
        },
    )
}
fn scene_trust_public_key(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::TrustPublicKey {
            typed: id('3'),
            error: None,
        },
    )
}
fn scene_trust_key_id(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::TrustKeyId {
            public_key: id('3'),
            typed: "maintainer-3".to_string(),
            error: None,
        },
    )
}
fn scene_trust_conflict(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::TrustConflict {
            warning: Box::new(warning()),
        },
    )
}
fn scene_trust_result(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::TrustResult {
            printed: vec!["maintainer key trusted: maintainer-3".to_string()],
            see_verify: None,
        },
    )
}
fn scene_bundle_path(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::BundlePath {
            typed: "shared/feature.bundle".to_string(),
        },
    )
}
fn scene_tag_message(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::TagMessage {
            name: "0.1.0".to_string(),
            target: MAIN.to_string(),
            typed: "the first release".to_string(),
        },
    )
}
fn scene_export_result(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::ExportResult {
            printed: vec!["wrote shared/main.bundle".to_string()],
        },
    )
}
fn scene_import_result(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::ImportResult {
            printed: vec!["imported as remotes/feature".to_string()],
        },
    )
}
fn scene_ref_write_result(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::RefWriteResult {
            kind: RefWriteKind::CreateBranch,
            lines: vec![format!("created heads/feature at {}", id('4'))],
        },
    )
}
fn scene_switch_result(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::SwitchResult {
            printed: vec!["switched to heads/feature".to_string()],
            noted: vec!["9 files written, 1 deleted".to_string()],
        },
    )
}
fn scene_merge_result(palette: Palette) -> Staged {
    over(
        palette,
        Overlay::MergeResult {
            printed: vec![format!("merged {OTHER} into {MAIN}")],
        },
    )
}

// -------------------------------------------------------------------------------------------
// The scene list, and the coverage the compiler checks.

/// Every scene, by the name the manifest uses.
const SCENES: &[(&str, Build)] = &[
    // The twelve `Focus` variants.
    ("orientation", scene_orientation),
    ("loading", scene_loading),
    ("history", scene_history),
    ("block-detail", scene_block_detail),
    ("changes", scene_changes),
    ("queue", scene_queue),
    ("patch-detail", scene_patch_detail),
    ("compare", scene_compare),
    ("merge-plan", scene_merge_plan),
    ("bundle", scene_bundle),
    ("verify", scene_verify),
    ("trust", scene_trust),
    // The thirty-four `Overlay` variants.
    ("glossary", scene_glossary),
    ("loading-overlay", scene_loading_overlay),
    ("operations", scene_operations),
    ("ref-picker", scene_ref_picker),
    ("refusal", scene_refusal),
    ("stale", scene_stale),
    ("commit-would-refuse", scene_commit_would_refuse),
    (
        "commit-would-refuse-declarations",
        scene_commit_would_refuse_declarations,
    ),
    ("action-list", scene_action_list),
    ("palette", scene_palette),
    ("refusals", scene_refusals),
    ("confirmation", scene_confirmation),
    ("commit-message", scene_commit_message),
    ("undo-message", scene_undo_message),
    ("undo-recorded", scene_undo_recorded),
    ("undo-stopped", scene_undo_stopped),
    ("commit-result", scene_commit_result),
    ("seal-consent", scene_seal_consent),
    ("seal-result", scene_seal_result),
    ("ref-name", scene_ref_name),
    ("export-path", scene_export_path),
    ("save-path", scene_save_path),
    ("save-result", scene_save_result),
    ("trust-public-key", scene_trust_public_key),
    ("trust-key-id", scene_trust_key_id),
    ("trust-conflict", scene_trust_conflict),
    ("trust-result", scene_trust_result),
    ("bundle-path", scene_bundle_path),
    ("tag-message", scene_tag_message),
    ("export-result", scene_export_result),
    ("import-result", scene_import_result),
    ("ref-write-result", scene_ref_write_result),
    ("switch-result", scene_switch_result),
    ("merge-result", scene_merge_result),
];

/// **The scene that shows each overlay** — exhaustive, with no `_` arm, and returning the builder
/// itself so an arm cannot name a scene that does not exist (§1.3).
fn scene_for_overlay(overlay: &Overlay) -> Build {
    match overlay {
        Overlay::Glossary => scene_glossary,
        Overlay::Loading { .. } => scene_loading_overlay,
        Overlay::Operations { .. } => scene_operations,
        Overlay::RefPicker { .. } => scene_ref_picker,
        Overlay::Refusal { .. } => scene_refusal,
        Overlay::Stale { .. } => scene_stale,
        Overlay::CommitWouldRefuse { .. } => scene_commit_would_refuse,
        Overlay::CommitWouldRefuseDeclarations { .. } => scene_commit_would_refuse_declarations,
        Overlay::ActionList { .. } => scene_action_list,
        Overlay::Palette { .. } => scene_palette,
        Overlay::Refusals { .. } => scene_refusals,
        Overlay::Confirmation { .. } => scene_confirmation,
        Overlay::CommitMessage { .. } => scene_commit_message,
        Overlay::UndoMessage { .. } => scene_undo_message,
        Overlay::UndoRecorded { .. } => scene_undo_recorded,
        Overlay::UndoStopped { .. } => scene_undo_stopped,
        Overlay::CommitResult { .. } => scene_commit_result,
        Overlay::SealConsent { .. } => scene_seal_consent,
        Overlay::SealResult { .. } => scene_seal_result,
        Overlay::RefName { .. } => scene_ref_name,
        Overlay::ExportPath { .. } => scene_export_path,
        Overlay::SavePath { .. } => scene_save_path,
        Overlay::SaveResult { .. } => scene_save_result,
        Overlay::TrustPublicKey { .. } => scene_trust_public_key,
        Overlay::TrustKeyId { .. } => scene_trust_key_id,
        Overlay::TrustConflict { .. } => scene_trust_conflict,
        Overlay::TrustResult { .. } => scene_trust_result,
        Overlay::BundlePath { .. } => scene_bundle_path,
        Overlay::TagMessage { .. } => scene_tag_message,
        Overlay::ExportResult { .. } => scene_export_result,
        Overlay::ImportResult { .. } => scene_import_result,
        Overlay::RefWriteResult { .. } => scene_ref_write_result,
        Overlay::SwitchResult { .. } => scene_switch_result,
        Overlay::MergeResult { .. } => scene_merge_result,
    }
}

/// **The scene that shows each focus** — exhaustive, for the same reason.
fn scene_for_focus(focus: &Focus<'_>) -> Build {
    match focus {
        Focus::Orientation(_) => scene_orientation,
        Focus::Loading(_) => scene_loading,
        Focus::History(..) => scene_history,
        Focus::BlockDetail(..) => scene_block_detail,
        Focus::Trust(..) => scene_trust,
        Focus::Changes(..) => scene_changes,
        Focus::Queue(..) => scene_queue,
        Focus::PatchDetail(..) => scene_patch_detail,
        Focus::Compare(_) => scene_compare,
        Focus::MergePlan(..) => scene_merge_plan,
        Focus::Bundle(..) => scene_bundle,
        Focus::Verify(..) => scene_verify,
    }
}

// -------------------------------------------------------------------------------------------
// The tests.

/// One frame's key in the manifest.
fn key(scene: &str, width: u16, height: u16, palette: &str, state: &str) -> String {
    format!("{scene} {width}x{height} {palette} {state}")
}

/// Every frame the matrix asks for, keyed as the manifest keys it (§1.2).
///
/// - **every scene at four sizes**, in `dark`;
/// - **every scene at 80 x 24** in each of the other four palettes;
/// - **every scene scrolled to rest** at the two sizes where a card most often overflows.
fn every_frame() -> BTreeMap<String, (u64, String)> {
    let mut frames = BTreeMap::new();
    for (scene, build) in SCENES {
        for (width, height) in SIZES {
            let staged = build(dark());
            let text = serialize(&draw(&staged.app, width, height));
            frames.insert(
                key(scene, width, height, "dark", "base"),
                (fnv1a(text.as_bytes()), text),
            );
        }
        for (name, palette) in palettes().into_iter().skip(1) {
            let staged = build(palette);
            let text = serialize(&draw(&staged.app, 80, 24));
            frames.insert(
                key(scene, 80, 24, name, "base"),
                (fnv1a(text.as_bytes()), text),
            );
        }
        for (width, height) in [(60, 15), (80, 24)] {
            let mut staged = build(dark());
            let text = serialize(&scrolled_to_rest(&mut staged.app, width, height));
            frames.insert(
                key(scene, width, height, "dark", "scrolled"),
                (fnv1a(text.as_bytes()), text),
            );
        }
    }
    frames
}

#[test]
fn every_scene_reaches_what_it_claims() {
    for (scene, build) in SCENES {
        let staged = build(dark());
        let reached = match staged.app.top_overlay() {
            Some(overlay) => scene_for_overlay(overlay),
            None => scene_for_focus(&staged.app.focus()),
        };
        assert!(
            std::ptr::fn_addr_eq(reached, *build),
            "scene {scene} does not reach the state it claims"
        );
    }
}

#[test]
fn no_scene_name_is_used_twice() {
    let mut names: Vec<&str> = SCENES.iter().map(|(name, _)| *name).collect();
    names.sort_unstable();
    let count = names.len();
    names.dedup();
    assert_eq!(names.len(), count, "two scenes share a name");
}

#[test]
fn every_frame_is_what_it_was() {
    let frames = every_frame();
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(MANIFEST);

    if std::env::var_os("STIKK_GOLDEN_WRITE").is_some() {
        let mut out = String::new();
        for (key, (digest, _)) in &frames {
            out.push_str(&format!("{key} {digest:016x}\n"));
        }
        std::fs::write(&manifest, out).expect("the manifest is writable");
        return;
    }

    let recorded = std::fs::read_to_string(&manifest).expect("the committed manifest");
    let recorded: BTreeMap<&str, &str> = recorded
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (key, digest) = line.rsplit_once(' ').expect("'<key> <digest>'");
            (key, digest)
        })
        .collect();

    let mut differ = Vec::new();
    for (key, (digest, text)) in &frames {
        let want = recorded.get(key.as_str());
        if want != Some(&format!("{digest:016x}").as_str()) {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(DIFF_DIR);
            std::fs::create_dir_all(&dir).expect("the diff directory");
            std::fs::write(dir.join(format!("{}.txt", key.replace(' ', "-"))), text)
                .expect("the frame is writable");
            differ.push(key.clone());
        }
    }
    let missing: Vec<&&str> = recorded
        .keys()
        .filter(|key| !frames.contains_key(**key))
        .collect();

    assert!(
        differ.is_empty() && missing.is_empty(),
        "{} frame(s) differ from the manifest and {} are gone from the matrix.\n\
         Differing: {differ:#?}\nGone: {missing:#?}\n\
         Each differing frame's full text is in target/golden-frames/. \
         **The manifest is not to be rewritten to make this pass** (RFC 067 Handoff A trap 1).",
        differ.len(),
        missing.len(),
    );
}
