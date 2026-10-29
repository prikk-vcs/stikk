//! Tests for the Bundle view (design TS-01; RFC 050 decision 3, Handoff A §4c).
//!
//! Every fixture is prikk 0.53.0's own output, from the §1 re-run
//! (`.git-exclude/reports/050-exchange/measure050-0.53.txt` and `measure050c-0.53.txt`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_core::{DIVERGED_CONFLICT, WOULD_BRING_FORWARD, sentence_for};
use stikk_prikk::{BundleConflict, BundlePreviewRead, Connectivity, EffectKind};

use super::*;
use crate::test_util::buffer_text;

/// `bundle verify`'s nine lines, verbatim.
const VERIFIED: &[&str] = &[
    "bundle verifies: heads/main",
    "RefState: d3172482cabc746238f75f9842b30f0d0dff24ec580f4206c532e76c3991a29c",
    "tip block: 33b2b1bb2183deade7783dd25bdd37b101206e12280b16096731849be62901a9",
    "objects: 6",
    "author key material: 1 present (continuity only, not a trust decision)",
    "repository format: 7",
    "tool version: 0.53.0",
    "note: this bundle contains one ref's closure only -- other refs in the source repository, \
     if any, are not included, and this bundle makes no claim about them",
    "note: this checks structural and internal consistency only -- no signature is \
     cryptographically verified (a standalone bundle carries no trust material to check one \
     against), and this bundle's own author-key section is recorded here, never independently \
     verified, the same as at import. A verified bundle is not yet a trusted one -- import it \
     and run `prikk verify` for that.",
];

const SEALED_NOTE: &str = "recorded signer identity, continuity only -- not a trust decision";
const RENAME_NOTE: &str = "a moved path is not yet distinguishable from a delete plus a create in this preview -- \
     until rename authoring lands, do not infer that any delete+create pair shown here was a move";
const CONFLICT_DETAIL: &str = "integrity error: EditText failed during localization for node \
     3a19e411d69484d0fb161ade5a1314958cffed2879110e40a834a72fdbf9e1ce span \
     e82f0ff279eebc030990061542c6b215c0417b442c38e2c3a6a4d669df987a1d: no occurrence's anchors \
     matched the record";

fn effect(path: &str, kind: EffectKind, before: Option<u64>, after: Option<u64>) -> BundleEffect {
    BundleEffect {
        path: path.to_string(),
        kind,
        current_bytes: before,
        after_bytes: after,
    }
}

fn opened(
    connectivity: Connectivity,
    conflict: Option<BundleConflict>,
    detail: Option<&str>,
    effects: Vec<BundleEffect>,
) -> BundleOpened {
    let preview = BundlePreviewRead {
        bundle_ref: "heads/main".to_string(),
        local_ref: "heads/main".to_string(),
        connectivity,
        conflict,
        conflict_detail: detail.map(ToString::to_string),
        sealed_by: vec!["z-maint".to_string()],
        sealed_by_note: SEALED_NOTE.to_string(),
        effects,
        rename_note: RENAME_NOTE.to_string(),
    };
    let sentence = sentence_for(connectivity, conflict).expect("a measured pair has a sentence");
    // **The sealers as a repository that has adopted nothing would judge them** (RFC 054):
    // these fixtures are a receiver meeting someone else's bundle for the first time.
    let sealers = stikk_core::SealerTrustRead::Judged(
        preview
            .sealed_by
            .iter()
            .map(|key_id| stikk_core::Sealer {
                key_id: key_id.clone(),
                trust: stikk_core::SealerTrust::NotTrusted,
            })
            .collect(),
    );
    BundleOpened::Read(Box::new(BundleView {
        path: "/home/someone/downloads/v2.bundle".to_string(),
        verified: VERIFIED.iter().map(ToString::to_string).collect(),
        preview,
        sentence,
        sealers,
    }))
}

fn fast_forward() -> BundleOpened {
    opened(
        Connectivity::FastForward,
        Some(BundleConflict::AppliesCleanly),
        None,
        vec![effect("a.txt", EffectKind::Edited, Some(2), Some(3))],
    )
}

/// A screen with its **box border dropped** and its runs of whitespace collapsed, so a phrase
/// that wrapped across a line break still matches -- the Glossary gate's own helper, for the
/// same reason: wrapping is the feature, and it should not be what defeats a content check.
fn flattened(screen: &str) -> String {
    screen
        .chars()
        .filter(|c| !matches!(c, '│' | '┌' | '┐' | '└' | '┘' | '─'))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn draw_at(view: &BundleOpened, width: u16, height: u16, offset: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let offset = std::cell::Cell::new(offset);
    terminal
        .draw(|f| render(view, &offset, &Palette::default(), f, f.area()))
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

/// **Capture: a fast-forward at 80×24.**
#[test]
fn capture_a_fast_forward_at_80x24() {
    let view = fast_forward();
    let text = draw_at(&view, 80, 24, 0);
    println!("{text}");
    // The resolved path, which is what prikk read.
    assert!(text.contains("/home/someone/downloads/v2.bundle"), "{text}");
    // prikk's verification, in prikk's words.
    assert!(text.contains("bundle verifies: heads/main"), "{text}");
    assert!(text.contains("objects: 6"), "{text}");
    // The title says what this view does, and what it does not.
    assert!(text.contains("Bundle"), "{text}");
    assert!(text.contains("read only"), "{text}");
    // **stikk advertises no key that does nothing** -- `I` arrives with Handoff B. prikk's own
    // closing note says *"import it and run `prikk verify`"*, which is prikk's sentence and
    // stays; what must not appear is an affordance of stikk's.
    assert!(!text.contains("Enter"), "{text}");
    assert!(!text.contains("I imports"), "{text}");
    assert!(!text.contains("to import"), "{text}");

    // The rest is below the fold at 24 rows, and reachable.
    let end = draw_at(&view, 80, 24, u16::MAX);
    println!("{end}");
    assert!(
        flattened(&end).contains("it would bring it forward"),
        "{end}"
    );
    assert!(
        WOULD_BRING_FORWARD.contains("it would bring it forward"),
        "the fragment above must stay a fragment of core's own sentence"
    );
    assert!(end.contains("edited"), "{end}");
    assert!(end.contains("a.txt"), "{end}");
    assert!(end.contains("(2 \u{2192} 3 bytes)"), "{end}");
}

/// **Capture: a conflict at 80×24** — stikk's sentence, and prikk's reason verbatim beneath it.
#[test]
fn capture_a_conflict_at_80x24() {
    let view = opened(
        Connectivity::Diverged,
        Some(BundleConflict::Conflict),
        Some(CONFLICT_DETAIL),
        Vec::new(),
    );
    let whole = draw_at(&view, 80, 60, 0);
    println!("{whole}");
    assert!(
        flattened(&whole).contains("does not apply on top of yours"),
        "{whole}"
    );
    assert!(
        DIVERGED_CONFLICT.contains("does not apply on top of yours"),
        "the fragment above must stay a fragment of core's own sentence"
    );
    // prikk's own reason, word for word -- not stikk's paraphrase of it.
    assert!(
        flattened(&whole).contains("EditText failed during localization"),
        "{whole}"
    );
    assert!(whole.contains("No files would change."), "{whole}");
}

/// **Capture: a file prikk would not read at 80×24** — prikk's words, and **nothing else**.
#[test]
fn capture_a_refused_file_at_80x24() {
    let view = BundleOpened::Refused {
        path: "/home/someone/downloads/v2.bundle".to_string(),
        message: "error: malformed persisted data: unexpected end of record".to_string(),
    };
    let text = draw_at(&view, 80, 24, 0);
    println!("{text}");
    // **The path, because prikk's words do not carry one** (review v1 C1): *"malformed
    // persisted data"* names no file, so a mistyped path and a damaged download would read
    // the same screen without it.
    assert!(text.contains("/home/someone/downloads/v2.bundle"), "{text}");
    // **A label naming prikk**, so the words below are not read as stikk's own prose.
    assert!(
        flattened(&text).contains("prikk could not read it:"),
        "{text}"
    );
    assert!(
        flattened(&text).contains("malformed persisted data"),
        "{text}"
    );
    // No preview was asked for, so none of its furniture is on screen.
    assert!(!text.contains("would change"), "{text}");
    assert!(!text.contains("Sealed by"), "{text}");
    assert!(!text.contains("bundle verifies"), "{text}");
}

/// **"Sealed by" is never alone**: prikk's continuity note is drawn with the ids, because the
/// ids by themselves read as an assurance nobody gave.
#[test]
fn the_key_ids_are_never_drawn_without_prikks_note() {
    let text = draw_at(&fast_forward(), 80, 60, 0);
    // **The heading and the id are two rows since RFC 054**, because each id now carries what
    // this repository says about it — a row of its own, in its own span.
    assert!(text.contains("Sealed by"), "{text}");
    assert!(text.contains("z-maint"), "{text}");
    assert!(flattened(&text).contains("not a trust decision"), "{text}");
    // **And the answer is beside the id**, which is what RFC 054 adds to this row.
    assert!(
        flattened(&text).contains("z-maint — not trusted by this repository"),
        "{text}"
    );
    // And prikk's rename note travels with the effects it qualifies.
    assert!(flattened(&text).contains("delete plus a create"), "{text}");
}

/// **Never a silent cut** (§4c): a long effects list says how many are left and where the rest
/// are, rather than stopping without a word.
#[test]
fn a_long_effects_list_counts_what_it_did_not_draw() {
    let many: Vec<BundleEffect> = (0..BUNDLE_EFFECTS_SHOWN + 7)
        .map(|n| {
            effect(
                &format!("src/file-{n:03}.rs"),
                EffectKind::Created,
                None,
                Some(9),
            )
        })
        .collect();
    let view = opened(
        Connectivity::NoLocalHistory,
        Some(BundleConflict::AppliesCleanly),
        None,
        many,
    );
    let text = draw_at(&view, 80, 200, 0);
    assert!(text.contains("57 files would change:"), "{text}");
    assert!(text.contains("src/file-000.rs"), "{text}");
    assert!(text.contains("src/file-049.rs"), "{text}");
    assert!(!text.contains("src/file-050.rs"), "{text}");
    assert!(
        flattened(&text).contains("and 7 more not shown of 57"),
        "{text}"
    );
    assert!(flattened(&text).contains("prikk bundle preview"), "{text}");
}

/// The scroll indicator appears only when there is something below, and the last row is
/// reachable (RFC 024: wrap, never clip).
#[test]
fn a_long_view_says_it_scrolls_and_its_last_row_is_reachable() {
    let view = fast_forward();
    // **The last row is `sealed_by_note`**, matched on its opening words: *"not a trust
    // decision"* alone also appears inside prikk's verification, which is on the first screen,
    // so it would prove nothing about the fold.
    let first = draw_at(&view, 80, 24, 0);
    assert!(first.contains("lines 1"), "{first}");
    assert!(
        !flattened(&first).contains("recorded signer identity"),
        "{first}"
    );
    let end = draw_at(&view, 80, 24, u16::MAX);
    assert!(
        flattened(&end).contains("recorded signer identity"),
        "{end}"
    );

    // Tall enough for everything: no indicator, nothing cut.
    let whole = draw_at(&view, 80, 60, 0);
    assert!(!whole.contains("lines 1"), "{whole}");
    assert!(
        flattened(&whole).contains("recorded signer identity"),
        "{whole}"
    );
}

/// A path carrying a bidirectional override is **neutralized before it reaches a cell**
/// (`C-T2a`): the escape is what the reader sees, and no raw control byte is in the buffer.
#[test]
fn repository_text_in_a_path_is_rendered_inert() {
    let view = opened(
        Connectivity::NoLocalHistory,
        Some(BundleConflict::AppliesCleanly),
        None,
        vec![effect(
            "src/\u{202e}gnp.txt",
            EffectKind::Created,
            None,
            Some(4),
        )],
    );
    let text = draw_at(&view, 80, 60, 0);
    assert!(
        !text.contains('\u{202e}'),
        "no raw override reaches a cell: {text}"
    );
    assert!(text.contains("\\u202e"), "{text}");
}

/// **The view advertises `I` exactly where the key would act** (Handoff B §3b): with the tip
/// line, it names the received branch; without it, it says why there is no key instead of
/// leaving a silence.
#[test]
fn the_import_key_is_advertised_only_when_the_tip_was_read() {
    let text = draw_at(&fast_forward(), 80, 60, 0);
    assert!(
        flattened(&text).contains("I imports this as remotes/heads/main"),
        "{text}"
    );

    let BundleOpened::Read(mut view) = fast_forward() else {
        panic!("a read bundle");
    };
    view.verified.retain(|line| !line.starts_with("tip block:"));
    let without = BundleOpened::Read(view);
    let text = draw_at(&without, 80, 60, 0);
    assert!(!text.contains("I imports"), "{text}");
    assert!(
        flattened(&text).contains("could not read a tip block"),
        "the absence is explained, not silent: {text}"
    );
}

/// **The comparison is labelled as a comparison** (review v1 C1): beside an advertised `I`, the
/// sentence must not read as what the key would do.
#[test]
fn the_comparison_says_importing_leaves_your_branch_alone() {
    let text = draw_at(&fast_forward(), 80, 60, 0);
    let flowed = flattened(&text);
    assert!(
        flowed.contains("Compared with your heads/main (importing leaves it as it is):"),
        "{text}"
    );
    // The sentence sits under it, and the local name is not also in a heading above.
    assert!(flowed.contains("it would bring it forward"), "{text}");
    assert!(
        !flowed.contains("heads/main → your heads/main"),
        "the name is said once, by the label: {text}"
    );
    assert!(flowed.contains("This bundle carries heads/main"), "{text}");
}

/// **Capture: the three judged sealer answers at 80 columns** (RFC 054 decision 3, as
/// amended) — stikk's sentence about what a name is worth beneath them, and prikk's own note
/// still beneath that, because what stikk adds is an answer prikk does not give, not a
/// replacement for what it does.
#[test]
fn capture_the_three_judged_sealer_answers_at_80_columns() {
    let mut view = match fast_forward() {
        BundleOpened::Read(view) => *view,
        other => panic!("expected a read view, got {other:?}"),
    };
    view.preview.sealed_by = vec![
        "ed25519-31f369d3abb7f64c".to_string(),
        "ed25519-9c0a44b1d2e3f501".to_string(),
        "ed25519-0000000000000000".to_string(),
    ];
    view.sealers = stikk_core::SealerTrustRead::Judged(vec![
        stikk_core::Sealer {
            key_id: "ed25519-31f369d3abb7f64c".to_string(),
            trust: stikk_core::SealerTrust::NamesYourKey,
        },
        stikk_core::Sealer {
            key_id: "ed25519-9c0a44b1d2e3f501".to_string(),
            trust: stikk_core::SealerTrust::NamesATrustedKey,
        },
        stikk_core::Sealer {
            key_id: "ed25519-0000000000000000".to_string(),
            trust: stikk_core::SealerTrust::NotTrusted,
        },
    ]);
    let text = draw_at(&BundleOpened::Read(Box::new(view)), 80, 60, 0);
    println!("{text}");
    let flat = flattened(&text);
    assert!(
        flat.contains("ed25519-31f369d3abb7f64c — names your key"),
        "{text}"
    );
    assert!(
        flat.contains("ed25519-9c0a44b1d2e3f501 — names a key this repository trusts"),
        "{text}"
    );
    assert!(
        flat.contains("ed25519-0000000000000000 — not trusted by this repository"),
        "{text}"
    );
    // The card never leaves a name reading as a signature: measured at 0.55.0, a sender can
    // seal under an id the receiver trusts and both preview and import exit 0.
    assert!(
        flat.contains(&flattened(stikk_core::NAMES_BUT_DOES_NOT_PROVE)),
        "what a name is worth: {text}"
    );
    assert!(
        flat.contains("not a trust decision"),
        "prikk's note: {text}"
    );
}

/// **Capture: the fourth answer at 80 columns.** Where the list could not be read every row
/// says so, and **stikk's sentence is absent**: nothing here named an adopted key, so there is
/// no claim for it to qualify (RFC 035 A3 — a failed read costs only what it would have
/// provided).
#[test]
fn capture_the_unreadable_sealer_answer_at_80_columns() {
    let mut view = match fast_forward() {
        BundleOpened::Read(view) => *view,
        other => panic!("expected a read view, got {other:?}"),
    };
    view.preview.sealed_by = vec!["ed25519-31f369d3abb7f64c".to_string()];
    view.sealers = stikk_core::SealerTrustRead::Judged(vec![stikk_core::Sealer {
        key_id: "ed25519-31f369d3abb7f64c".to_string(),
        trust: stikk_core::SealerTrust::Unknown,
    }]);
    let text = draw_at(&BundleOpened::Read(Box::new(view)), 80, 60, 0);
    println!("{text}");
    let flat = flattened(&text);
    assert!(
        flat.contains(
            "ed25519-31f369d3abb7f64c — whether this repository trusts it could not be read"
        ),
        "{text}"
    );
    assert!(
        !flat.contains("does not prove it"),
        "no claim to qualify: {text}"
    );
    assert!(
        flat.contains("not a trust decision"),
        "prikk's note: {text}"
    );
}

/// **A sealer id is chosen by whoever sealed the bundle**, so a long one must not be able to
/// push stikk's answer off the card: `check` accepted `not-a-key-id`, and nothing bounds the
/// length. At 80 columns with a 120-character id, the id is split and the answer still has a
/// row of its own.
#[test]
fn a_very_long_sealer_id_still_shows_its_answer_at_80_columns() {
    let long = "ed25519-".to_string() + &"a1b2c3d4".repeat(14);
    assert_eq!(long.chars().count(), 120, "the id under test");
    let mut view = match fast_forward() {
        BundleOpened::Read(view) => *view,
        other => panic!("expected a read view, got {other:?}"),
    };
    view.preview.sealed_by = vec![long.clone()];
    view.sealers = stikk_core::SealerTrustRead::Judged(vec![stikk_core::Sealer {
        key_id: long.clone(),
        trust: stikk_core::SealerTrust::NotTrusted,
    }]);
    let text = draw_at(&BundleOpened::Read(Box::new(view)), 80, 60, 0);
    println!("{text}");
    assert!(
        text.lines().all(|line| line.chars().count() <= 80),
        "nothing overflows the card: {text}"
    );
    let flat = flattened(&text);
    assert!(
        flat.contains("not trusted by this repository"),
        "the answer is drawn: {text}"
    );
    // The id itself is not lost either: it is split, so its halves are both on the card.
    assert!(flat.contains("ed25519-a1b2c3d4"), "{text}");
    assert!(flat.contains("a1b2c3d4a1b2c3d4"), "{text}");
}

/// **Below the band the row is 0.16.0's**: prikk's ids and note, and nothing stikk added
/// (RFC 054 decision 4). *Could not be read* would report a limit of stikk's own as if it had
/// asked and failed.
#[test]
fn below_the_band_the_sealer_row_is_prikks_ids_alone() {
    let mut view = match fast_forward() {
        BundleOpened::Read(view) => *view,
        other => panic!("expected a read view, got {other:?}"),
    };
    view.sealers = stikk_core::SealerTrustRead::Unavailable;
    let text = draw_at(&BundleOpened::Read(Box::new(view)), 80, 60, 0);
    let flat = flattened(&text);
    assert!(flat.contains("z-maint"), "the id is still drawn: {text}");
    assert!(!flat.contains("trusted by this repository"), "{text}");
    assert!(!flat.contains("could not be read"), "{text}");
    assert!(
        flat.contains("not a trust decision"),
        "prikk's note: {text}"
    );
}
