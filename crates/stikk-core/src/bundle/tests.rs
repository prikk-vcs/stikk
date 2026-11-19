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
        // The pre-0.56 shape: prikk checked no seal and listed no refusal. The tests that want
        // either build it from here (RFC 063 F6).
        seal_outcomes: None,
        import_refusals: None,
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
    let BundleOpened::Refused { path, message } = opened(&backend) else {
        panic!("a malformed bundle is refused");
    };
    assert!(
        message.contains("malformed persisted data: unexpected end of record"),
        "prikk's words, kept: {message}"
    );
    // **The path travels with the refusal** (review v1 C1): prikk's own words name no file
    // here, so a reader who mistyped a path could not otherwise tell that from a damaged
    // download.
    assert_eq!(path, "/tmp/v1.bundle");
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

// =============================================================================================
// RFC 050 Handoff B — the import preview and its two guards.
//
// prikk's own `bundle import` report at 0.53, from `measure050b-0.53.txt`, is what success
// carries; the guards are F2's measurement turned into a refusal that never reaches prikk.
// =============================================================================================

use stikk_prikk::{BlockRow, History};

/// prikk's import report, verbatim.
fn import_lines() -> Vec<String> {
    [
        "received remotes/heads/main",
        "RefState: 89ab4b25e0c3826a679db6d1754d01351665cfb8b3c8f7b2c7a2b4d880b96b0b",
        "objects: 9",
        "new objects: 3",
        "author key material: 1 recorded (continuity only, not a trust decision)",
        "note: no local ref was created or advanced, and no MAINTAINER key was trusted; this \
         repository now holds the received history at remotes/heads/main, which `prikk log \
         --ref remotes/heads/main` and `prikk show` read, and `prikk verify` checks once \
         `trust maintainer add` trusts the sealing key",
    ]
    .iter()
    .map(ToString::to_string)
    .collect()
}

/// A full 64-hex block id ending in `tag`, so a lineage can hold distinguishable ones.
fn id(tag: &str) -> String {
    format!("{tag}{}", "0".repeat(64 - tag.len()))
}

/// `bundle verify`'s lines with `tip` as the pinned `tip block:` value.
fn verified_with_tip(tip: &str) -> Vec<String> {
    verified_lines()
        .into_iter()
        .map(|line| {
            if line.starts_with("tip block:") {
                format!("tip block: {tip}")
            } else {
                line
            }
        })
        .collect()
}

fn received_entry() -> stikk_prikk::RefEntry {
    stikk_prikk::RefEntry {
        name: "remotes/heads/main".to_string(),
        id: id("aa"),
        closed: false,
        received: true,
    }
}

fn block(block_id: &str, seq: u64) -> BlockRow {
    BlockRow {
        block_id: block_id.to_string(),
        ref_state_id: format!("rs-{block_id}"),
        update_seq: seq,
        kind: "Normal".to_string(),
        rollback_block: false,
        parents: 1,
        patches: 1,
        rollback_patches: 0,
        required_attestations: 0,
        messages: Vec::new(),
        previous_ref_state: None,
    }
}

/// A backend at 0.53 whose received branch holds `lineage`, newest first, and whose bundle
/// verifies with `tip`.
fn importing(tip: &str, lineage: Vec<BlockRow>) -> NullBackend {
    NullBackend::supported()
        .with_version(0, 53, 0)
        .with_bundle_verify(verified_with_tip(tip))
        .with_bundle_preview(preview(
            Connectivity::FastForward,
            Some(BundleConflict::AppliesCleanly),
            None,
            edited_a(),
        ))
        .with_bundle_import(import_lines())
        .with_refs(vec![received_entry()])
        .with_history_of(
            "remotes/heads/main",
            History {
                reff: "remotes/heads/main".to_string(),
                blocks: lineage,
            },
        )
}

fn previewed(backend: &NullBackend, limit: usize) -> ImportPreview {
    import_preview(
        backend,
        Path::new("/repo"),
        Path::new("/tmp/v1.bundle"),
        limit,
    )
    .expect("a preview")
}

fn blocked_for(backend: &NullBackend, limit: usize) -> String {
    match previewed(backend, limit) {
        ImportPreview::Blocked(reason) => reason,
        other => panic!("expected a block in stikk's own words, got {other:?}"),
    }
}

/// **Guard 1: the bundle is already received.** The received branch's tip *is* the bundle's
/// tip, so importing changes nothing — and `import` is never spawned.
#[test]
fn an_already_received_bundle_is_declined_without_importing() {
    let backend = importing(&id("aa"), vec![block(&id("aa"), 2), block(&id("bb"), 1)]);
    assert_eq!(blocked_for(&backend, 200), ALREADY_RECEIVED);
    assert_eq!(
        backend.bundle_import_calls(),
        0,
        "a guard that declined after the write would be no guard"
    );
}

/// **Guard 2: the bundle is older than what was received** — F2's measurement, as a refusal.
#[test]
fn an_older_bundle_is_declined_without_importing() {
    let backend = importing(&id("bb"), vec![block(&id("aa"), 2), block(&id("bb"), 1)]);
    let reason = blocked_for(&backend, 200);
    assert_eq!(reason, WOULD_MOVE_THE_RECEIVED_BRANCH_BACK);
    assert!(
        reason.contains("move that branch back"),
        "the reason says what would happen, not that something is wrong: {reason}"
    );
    assert_eq!(backend.bundle_import_calls(), 0);
}

/// **Guard 3: the lineage came back full and the tip is not in it**, so stikk cannot tell.
/// It names prikk's own command as the way out rather than guessing in the direction that
/// writes.
#[test]
fn a_lineage_longer_than_the_bound_declines_and_names_the_way_out() {
    let lineage: Vec<BlockRow> = (0..3u64)
        .map(|n| block(&id(&format!("c{n}")), 3 - n))
        .collect();
    let backend = importing(&id("ff"), lineage);
    // The bound is what makes this case: the same lineage read with room to spare imports.
    let reason = blocked_for(&backend, 3);
    assert_eq!(reason, LINEAGE_TOO_LONG_TO_TELL);
    assert!(
        reason.contains("prikk bundle import"),
        "the way out is named: {reason}"
    );
    assert_eq!(backend.bundle_import_calls(), 0);
    assert!(matches!(
        previewed(&backend, 200),
        ImportPreview::Ready { .. }
    ));
}

/// A bundle newer than everything received **arms the card**, and the card is Tier Two with no
/// key, labelled rows, stikk's sentence and prikk's caveat.
#[test]
fn a_newer_bundle_arms_a_tier_two_card_that_asks_for_no_key() {
    let backend = importing(&id("ff"), vec![block(&id("aa"), 2), block(&id("bb"), 1)]);
    let ImportPreview::Ready { token } = previewed(&backend, 200) else {
        panic!("a newer bundle imports");
    };
    assert_eq!(token.tier(), stikk_model::Tier::Two);
    let summary = token.summary();
    assert_eq!(summary.operation, "Import");
    assert_eq!(
        summary.capability, None,
        "an import signs nothing, so the card claims no key"
    );
    let rows: Vec<(Option<&str>, &str)> = summary
        .target_ids
        .iter()
        .map(|row| (row.role, row.id.as_str()))
        .collect();
    assert_eq!(
        rows,
        vec![
            (Some("received as"), "remotes/heads/main"),
            (Some("tip"), id("ff").get(..12).expect("a short id")),
            (Some("file"), "/tmp/v1.bundle"),
        ]
    );
    // The consequence is stikk's, and names nothing (RFC 044 C1).
    for name in ["remotes/heads/main", "heads/main", "/tmp/v1.bundle"] {
        assert!(
            !summary.consequence.contains(name),
            "`{name}` is a target row, not prose: {}",
            summary.consequence
        );
    }
    assert!(summary.consequence.contains("read-only history"));
    assert!(
        summary
            .consequence
            .contains("nobody's signing key is trusted by this")
    );
    // **prikk's note is on the card, as an aside, with the ids it qualifies** (review v1 C1) --
    // and the sentence about the local branch says, in its label, that importing leaves it be.
    let asides: Vec<(&str, &str, Vec<String>)> = summary
        .asides
        .iter()
        .map(|aside| (aside.before, aside.name.as_str(), aside.body.clone()))
        .collect();
    assert_eq!(
        asides,
        vec![
            (
                COMPARED_WITH_BEFORE,
                "heads/main",
                vec![WOULD_BRING_FORWARD.to_string()]
            ),
            (SEALED_BY_BEFORE, "z-maint", vec![SEALED_NOTE.to_string()]),
        ]
    );
    assert!(
        summary.asides[0]
            .after
            .contains("importing leaves it as it is"),
        "the label says what the comparison is not: {}",
        summary.asides[0].after
    );
    assert!(summary.branch_notice.is_none(), "not a target row");
    assert!(summary.history_notice.is_none(), "not a target row");
}

/// **The import card's sealer aside, in band**: the row, then what a name is worth, then
/// prikk's own note — in that order (RFC 054 decision 3, as amended). The sentence is there
/// because the row says the id **names** an adopted key, and preview cannot know more than
/// that.
#[test]
fn a_sealer_naming_a_trusted_key_carries_the_sentence_above_prikks_note() {
    let backend = importing(&id("ff"), vec![block(&id("aa"), 2), block(&id("bb"), 1)])
        .with_version(0, 54, 0)
        .with_trusted_keys(vec![stikk_prikk::TrustedKey {
            key_id: "z-maint".to_string(),
            public_key: "ab".repeat(32),
        }]);
    let ImportPreview::Ready { token } = previewed(&backend, 200) else {
        panic!("a newer bundle imports");
    };
    let summary = token.summary();
    let aside = summary
        .asides
        .iter()
        .find(|aside| aside.before == SEALED_BY_BEFORE)
        .expect("the sealer aside");
    assert_eq!(
        aside.body,
        vec![
            "z-maint — names a key this repository trusts".to_string(),
            crate::trust::NAMES_BUT_DOES_NOT_PROVE.to_string(),
            SEALED_NOTE.to_string(),
        ]
    );
}

/// **And it is absent where nothing names an adopted key.** Nobody has been told the id means
/// anything, so a caution about it would be noise on a card that is already long.
#[test]
fn an_untrusted_sealer_gets_the_row_and_prikks_note_and_nothing_else() {
    let backend = importing(&id("ff"), vec![block(&id("aa"), 2), block(&id("bb"), 1)])
        .with_version(0, 54, 0)
        .with_trusted_keys(Vec::new());
    let ImportPreview::Ready { token } = previewed(&backend, 200) else {
        panic!("a newer bundle imports");
    };
    let summary = token.summary();
    let aside = summary
        .asides
        .iter()
        .find(|aside| aside.before == SEALED_BY_BEFORE)
        .expect("the sealer aside");
    assert_eq!(
        aside.body,
        vec![
            "z-maint — not trusted by this repository".to_string(),
            SEALED_NOTE.to_string(),
        ]
    );
}

/// **Where the received branch does not exist yet, there is nothing to move back**: the guards
/// read no lineage and the import arms.
#[test]
fn a_first_import_is_not_guarded_against_a_branch_that_is_not_there() {
    let backend = NullBackend::supported()
        .with_version(0, 53, 0)
        .with_bundle_verify(verified_with_tip(&id("ff")))
        .with_bundle_preview(preview(
            Connectivity::NoLocalHistory,
            Some(BundleConflict::AppliesCleanly),
            None,
            created_two(),
        ))
        .with_bundle_import(import_lines())
        .with_refs(Vec::new());
    assert!(matches!(
        previewed(&backend, 200),
        ImportPreview::Ready { .. }
    ));
}

/// **A file prikk will not verify blocks with prikk's words, and is never previewed or
/// imported** — the same rule `open_bundle` follows, inside the preview this time.
#[test]
fn a_refused_file_blocks_the_import_with_prikks_words() {
    let backend = NullBackend::supported()
        .with_version(0, 53, 0)
        .with_bundle_verify_refusal("error: malformed persisted data: invalid bundle magic")
        .with_bundle_import(import_lines());
    let reason = blocked_for(&backend, 200);
    assert!(reason.contains("invalid bundle magic"), "{reason}");
    assert_eq!(backend.bundle_preview_calls(), 0);
    assert_eq!(backend.bundle_import_calls(), 0);
}

/// **No tip line, no import.** Without the pinned line there is no guard, and an import with no
/// guard is the one F2 measured.
#[test]
fn a_verification_with_no_tip_line_blocks_the_import() {
    let without: Vec<String> = verified_lines()
        .into_iter()
        .filter(|line| !line.starts_with("tip block:"))
        .collect();
    let backend = NullBackend::supported()
        .with_version(0, 53, 0)
        .with_bundle_verify(without)
        .with_bundle_import(import_lines());
    let reason = blocked_for(&backend, 200);
    assert!(reason.contains("no `tip block:` line"), "{reason}");
    assert_eq!(backend.bundle_preview_calls(), 0);
    assert_eq!(backend.bundle_import_calls(), 0);
}

/// Below the band the preview blocks in stikk's own voice, and nothing is spawned.
#[test]
fn below_the_band_the_import_preview_blocks() {
    let backend = NullBackend::supported().with_version(0, 52, 0);
    let reason = blocked_for(&backend, 200);
    assert!(reason.contains("0.53"), "{reason}");
    assert!(reason.contains("0.52"), "{reason}");
    assert_eq!(backend.bundle_import_calls(), 0);
}

// =============================================================================================
// RFC 050 Handoff B — the export preview and its two destination rules.
// =============================================================================================

fn branch_entry(name: &str, closed: bool, received: bool) -> stikk_prikk::RefEntry {
    stikk_prikk::RefEntry {
        name: name.to_string(),
        id: id("ee"),
        closed,
        received,
    }
}

/// A backend at 0.53 with one open branch and a tip to name.
fn exporting(entries: Vec<stikk_prikk::RefEntry>) -> NullBackend {
    NullBackend::supported()
        .with_version(0, 53, 0)
        .with_refs(entries)
        .with_history_of(
            "heads/main",
            History {
                reff: "heads/main".to_string(),
                blocks: vec![block(&id("dd"), 1)],
            },
        )
        .with_bundle_export(vec!["wrote the bundle".to_string()])
}

fn export_blocked_for(backend: &NullBackend, reff: &str, out: &Path) -> String {
    match export_preview(backend, Path::new("/repo"), reff, out) {
        Ok(ExportPreview::Blocked(reason)) => reason,
        other => panic!("expected a block, got {other:?}"),
    }
}

/// **The destination is outside the repository, and does not exist**: the card arms, labelled.
#[test]
fn an_exportable_branch_arms_a_tier_two_card_with_no_key() {
    let dir = std::env::temp_dir().join(format!("stikk-export-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let out = dir.join("main-ddddddddd.bundle");

    let backend = exporting(vec![branch_entry("heads/main", false, false)]);
    let Ok(ExportPreview::Ready { token }) =
        export_preview(&backend, Path::new("/repo"), "heads/main", &out)
    else {
        panic!("an open published branch exports");
    };
    assert_eq!(token.tier(), stikk_model::Tier::Two);
    let summary = token.summary();
    assert_eq!(summary.operation, "Export");
    assert_eq!(summary.capability, None, "an export signs nothing");
    let roles: Vec<Option<&str>> = summary.target_ids.iter().map(|row| row.role).collect();
    assert_eq!(
        roles,
        vec![Some("branch"), Some("tip"), Some("file")],
        "every row says what it is"
    );
    assert_eq!(summary.target_ids[0].id, "heads/main");
    assert_eq!(summary.target_ids[2].id, out.display().to_string());
    assert_eq!(backend.bundle_export_calls(), 0, "a preview writes nothing");
    let _ = std::fs::remove_dir_all(&dir);
}

/// **Inside the repository is blocked** — `C-E2`'s own resolved check, applied to the one file
/// stikk asks prikk to write — and `export` is never spawned.
#[test]
fn a_destination_inside_the_repository_is_declined_without_exporting() {
    let repo = std::env::temp_dir().join(format!("stikk-export-repo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(&repo).expect("a scratch repository");
    let backend = exporting(vec![branch_entry("heads/main", false, false)]);

    for inside in [repo.join("out.bundle"), repo.join("deeper/out.bundle")] {
        let reason = match export_preview(&backend, &repo, "heads/main", &inside) {
            Ok(ExportPreview::Blocked(reason)) => reason,
            other => panic!("{}: expected a block, got {other:?}", inside.display()),
        };
        assert_eq!(reason, INSIDE_THE_REPOSITORY, "{}", inside.display());
    }
    assert_eq!(backend.bundle_export_calls(), 0);
    let _ = std::fs::remove_dir_all(&repo);
}

/// **An existing file is blocked**, and the file is not touched: stikk does not overwrite, and
/// `--force` is never passed either.
#[test]
fn an_existing_destination_is_declined_without_exporting() {
    let dir = std::env::temp_dir().join(format!("stikk-export-exists-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let out = dir.join("taken.bundle");
    std::fs::write(&out, b"not a bundle").expect("a file in the way");

    let backend = exporting(vec![branch_entry("heads/main", false, false)]);
    assert_eq!(
        export_blocked_for(&backend, "heads/main", &out),
        DESTINATION_EXISTS
    );
    assert_eq!(backend.bundle_export_calls(), 0);
    assert_eq!(
        std::fs::read(&out).expect("still there"),
        b"not a bundle",
        "the file in the way is untouched"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **A dangling symlink is something there** (RFC 053 Handoff B §1): `Path::exists` follows the
/// link and sees nothing, prikk's `destination_exists` sees the link and refuses. stikk now asks
/// prikk's question, so its own sentence answers first and `export` is still never spawned.
///
/// **Unix only**: a symlink on Windows needs a privilege the runner may not hold.
#[cfg(unix)]
#[test]
fn a_dangling_symlink_destination_is_declined_without_exporting() {
    let dir = std::env::temp_dir().join(format!("stikk-export-dangling-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let out = dir.join("link.bundle");
    std::os::unix::fs::symlink(dir.join("nothing-is-here"), &out).expect("a dangling link");
    assert!(!out.exists(), "the link's target is not there");

    let backend = exporting(vec![branch_entry("heads/main", false, false)]);
    assert_eq!(
        export_blocked_for(&backend, "heads/main", &out),
        DESTINATION_EXISTS
    );
    assert_eq!(backend.bundle_export_calls(), 0);
    assert!(
        std::fs::symlink_metadata(&out)
            .expect("still there")
            .is_symlink(),
        "the link is untouched"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// **The four rows stikk will not export**, each with its own reason, re-asked from freshly
/// read refs — the preview's copy of what the picker already declined.
#[test]
fn each_row_stikk_will_not_export_has_its_own_reason() {
    let dir = std::env::temp_dir().join(format!("stikk-export-rows-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let out = dir.join("out.bundle");

    let backend = exporting(vec![
        branch_entry("heads/main", false, false),
        branch_entry("heads/old", true, false),
        branch_entry("remotes/heads/theirs", false, true),
        branch_entry("tags/v1", false, false),
    ]);
    for (reff, want) in [
        ("tags/v1", TAGS_ARE_NOT_EXPORTED),
        ("remotes/heads/theirs", RECEIVED_IS_NOT_EXPORTED),
        ("heads/old", CLOSED_IS_NOT_EXPORTED),
        ("heads/never-sealed", UNPUBLISHED_IS_NOT_EXPORTED),
    ] {
        assert_eq!(export_blocked_for(&backend, reff, &out), want, "{reff}");
    }
    assert_eq!(backend.bundle_export_calls(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

/// stikk's suggestion is **outside the repository by construction**, so a reader who accepts it
/// never meets [`INSIDE_THE_REPOSITORY`].
#[test]
fn the_suggested_destination_is_outside_the_repository() {
    let repo = Path::new("/home/someone/work/my-repo");
    let suggested = suggested_destination(repo, "heads/feature/thing", &id("ab"));
    assert_eq!(
        suggested,
        Path::new("/home/someone/work/thing-ab0000000000.bundle"),
        "the parent, the last segment, and the short tip"
    );
    assert!(!suggested.starts_with(repo));
}

// ---------------------------------------------------------------------------------------------
// RFC 063 Handoff B: what prikk checked, and the import it would refuse (decisions 6 and 8).
// ---------------------------------------------------------------------------------------------

/// prikk's own `forged-publication-signature` refusal, shortened to the clause that matters —
/// its **id** is what stikk keys on, and this text is only ever shown (`measure063-0.58.txt`
/// §D1).
const FORGED: &str = "precondition not met: ref-state carries a MAINTAINER signature by adopted key shared that \
     does not verify: invalid signature -- a forgery, or another maintainer using the same key id";

fn refusal(id: &str) -> stikk_prikk::ImportRefusal {
    stikk_prikk::ImportRefusal {
        id: id.to_string(),
        message: FORGED.to_string(),
    }
}

fn outcome(key_id: &str, verdict: stikk_prikk::SealVerdict) -> stikk_prikk::SealOutcome {
    stikk_prikk::SealOutcome {
        key_id: key_id.to_string(),
        outcome: verdict,
    }
}

/// An importable bundle at `minor`, with prikk's own two new fields as given.
fn importing_at(
    minor: u32,
    outcomes: Option<Vec<stikk_prikk::SealOutcome>>,
    refusals: Option<Vec<stikk_prikk::ImportRefusal>>,
) -> NullBackend {
    let mut read = preview(
        Connectivity::NoLocalHistory,
        Some(BundleConflict::AppliesCleanly),
        None,
        edited_a(),
    );
    read.seal_outcomes = outcomes;
    read.import_refusals = refusals;
    NullBackend::supported()
        .with_version(0, minor, 0)
        .with_bundle_verify(verified_with_tip(&id("aa")))
        .with_bundle_preview(read)
        .with_bundle_import(import_lines())
        .with_refs(Vec::new())
        .with_trusted_keys(vec![stikk_prikk::TrustedKey {
            key_id: "z-maint".to_string(),
            public_key: "e".repeat(64),
        }])
}

/// **At 0.58 a non-empty refusal list withholds Import, and the refusals themselves come back**
/// — by id, with prikk's message to show. **Nothing is spawned.**
#[test]
fn at_0_58_prikks_own_refusal_withholds_the_import() {
    let backend = importing_at(
        58,
        Some(vec![outcome("z-maint", stikk_prikk::SealVerdict::Invalid)]),
        Some(vec![refusal("forged-publication-signature")]),
    );
    match previewed(&backend, 200) {
        ImportPreview::WouldRefuse(refusals) => {
            assert_eq!(refusals.len(), 1);
            assert_eq!(refusals[0].id, "forged-publication-signature");
            assert_eq!(refusals[0].message, FORGED, "prikk's words, verbatim");
        }
        other => panic!("expected prikk's refusal, got {other:?}"),
    }
    assert_eq!(backend.bundle_import_calls(), 0, "nothing was spawned");
}

/// **At 0.56–0.57 the `invalid` outcome withholds it**, with the security warning rather than a
/// refusal list: prikk refuses that import from 0.56 (F5), and its list is partial until 0.58.
#[test]
fn at_0_56_and_0_57_an_invalid_seal_withholds_the_import_with_the_warning() {
    for minor in [56, 57] {
        let backend = importing_at(
            minor,
            Some(vec![outcome("z-maint", stikk_prikk::SealVerdict::Invalid)]),
            None,
        );
        match previewed(&backend, 200) {
            ImportPreview::InvalidSeal(warning) => {
                assert_eq!(warning.heading, crate::INVALID_SEAL_HEADING, "0.{minor}");
                assert_eq!(warning.body, vec![crate::INVALID_SEAL]);
                let rows: Vec<(&str, &str)> = warning
                    .rows
                    .iter()
                    .map(|row| (row.label, row.value.as_str()))
                    .collect();
                assert_eq!(
                    rows,
                    vec![
                        ("key id", "z-maint"),
                        ("trusted here", "e".repeat(64).as_str())
                    ],
                    "0.{minor}: the id, and the key the signature was checked against"
                );
            }
            other => panic!("0.{minor}: expected the warning, got {other:?}"),
        }
        assert_eq!(
            backend.bundle_import_calls(),
            0,
            "0.{minor}: nothing spawned"
        );
    }
}

/// **`unverifiable` withholds nothing** (§7 trap 2): this repository trusts no key under that
/// id, which is the ordinary case for a first exchange — and an **empty** refusal list at 0.58
/// is prikk's own answer, not a missing one.
#[test]
fn an_unverifiable_seal_and_an_empty_refusal_list_still_import() {
    for minor in [56, 57, 58] {
        let refusals = (minor >= 58).then(Vec::new);
        let backend = importing_at(
            minor,
            Some(vec![
                outcome("z-maint", stikk_prikk::SealVerdict::Unverifiable),
                outcome("other", stikk_prikk::SealVerdict::Sound),
            ]),
            refusals,
        );
        assert!(
            matches!(previewed(&backend, 200), ImportPreview::Ready { .. }),
            "0.{minor}: an unverifiable seal is not a warning"
        );
    }
}

/// **Below 0.56 nothing changes**: neither field is read, so the import path is the one RFC 050
/// shipped.
#[test]
fn below_0_56_the_import_path_is_unchanged() {
    let backend = importing_at(55, None, None);
    assert!(matches!(
        previewed(&backend, 200),
        ImportPreview::Ready { .. }
    ));
}

/// **The bundle card's rows are prikk's answer from 0.56**, keyed by outcome, and
/// `NAMES_BUT_DOES_NOT_PROVE` is not owed beneath them — it exists because a name is not a
/// proof, and prikk checked the signature itself.
#[test]
fn the_card_reads_each_seal_as_prikk_checked_it_from_0_56() {
    use crate::trust::SealerTrust;
    let backend = importing_at(
        56,
        Some(vec![
            outcome("z-maint", stikk_prikk::SealVerdict::Sound),
            outcome("stranger", stikk_prikk::SealVerdict::Unverifiable),
        ]),
        None,
    );
    let read = crate::open_bundle(&backend, Path::new("/repo"), Path::new("/tmp/v1.bundle"))
        .expect("the bundle reads");
    let BundleOpened::Read(view) = read else {
        panic!("expected a read bundle");
    };
    let rows: Vec<(&str, SealerTrust)> = view
        .sealers
        .judged()
        .iter()
        .map(|sealer| (sealer.key_id.as_str(), sealer.trust))
        .collect();
    assert_eq!(
        rows,
        vec![
            (
                "z-maint",
                SealerTrust::Checked(stikk_prikk::SealVerdict::Sound)
            ),
            (
                "stranger",
                SealerTrust::Checked(stikk_prikk::SealVerdict::Unverifiable)
            ),
        ],
        "prikk's ids, in prikk's order, with prikk's verdicts"
    );
    assert!(
        !view.sealers.names_an_adopted_key(),
        "no name is being taken for a proof here"
    );
    assert_eq!(view.warning, None, "no invalid seal, no warning");
}

/// **`NAMES_BUT_DOES_NOT_PROVE` stays below 0.56**, where stikk's name match is all there is.
#[test]
fn below_0_56_the_card_still_owes_names_but_does_not_prove() {
    let backend = importing_at(55, None, None);
    let read = crate::open_bundle(&backend, Path::new("/repo"), Path::new("/tmp/v1.bundle"))
        .expect("the bundle reads");
    let BundleOpened::Read(view) = read else {
        panic!("expected a read bundle");
    };
    assert!(
        view.sealers.names_an_adopted_key(),
        "the id names an adopted key, and that is not a proof"
    );
    assert_eq!(view.warning, None);
}

/// **An `invalid` row puts the warning on the view**, for the card to open over itself.
#[test]
fn an_invalid_seal_puts_the_warning_on_the_view() {
    let backend = importing_at(
        58,
        Some(vec![outcome("z-maint", stikk_prikk::SealVerdict::Invalid)]),
        Some(vec![refusal("forged-publication-signature")]),
    );
    let read = crate::open_bundle(&backend, Path::new("/repo"), Path::new("/tmp/v1.bundle"))
        .expect("the bundle reads");
    let BundleOpened::Read(view) = read else {
        panic!("expected a read bundle");
    };
    let warning = view.warning.expect("the security event");
    assert_eq!(warning.heading, crate::INVALID_SEAL_HEADING);
    assert_eq!(
        warning.detail, None,
        "prikk's message is on the card's list"
    );
    assert_eq!(
        view.preview
            .import_refusals
            .as_deref()
            .map(<[stikk_prikk::ImportRefusal]>::len),
        Some(1),
        "and the card still lists what prikk would refuse"
    );
}

// ---------------------------------------------------------------------------------------------
// Review v1 D1: a trust list stikk cannot read must not cost prikk's own verdicts.
// ---------------------------------------------------------------------------------------------

/// As [`importing_at`], with `trust maintainer list` **refused** — prikk answered the preview,
/// and stikk cannot list the keys that answer was made against.
fn importing_at_with_the_list_refused(
    minor: u32,
    outcomes: Option<Vec<stikk_prikk::SealOutcome>>,
    refusals: Option<Vec<stikk_prikk::ImportRefusal>>,
) -> NullBackend {
    importing_at(minor, outcomes, refusals)
        .with_trusted_keys_refusal("error: precondition not met: trust policy is unreadable")
}

/// **At 0.56–0.57 the import is still withheld when the list cannot be read** (review v1 D1).
///
/// This is the case that failed open: every row became `Unknown`, the warning was lost, and
/// stikk armed a confirmation for a bundle prikk had itself called `invalid`.
#[test]
fn a_refused_trust_list_does_not_offer_an_import_prikk_called_invalid() {
    for minor in [56, 57] {
        let backend = importing_at_with_the_list_refused(
            minor,
            Some(vec![outcome("z-maint", stikk_prikk::SealVerdict::Invalid)]),
            None,
        );
        match previewed(&backend, 200) {
            ImportPreview::InvalidSeal(warning) => {
                assert_eq!(warning.heading, crate::INVALID_SEAL_HEADING, "0.{minor}");
                // **The row is left out and its absence is said**, rather than a key stikk never
                // read being drawn as one.
                assert_eq!(
                    warning.rows.len(),
                    1,
                    "0.{minor}: the id only: {:?}",
                    warning.rows
                );
                assert!(
                    warning.body.contains(&crate::KEY_TRUSTED_HERE_UNREAD),
                    "0.{minor}: and the card says why: {:?}",
                    warning.body
                );
            }
            other => panic!("0.{minor}: expected the warning, got {other:?}"),
        }
        assert_eq!(
            backend.bundle_import_calls(),
            0,
            "0.{minor}: nothing spawned"
        );
    }
}

/// **At 0.58 the same warning, and prikk's refusal list still withholds the import.**
#[test]
fn a_refused_trust_list_keeps_the_warning_and_prikks_refusals_at_0_58() {
    let backend = importing_at_with_the_list_refused(
        58,
        Some(vec![outcome("z-maint", stikk_prikk::SealVerdict::Invalid)]),
        Some(vec![refusal("forged-publication-signature")]),
    );
    assert!(
        matches!(previewed(&backend, 200), ImportPreview::WouldRefuse(_)),
        "prikk's own list is what withholds at 0.58"
    );
    let read = crate::open_bundle(&backend, Path::new("/repo"), Path::new("/tmp/v1.bundle"))
        .expect("the bundle reads");
    let BundleOpened::Read(view) = read else {
        panic!("expected a read bundle");
    };
    let warning = view.warning.expect("the card still opens the warning");
    assert_eq!(warning.heading, crate::INVALID_SEAL_HEADING);
    assert_eq!(warning.rows.len(), 1, "{:?}", warning.rows);
    assert!(warning.body.contains(&crate::KEY_TRUSTED_HERE_UNREAD));
    assert_eq!(backend.bundle_import_calls(), 0);
}

/// **Every row still carries prikk's own sentence, not `Unknown`**, when the list is refused —
/// the verdict is prikk's, and the list only supplied the key shown beside it.
#[test]
fn a_refused_trust_list_still_shows_each_verdict_prikk_gave() {
    use crate::trust::SealerTrust;
    let backend = importing_at_with_the_list_refused(
        58,
        Some(vec![
            outcome("z-maint", stikk_prikk::SealVerdict::Sound),
            outcome("stranger", stikk_prikk::SealVerdict::Unverifiable),
        ]),
        Some(Vec::new()),
    );
    let read = crate::open_bundle(&backend, Path::new("/repo"), Path::new("/tmp/v1.bundle"))
        .expect("the bundle reads");
    let BundleOpened::Read(view) = read else {
        panic!("expected a read bundle");
    };
    let rows: Vec<(&str, SealerTrust, bool)> = view
        .sealers
        .judged()
        .iter()
        .map(|sealer| {
            (
                sealer.key_id.as_str(),
                sealer.trust,
                sealer.trusted_here.is_some(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            (
                "z-maint",
                SealerTrust::Checked(stikk_prikk::SealVerdict::Sound),
                false
            ),
            (
                "stranger",
                SealerTrust::Checked(stikk_prikk::SealVerdict::Unverifiable),
                false
            ),
        ],
        "prikk's verdicts, with no key to show beside them"
    );
    assert_eq!(view.warning, None, "nothing here is the security event");
    // **And an import is still offered**, because `unverifiable` withholds nothing and prikk's
    // refusal list is empty.
    assert!(matches!(
        previewed(&backend, 200),
        ImportPreview::Ready { .. }
    ));
}

/// **Below 0.56 nothing changed**: a refused list is `Unknown` for every id, as RFC 054 built it.
#[test]
fn below_0_56_a_refused_trust_list_is_still_unknown_for_every_id() {
    use crate::trust::SealerTrust;
    let backend = importing_at_with_the_list_refused(55, None, None);
    let read = crate::open_bundle(&backend, Path::new("/repo"), Path::new("/tmp/v1.bundle"))
        .expect("the bundle reads");
    let BundleOpened::Read(view) = read else {
        panic!("expected a read bundle");
    };
    assert!(
        view.sealers
            .judged()
            .iter()
            .all(|sealer| sealer.trust == SealerTrust::Unknown),
        "{:?}",
        view.sealers
    );
}
