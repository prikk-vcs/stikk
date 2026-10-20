//! Tests for opening a bundle (RFC 050 Handoff A §4a).
//!
//! Every preview below is prikk 0.53.0's own report, pasted from
//! `.git-exclude/reports/050-exchange/measure050c-0.53.txt` as that file holds it, and the
//! verification from `measure050-0.53.txt`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use stikk_prikk::{BundleEffect, EffectKind, NullBackend};

use super::*;

/// `bundle verify`'s nine lines, verbatim — prikk's two closing notes included.
fn verified_lines() -> Vec<String> {
    [
        "bundle verifies: heads/main",
        "RefState: d3172482cabc746238f75f9842b30f0d0dff24ec580f4206c532e76c3991a29c",
        "tip block: 33b2b1bb2183deade7783dd25bdd37b101206e12280b16096731849be62901a9",
        "objects: 6",
        "author key material: 1 present (continuity only, not a trust decision)",
        "repository format: 7",
        "tool version: 0.53.0",
        "note: this bundle contains one ref's closure only -- other refs in the source \
         repository, if any, are not included, and this bundle makes no claim about them",
        "note: this checks structural and internal consistency only -- no signature is \
         cryptographically verified (a standalone bundle carries no trust material to check one \
         against), and this bundle's own author-key section is recorded here, never independently \
         verified, the same as at import. A verified bundle is not yet a trusted one -- import it \
         and run `prikk verify` for that.",
    ]
    .iter()
    .map(ToString::to_string)
    .collect()
}

/// prikk's `sealed_by_note`, word for word.
const SEALED_NOTE: &str = "recorded signer identity, continuity only -- not a trust decision";

/// prikk's `rename_note`, word for word.
const RENAME_NOTE: &str = "a moved path is not yet distinguishable from a delete plus a create in this preview -- \
     until rename authoring lands, do not infer that any delete+create pair shown here was a move";

/// prikk's replay error for the conflicting pair, word for word.
const CONFLICT_DETAIL: &str = "integrity error: EditText failed during localization for node \
     3a19e411d69484d0fb161ade5a1314958cffed2879110e40a834a72fdbf9e1ce span \
     e82f0ff279eebc030990061542c6b215c0417b442c38e2c3a6a4d669df987a1d: no occurrence's anchors \
     matched the record";

fn preview(
    connectivity: Connectivity,
    conflict: Option<BundleConflict>,
    conflict_detail: Option<&str>,
    effects: Vec<BundleEffect>,
) -> BundlePreviewRead {
    BundlePreviewRead {
        bundle_ref: "heads/main".to_string(),
        local_ref: "heads/main".to_string(),
        connectivity,
        conflict,
        conflict_detail: conflict_detail.map(ToString::to_string),
        sealed_by: vec!["z-maint".to_string()],
        sealed_by_note: SEALED_NOTE.to_string(),
        effects,
        rename_note: RENAME_NOTE.to_string(),
    }
}

fn edited_a() -> Vec<BundleEffect> {
    vec![BundleEffect {
        path: "a.txt".to_string(),
        kind: EffectKind::Edited,
        current_bytes: Some(2),
        after_bytes: Some(3),
    }]
}

fn created_two() -> Vec<BundleEffect> {
    vec![
        BundleEffect {
            path: "a.txt".to_string(),
            kind: EffectKind::Created,
            current_bytes: None,
            after_bytes: Some(2),
        },
        BundleEffect {
            path: "b.txt".to_string(),
            kind: EffectKind::Created,
            current_bytes: None,
            after_bytes: Some(2),
        },
    ]
}

/// A backend at 0.53 that verifies and previews.
fn opens(read: BundlePreviewRead) -> NullBackend {
    NullBackend::supported()
        .with_version(0, 53, 0)
        .with_bundle_verify(verified_lines())
        .with_bundle_preview(read)
}

fn opened(backend: &NullBackend) -> BundleOpened {
    open_bundle(backend, Path::new("/repo"), Path::new("/tmp/v1.bundle")).expect("a bundle opens")
}

/// **Every row of the handoff's table**, each from a real measured report, maps to the sentence
/// the handoff means — and `diverged` is the only connectivity whose conflict word changes the
/// answer.
#[test]
fn each_measured_connectivity_gets_its_own_sentence() {
    use BundleConflict as K;
    use Connectivity as C;
    for (tag, read, want) in [
        (
            "fresh",
            preview(
                C::NoLocalHistory,
                Some(K::AppliesCleanly),
                None,
                created_two(),
            ),
            NEW_TO_THIS_REPOSITORY,
        ),
        (
            "unrelated",
            preview(C::DoesNotConnect, None, None, Vec::new()),
            SHARES_NOTHING,
        ),
        (
            "included",
            preview(C::AlreadyIncluded, None, None, Vec::new()),
            ALREADY_HELD,
        ),
        (
            "fast-forward",
            preview(C::FastForward, Some(K::AppliesCleanly), None, edited_a()),
            WOULD_BRING_FORWARD,
        ),
        (
            "diverged, clean",
            preview(C::Diverged, Some(K::AppliesCleanly), None, edited_a()),
            DIVERGED_CLEAN,
        ),
        (
            "diverged, conflict",
            preview(
                C::Diverged,
                Some(K::Conflict),
                Some(CONFLICT_DETAIL),
                Vec::new(),
            ),
            DIVERGED_CONFLICT,
        ),
        (
            "diverged, undetermined",
            preview(C::Diverged, Some(K::Undetermined), None, Vec::new()),
            DIVERGED_UNDETERMINED,
        ),
    ] {
        let backend = opens(read);
        let BundleOpened::Read(view) = opened(&backend) else {
            panic!("{tag}: a verified bundle reads");
        };
        assert_eq!(view.sentence, want, "{tag}");
        assert_eq!(
            view.verified,
            verified_lines(),
            "{tag}: prikk's verification travels whole"
        );
        assert_eq!(view.path, "/tmp/v1.bundle", "{tag}");
    }
}

/// **No sentence carries a name.** The branch names are drawn beside the sentence, through
/// `identifier`, so repository text and stikk's own words never share a span (`C-T2a`, RFC 044
/// C1 applied here).
#[test]
fn no_sentence_names_a_branch_or_a_path() {
    for sentence in [
        NEW_TO_THIS_REPOSITORY,
        SHARES_NOTHING,
        ALREADY_HELD,
        WOULD_BRING_FORWARD,
        DIVERGED_CLEAN,
        DIVERGED_CONFLICT,
        DIVERGED_UNDETERMINED,
    ] {
        for name in ["heads/main", "heads/", "a.txt", "remotes/", "z-maint"] {
            assert!(
                !sentence.contains(name),
                "`{name}` must be drawn beside the sentence, not inside it: {sentence}"
            );
        }
    }
}

/// **The conflicting row quotes prikk**, and only that row does.
#[test]
fn only_the_conflicting_row_carries_prikks_reason() {
    let backend = opens(preview(
        Connectivity::Diverged,
        Some(BundleConflict::Conflict),
        Some(CONFLICT_DETAIL),
        Vec::new(),
    ));
    let BundleOpened::Read(view) = opened(&backend) else {
        panic!("a verified bundle reads");
    };
    assert_eq!(view.conflict_detail(), Some(CONFLICT_DETAIL));

    let backend = opens(preview(
        Connectivity::FastForward,
        Some(BundleConflict::AppliesCleanly),
        None,
        edited_a(),
    ));
    let BundleOpened::Read(view) = opened(&backend) else {
        panic!("a verified bundle reads");
    };
    assert_eq!(view.conflict_detail(), None);
}

/// **A fast-forward that does not apply cleanly is a shape error, not a sentence** (§4a):
/// prikk replays a fast-forward onto a tip the bundle already contains, so the replay cannot
/// fail. A report that says it did is one stikk misread.
#[test]
fn an_impossible_combination_refuses_rather_than_invents_a_sentence() {
    for conflict in [BundleConflict::Conflict, BundleConflict::Undetermined] {
        let error = sentence_for(Connectivity::FastForward, Some(conflict))
            .expect_err("an impossible combination has no sentence");
        let text = error.to_string();
        assert!(text.contains("fast-forward"), "{text}");
        assert!(text.contains(conflict.name()), "{text}");
    }
    // The two that *are* possible still answer.
    assert_eq!(
        sentence_for(
            Connectivity::FastForward,
            Some(BundleConflict::AppliesCleanly)
        )
        .expect("a clean fast-forward has a sentence"),
        WOULD_BRING_FORWARD
    );
}

/// **A bundle prikk refuses is never previewed** (§4a): prikk's words, verbatim, and nothing
/// else — proved by the backend's own call counter, not by the absence of a field.
#[test]
fn a_refused_bundle_is_prikks_words_and_no_preview_is_asked_for() {
    let backend = NullBackend::supported()
        .with_version(0, 53, 0)
        .with_bundle_verify_refusal("error: malformed persisted data: unexpected end of record")
        .with_bundle_preview(preview(
            Connectivity::FastForward,
            Some(BundleConflict::AppliesCleanly),
            None,
            edited_a(),
        ));
    let BundleOpened::Refused(message) = opened(&backend) else {
        panic!("a malformed bundle is refused");
    };
    assert!(
        message.contains("malformed persisted data: unexpected end of record"),
        "prikk's words, kept: {message}"
    );
    assert_eq!(
        backend.bundle_preview_calls(),
        0,
        "a file prikk will not read has nothing to preview"
    );
}

/// **Below the band nothing is asked of prikk**, and stikk says so in its own voice, naming both
/// the number it needs and the number it found (`C-T4d`).
#[test]
fn below_the_band_stikk_says_so_in_its_own_words() {
    let backend = NullBackend::supported().with_version(0, 52, 0);
    let BundleOpened::Unavailable(message) = opened(&backend) else {
        panic!("below the band is unavailable, not refused");
    };
    assert!(message.contains("0.53"), "{message}");
    assert!(message.contains("0.52"), "{message}");
    assert!(
        message.contains("The rest of stikk works."),
        "a band is not a broken session: {message}"
    );
    assert_eq!(backend.bundle_preview_calls(), 0);
}

/// prikk's two notes are carried to the view, because the key ids and the effects each read as a
/// claim prikk did not make without them.
#[test]
fn prikks_two_notes_reach_the_view() {
    let backend = opens(preview(
        Connectivity::NoLocalHistory,
        Some(BundleConflict::AppliesCleanly),
        None,
        created_two(),
    ));
    let BundleOpened::Read(view) = opened(&backend) else {
        panic!("a verified bundle reads");
    };
    assert_eq!(view.preview.sealed_by, vec!["z-maint".to_string()]);
    assert_eq!(view.preview.sealed_by_note, SEALED_NOTE);
    assert_eq!(view.preview.rename_note, RENAME_NOTE);
    assert_eq!(view.preview.effects, created_two());
}
