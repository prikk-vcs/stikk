//! Tests for the trust read and the bundle cards' sealer row (RFC 054).
//!
//! The keys below are prikk 0.54.1's own, from
//! `.git-exclude/reports/054-trust-read/measure054-0.54.txt`: `setup`'s own maintainer key and a
//! collaborator's, adopted after it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use stikk_prikk::{NullBackend, TrustedKey};

use super::*;

const MINE_ID: &str = "ed25519-31f369d3abb7f64c";
const MINE_KEY: &str = "31f369d3abb7f64cd2cf70be31158181e361f25444d8de122cdff59389e7e946";
const THEIRS_ID: &str = "ed25519-9c0a44b1d2e3f501";
const THEIRS_KEY: &str = "9c0a44b1d2e3f5012b7c0a4e55d2117ab3340ef6627c118e2b4a9d0cf3e21847";

fn key(id: &str, public: &str) -> TrustedKey {
    TrustedKey {
        key_id: id.to_string(),
        public_key: public.to_string(),
    }
}

/// `key status`'s answer for the MAINTAINER role, with `id` and `public` as prikk reports them.
fn readiness_for(id: &str, public: &str) -> stikk_prikk::ReadinessReport {
    stikk_prikk::ReadinessReport {
        readiness: stikk_model::Readiness::none(),
        author: stikk_prikk::RoleDetail::default(),
        maintainer: stikk_prikk::RoleDetail {
            key_id: Some(id.to_string()),
            public_key: Some(public.to_string()),
            ..stikk_prikk::RoleDetail::default()
        },
    }
}

/// A backend at the ceiling that lists `keys` and reports `MINE_*` as the maintainer role.
fn trusting(keys: Vec<TrustedKey>) -> NullBackend {
    NullBackend::supported()
        .with_version(0, 55, 0)
        .with_trusted_keys(keys)
        .with_readiness(readiness_for(MINE_ID, MINE_KEY))
}

fn rows(backend: &NullBackend) -> Vec<TrustRow> {
    match read(backend, Path::new("/repo")).expect("the read answers") {
        TrustView::Listed { rows, .. } => rows,
        other => panic!("expected a list, got {other:?}"),
    }
}

/// **Yours needs the id *and* the public key** (RFC 054 §7 trap 3).
#[test]
fn yours_is_marked_only_when_both_halves_match() {
    let listed = rows(&trusting(vec![
        key(MINE_ID, MINE_KEY),
        key(THEIRS_ID, THEIRS_KEY),
    ]));
    assert_eq!(listed.len(), 2);
    assert!(listed[0].yours, "setup's own key");
    assert!(!listed[1].yours, "the collaborator's");
    // **Order is prikk's**, which is adoption order.
    assert_eq!(listed[0].key_id, MINE_ID);
    assert_eq!(listed[1].key_id, THEIRS_ID);
}

/// **An id is a label anyone can choose.** A row carrying your id under someone else's public
/// key is not yours, and marking it so would be the one mistake this view exists to prevent.
#[test]
fn your_id_under_another_key_is_not_yours() {
    let listed = rows(&trusting(vec![key(MINE_ID, THEIRS_KEY)]));
    assert!(
        !listed[0].yours,
        "the id matches and the key does not — that is someone else"
    );
    // And the mirror: your key under another id is not yours either.
    let listed = rows(&trusting(vec![key(THEIRS_ID, MINE_KEY)]));
    assert!(!listed[0].yours);
}

/// **A failed `key status` costs the marking and nothing else** (RFC 035 A3's rule).
#[test]
fn an_unreadable_key_status_leaves_the_list_whole_and_unmarked() {
    let backend = NullBackend::supported()
        .with_version(0, 55, 0)
        .with_trusted_keys(vec![key(MINE_ID, MINE_KEY)])
        .with_readiness_refusal("prikk could not read the key directory");
    match read(&backend, Path::new("/repo")).expect("the list still answers") {
        TrustView::Listed {
            rows,
            own_key_unknown,
        } => {
            assert_eq!(
                rows.len(),
                1,
                "the list is prikk's and does not depend on it"
            );
            assert!(!rows[0].yours, "nothing is marked");
            assert_eq!(own_key_unknown.as_deref(), Some(OWN_KEY_UNKNOWN));
        }
        other => panic!("expected a list, got {other:?}"),
    }
}

/// An empty list is an answer: a repository made with `init` alone.
#[test]
fn a_repository_that_adopted_nothing_lists_nothing() {
    match read(&trusting(Vec::new()), Path::new("/repo")).expect("an empty list answers") {
        TrustView::Listed { rows, .. } => assert!(rows.is_empty()),
        other => panic!("expected a list, got {other:?}"),
    }
}

/// **Below the band, stikk's own sentence**, and the band is named.
#[test]
fn below_the_band_the_view_says_so() {
    let backend = NullBackend::supported().with_version(0, 53, 0);
    match read(&backend, Path::new("/repo")).expect("below the band is an answer") {
        TrustView::Unavailable(reason) => {
            assert_eq!(reason, trust_unavailable(53));
            assert!(reason.contains("0.54"), "{reason}");
        }
        other => panic!("expected unavailable, got {other:?}"),
    }
}

/// **A refused list is prikk's words, and nothing else is shown.**
#[test]
fn a_refused_list_carries_prikks_message_verbatim() {
    let backend = NullBackend::supported()
        .with_version(0, 55, 0)
        .with_trusted_keys_refusal("integrity error: the trust store is unreadable");
    match read(&backend, Path::new("/repo")).expect("a refusal is an answer, not a panic") {
        TrustView::Failed(message) => assert!(
            message.contains("the trust store is unreadable"),
            "prikk's words: {message}"
        ),
        other => panic!("expected a failure, got {other:?}"),
    }
}

// ---------------------------------------------------------------- the sealer row

fn judged(backend: &NullBackend, ids: &[&str]) -> Vec<Sealer> {
    let ids: Vec<String> = ids.iter().map(ToString::to_string).collect();
    match sealers(backend, Path::new("/repo"), &ids) {
        SealerTrustRead::Judged(sealers) => sealers,
        other => panic!("expected judged sealers, got {other:?}"),
    }
}

/// **The three answers**, from one read (RFC 054 decision 3, as amended) — and each says what
/// the id *names*, because that is all a bundle records.
#[test]
fn a_sealer_names_your_key_a_trusted_key_or_nothing_this_repository_adopted() {
    let backend = trusting(vec![key(MINE_ID, MINE_KEY), key(THEIRS_ID, THEIRS_KEY)]);
    let out = judged(&backend, &[MINE_ID, THEIRS_ID, "ed25519-0000000000000000"]);
    assert_eq!(out[0].trust, SealerTrust::NamesYourKey);
    assert_eq!(out[1].trust, SealerTrust::NamesATrustedKey);
    assert_eq!(out[2].trust, SealerTrust::NotTrusted);
    // prikk's ids, in prikk's order, unrewritten.
    assert_eq!(out[0].key_id, MINE_ID);
    assert_eq!(out[2].key_id, "ed25519-0000000000000000");
    for sealer in &out {
        assert!(!sealer.trust.sentence().is_empty());
    }
}

/// **A failed read says so for every id**, and says nothing else.
#[test]
fn a_failed_trust_read_leaves_every_sealer_unknown() {
    let backend = NullBackend::supported()
        .with_version(0, 55, 0)
        .with_trusted_keys_refusal("integrity error: the trust store is unreadable");
    let out = judged(&backend, &[MINE_ID, THEIRS_ID]);
    assert!(out.iter().all(|s| s.trust == SealerTrust::Unknown));
    assert_eq!(
        out[0].trust.sentence(),
        "whether this repository trusts it could not be read"
    );
}

/// **Below the band the card says nothing extra at all** (decision 4) — not *could not be read*,
/// which would report a limit of stikk's own as if it had tried and failed.
#[test]
fn below_the_band_the_sealer_row_is_0_16_0s() {
    let backend = NullBackend::supported().with_version(0, 53, 0);
    let ids = vec![MINE_ID.to_string()];
    assert_eq!(
        sealers(&backend, Path::new("/repo"), &ids),
        SealerTrustRead::Unavailable
    );
    assert!(
        SealerTrustRead::Unavailable.judged().is_empty(),
        "nothing is added beside prikk's id and note"
    );
}

/// **No guard**: an untrusted sealer does not stop anything (decision 3). This is the shape of
/// that promise — `sealers` returns an answer and never an error, and nothing in it is a block.
#[test]
fn an_untrusted_sealer_is_an_answer_and_not_a_refusal() {
    let backend = trusting(Vec::new());
    let out = judged(&backend, &[THEIRS_ID]);
    assert_eq!(out[0].trust, SealerTrust::NotTrusted);
    assert_eq!(
        out[0].trust.sentence(),
        "not trusted by this repository",
        "a statement, not a reason to decline"
    );
}

/// **Which answers oblige the card to say what a name is worth.** The sentence follows the
/// rows wherever one of them names an adopted key, and not where none does: an id nobody has
/// adopted makes no claim for it to qualify.
#[test]
fn the_sentence_follows_the_rows_that_name_an_adopted_key() {
    let backend = trusting(vec![key(MINE_ID, MINE_KEY), key(THEIRS_ID, THEIRS_KEY)]);
    for (tag, ids, want) in [
        ("yours", vec![MINE_ID], true),
        ("theirs", vec![THEIRS_ID], true),
        (
            "one of each",
            vec!["ed25519-0000000000000000", THEIRS_ID],
            true,
        ),
        ("none adopted", vec!["ed25519-0000000000000000"], false),
    ] {
        let ids: Vec<String> = ids.iter().map(ToString::to_string).collect();
        assert_eq!(
            sealers(&backend, Path::new("/repo"), &ids).names_an_adopted_key(),
            want,
            "{tag}"
        );
    }
    // Below the band and after a failed read there are no claims either.
    assert!(!SealerTrustRead::Unavailable.names_an_adopted_key());
    let refused = NullBackend::supported()
        .with_version(0, 55, 0)
        .with_trusted_keys_refusal("integrity error: the trust store is unreadable");
    let ids = vec![MINE_ID.to_string()];
    assert!(!sealers(&refused, Path::new("/repo"), &ids).names_an_adopted_key());
    // The sentence itself names no repository text, and says where the check happens.
    assert!(NAMES_BUT_DOES_NOT_PROVE.contains("Verify"));
}

/// **No key material anywhere in what this module produces** (`NFR-S03`, §7 trap 1).
#[test]
fn nothing_here_carries_a_seed_or_a_path() {
    let backend = trusting(vec![key(MINE_ID, MINE_KEY)]);
    let view = read(&backend, Path::new("/repo")).expect("the read answers");
    let rendered = format!("{view:?}");
    for forbidden in ["seed", "Seed", "SEED", ".config", "/home/", "path"] {
        assert!(
            !rendered.contains(forbidden),
            "the view must carry no {forbidden}: {rendered}"
        );
    }
}

// ============================================================ RFC 056: adopt and remove

/// A third key, neither yours nor the collaborator's — the one an adopt brings in.
const NEW_ID: &str = "ed25519-6762b6359a650c0a";
const NEW_KEY: &str = "6762b6359a650c0a5acaa723a5fc2ad78b354fcac296495b9cbf46c0d32a3f7f";

/// A backend that lists `keys`, reports `MINE_*` as the maintainer role, and answers both
/// writes with prikk's own lines.
fn writing(keys: Vec<TrustedKey>) -> NullBackend {
    trusting(keys).with_trust_write(vec![
        "trusted maintainer key: ed25519-6762b6359a650c0a".to_string(),
        "adopted maintainer keys: 2".to_string(),
    ])
}

/// An operator who may write. Every guard below must decline on its own terms, not on this.
fn operator() -> stikk_model::Readiness {
    stikk_model::Readiness {
        read_only: false,
        ..stikk_model::Readiness::none()
    }
}

fn adopt(backend: &NullBackend, id: &str, key: &str) -> AdoptPreview {
    adopt_preview(backend, Path::new("/repo"), operator(), id, key).expect("the preview answers")
}

fn remove(backend: &NullBackend, id: &str) -> RemovePreview {
    remove_preview(backend, Path::new("/repo"), operator(), id).expect("the preview answers")
}

fn armed_adopt(backend: &NullBackend, id: &str, key: &str) -> PreviewToken {
    match adopt(backend, id, key) {
        AdoptPreview::Ready { token } => *token,
        other => panic!("expected an armed adopt, got {other:?}"),
    }
}

fn armed_remove(backend: &NullBackend, id: &str) -> PreviewToken {
    match remove(backend, id) {
        RemovePreview::Ready { token } => *token,
        other => panic!("expected an armed removal, got {other:?}"),
    }
}

// ---------------------------------------------------------------- what stikk checks itself

/// **The input checks are stikk's own sentences**, so a prompt can stay open (decision 2) — and
/// **nothing is spawned** for any of them.
#[test]
fn a_malformed_key_or_id_is_declined_before_prikk_is_asked() {
    let backend = writing(vec![key(MINE_ID, MINE_KEY)]);
    for (what, id, public, want) in [
        ("short", NEW_ID, &NEW_KEY[..63], PUBLIC_KEY_SHAPE),
        ("long", NEW_ID, &format!("{NEW_KEY}a")[..], PUBLIC_KEY_SHAPE),
        ("not hex", NEW_ID, &"z".repeat(64)[..], PUBLIC_KEY_SHAPE),
        (
            "upper case",
            NEW_ID,
            &NEW_KEY.to_uppercase()[..],
            PUBLIC_KEY_SHAPE,
        ),
        ("empty id", "", NEW_KEY, KEY_ID_SHAPE),
        ("a space in the id", "ed25519 6762", NEW_KEY, KEY_ID_SHAPE),
        (
            "a control character",
            "ed25519-\u{7}x",
            NEW_KEY,
            KEY_ID_SHAPE,
        ),
    ] {
        match adopt(&backend, id, public) {
            AdoptPreview::Blocked(said) => assert_eq!(said, want, "{what}"),
            other => panic!("{what}: expected a block, got {other:?}"),
        }
    }
    assert_eq!(backend.trust_add_calls(), 0);
    assert_eq!(backend.trust_remove_calls(), 0);
}

/// **A pasted key carries whatever the terminal put around it**, and that is not a malformed
/// key: the check trims first (decision 2).
#[test]
fn a_pasted_key_is_trimmed_rather_than_refused() {
    assert_eq!(
        check_public_key(&format!("  {NEW_KEY}\n")).expect("trimmed"),
        NEW_KEY
    );
    assert_eq!(
        check_key_id("  ed25519-abc  ").expect("trimmed"),
        "ed25519-abc"
    );
}

/// **prikk's own default id, which stikk computes** (F3): `ed25519-` and the first 16 characters
/// of the public key — checked against the measured pairs.
#[test]
fn the_default_id_is_prikks_own_rule() {
    for (public, id) in [
        (MINE_KEY, MINE_ID),
        (THEIRS_KEY, THEIRS_ID),
        (NEW_KEY, NEW_ID),
    ] {
        assert_eq!(default_key_id(public), id, "{public}");
    }
}

// ---------------------------------------------------------------- the adopt guards

/// **An id adopted with this very key** declines, and spawns no `add` (decision 2).
#[test]
fn adopting_a_key_this_repository_already_trusts_is_declined_without_writing() {
    let backend = writing(vec![key(MINE_ID, MINE_KEY), key(THEIRS_ID, THEIRS_KEY)]);
    match adopt(&backend, THEIRS_ID, THEIRS_KEY) {
        AdoptPreview::Blocked(said) => assert_eq!(said, ALREADY_TRUSTED),
        other => panic!("expected a block, got {other:?}"),
    }
    assert_eq!(backend.trust_add_calls(), 0);
}

/// **An id adopted with a *different* key is the security event** (decision 4), carrying both
/// public keys so a reader can see they differ — and it is **not a confirmation**: no token
/// comes back, and no `add` was spawned.
#[test]
fn a_different_key_under_a_known_id_is_the_security_event_with_both_keys() {
    let backend = writing(vec![key(MINE_ID, MINE_KEY), key(THEIRS_ID, THEIRS_KEY)]);
    match adopt(&backend, THEIRS_ID, NEW_KEY) {
        AdoptPreview::TwoKeys(conflict) => {
            assert_eq!(conflict.key_id, THEIRS_ID);
            assert_eq!(conflict.adopted, THEIRS_KEY, "what this repository holds");
            assert_eq!(conflict.given, NEW_KEY, "what was offered for the same id");
            assert_ne!(
                conflict.adopted, conflict.given,
                "the whole point of the card"
            );
        }
        other => panic!("expected the security event, got {other:?}"),
    }
    assert_eq!(backend.trust_add_calls(), 0);
    // stikk's own words, and no id inside them: the card draws the id as its own row.
    assert!(TWO_KEYS_ONE_ID.contains("Two keys cannot share an id"));
    assert!(!TWO_KEYS_ONE_ID.contains("ed25519-"));
}

/// **Below the band, nothing is offered and nothing is spawned** (decision 1).
#[test]
fn below_the_band_both_previews_decline_and_spawn_nothing() {
    let backend = NullBackend::supported().with_version(0, 53, 0);
    let minor_said = trust_unavailable(53);
    match adopt(&backend, NEW_ID, NEW_KEY) {
        AdoptPreview::Blocked(said) => assert_eq!(said, minor_said),
        other => panic!("expected a block, got {other:?}"),
    }
    match remove(&backend, MINE_ID) {
        RemovePreview::Blocked(said) => assert_eq!(said, minor_said),
        other => panic!("expected a block, got {other:?}"),
    }
    assert_eq!(backend.trust_add_calls(), 0);
    assert_eq!(backend.trust_remove_calls(), 0);
}

/// **Read-only declines both**, in the gate's own words and before anything is read.
#[test]
fn read_only_declines_both_previews() {
    let backend = writing(vec![key(MINE_ID, MINE_KEY), key(THEIRS_ID, THEIRS_KEY)]);
    let read_only = stikk_model::Readiness {
        read_only: true,
        ..stikk_model::Readiness::none()
    };
    let said = match adopt_preview(&backend, Path::new("/repo"), read_only, NEW_ID, NEW_KEY)
        .expect("a decline is an answer")
    {
        AdoptPreview::Blocked(said) => said,
        other => panic!("expected a block, got {other:?}"),
    };
    assert!(said.contains("read-only"), "{said}");
    match remove_preview(&backend, Path::new("/repo"), read_only, THEIRS_ID)
        .expect("a decline is an answer")
    {
        RemovePreview::Blocked(said) => assert!(said.contains("read-only"), "{said}"),
        other => panic!("expected a block, got {other:?}"),
    }
    assert_eq!(backend.trust_add_calls(), 0);
    assert_eq!(backend.trust_remove_calls(), 0);
}

// ---------------------------------------------------------------- the adopt card

/// **The card says what adopting means, where the key came from, and what follows** — with the
/// typed evidence being the key id (§7 trap 1).
#[test]
fn the_adopt_card_carries_both_values_the_two_sentences_and_the_consequence() {
    let backend = writing(vec![key(MINE_ID, MINE_KEY)]);
    let token = armed_adopt(&backend, NEW_ID, NEW_KEY);
    let summary = token.summary();
    assert_eq!(summary.operation, ADOPT_OPERATION);
    assert_eq!(summary.capability, None, "neither write consumes a key");
    assert_eq!(summary.target_name.as_deref(), Some(NEW_ID));
    let rows: Vec<(Option<&str>, &str)> = summary
        .target_ids
        .iter()
        .map(|row| (row.role, row.id.as_str()))
        .collect();
    assert_eq!(rows, vec![(Some("key id"), NEW_ID)]);
    assert_eq!(summary.consequence, ADOPT_CONSEQUENCE);
    // **The public key is an aside's body**, because that is what wraps: measured at 80
    // columns, a target row drawn unwrapped clipped a 64-character key mid-value.
    assert_eq!(summary.asides[0].before, "the public key");
    assert_eq!(
        summary.asides[0].body,
        vec![
            NEW_KEY.to_string(),
            WHAT_TRUST_MEANS.to_string(),
            WHERE_TO_GET_THE_KEY.to_string()
        ]
    );
    // The id is this key's own default, so no notice.
    assert!(summary.notices.is_empty(), "{:?}", summary.notices);
    // And the consequence names nothing (RFC 044 C1).
    assert!(!summary.consequence.contains("ed25519-"));
}

/// **The notice appears exactly when the id is not the key's default** (F3).
#[test]
fn the_default_id_notice_appears_only_for_an_id_the_key_did_not_derive() {
    let backend = writing(vec![key(MINE_ID, MINE_KEY)]);
    let chosen = "ed25519-2222222222222222";
    let token = armed_adopt(&backend, chosen, NEW_KEY);
    let summary = token.summary();
    assert_eq!(summary.notices.len(), 1, "{:?}", summary.notices);
    assert!(summary.notices[0].contains(NEW_ID), "it names the default");
    assert!(
        summary.notices[0].contains("ask them"),
        "{:?}",
        summary.notices
    );
    // And the default-id case has none, which the test above also pins.
    let plain_token = armed_adopt(&backend, NEW_ID, NEW_KEY);
    let plain = plain_token.summary();
    assert!(plain.notices.is_empty());
}

// ---------------------------------------------------------------- the remove guards and card

/// **An id that is not adopted** declines, and spawns no `remove` (decision 3).
#[test]
fn removing_a_key_that_is_not_adopted_is_declined_without_writing() {
    let backend = writing(vec![key(MINE_ID, MINE_KEY), key(THEIRS_ID, THEIRS_KEY)]);
    match remove(&backend, NEW_ID) {
        RemovePreview::Blocked(said) => assert_eq!(said, NO_LONGER_TRUSTED),
        other => panic!("expected a block, got {other:?}"),
    }
    assert_eq!(backend.trust_remove_calls(), 0);
}

/// **A list of one** declines with what to do instead — prikk refuses it (F4), and stikk says so
/// before asking.
#[test]
fn removing_the_only_key_is_declined_without_writing() {
    let backend = writing(vec![key(MINE_ID, MINE_KEY)]);
    match remove(&backend, MINE_ID) {
        RemovePreview::Blocked(said) => assert_eq!(said, LAST_KEY),
        other => panic!("expected a block, got {other:?}"),
    }
    assert_eq!(backend.trust_remove_calls(), 0);
}

/// **The Verify notice is always there, and the "yours" notice only for your own key** — which
/// needs id **and** public key to match (RFC 054's rule).
#[test]
fn the_remove_card_warns_about_verify_and_marks_your_own_key() {
    let backend = writing(vec![key(MINE_ID, MINE_KEY), key(THEIRS_ID, THEIRS_KEY)]);
    let theirs_token = armed_remove(&backend, THEIRS_ID);
    let theirs = theirs_token.summary();
    assert_eq!(theirs.notices, vec![REMOVE_MAY_FAIL_VERIFY.to_string()]);
    assert_eq!(theirs.consequence, REMOVE_CONSEQUENCE);
    assert_eq!(theirs.target_name.as_deref(), Some(THEIRS_ID));

    let mine_token = armed_remove(&backend, MINE_ID);
    let mine = mine_token.summary();
    assert_eq!(
        mine.notices,
        vec![
            REMOVE_MAY_FAIL_VERIFY.to_string(),
            REMOVE_YOUR_OWN.to_string()
        ]
    );
}

/// **A failed `key status` costs only the "yours" notice** (RFC 035 A3's rule): the removal
/// itself does not depend on knowing whose key this is.
#[test]
fn a_failed_key_status_costs_only_the_your_own_key_notice() {
    let backend = trusting(vec![key(MINE_ID, MINE_KEY), key(THEIRS_ID, THEIRS_KEY)])
        .with_readiness_refusal("integrity error: the key directory is unreadable");
    let token = armed_remove(&backend, MINE_ID);
    let summary = token.summary();
    assert_eq!(
        summary.notices,
        vec![REMOVE_MAY_FAIL_VERIFY.to_string()],
        "the card still warns about Verify"
    );
}

// ---------------------------------------------------------------- the typed evidence

/// **Typed evidence is the key id, matched exactly** (§7 trap 1): an explicit yes, a near miss
/// and empty input are all refused, and **no write is spawned** by any of them.
#[test]
fn only_the_typed_key_id_confirms_either_write() {
    for near in [
        Evidence::ExplicitYes,
        Evidence::TypedName(String::new()),
        Evidence::TypedName("ed25519-6762b6359a650c0".to_string()),
        Evidence::TypedName(format!("{NEW_ID} ")),
        Evidence::TypedName(MINE_ID.to_string()),
    ] {
        let backend = writing(vec![key(MINE_ID, MINE_KEY), key(THEIRS_ID, THEIRS_KEY)]);
        let token = armed_adopt(&backend, NEW_ID, NEW_KEY);
        let err = adopt_confirm_and_execute(
            &backend,
            Path::new("/repo"),
            token,
            operator(),
            near.clone(),
            NEW_ID,
            NEW_KEY,
        )
        .expect_err("not the typed id");
        assert_eq!(err.class(), "declined", "{near:?}: {err}");
        assert_eq!(backend.trust_add_calls(), 0, "{near:?}");

        let token = armed_remove(&backend, THEIRS_ID);
        let err = remove_confirm_and_execute(
            &backend,
            Path::new("/repo"),
            token,
            operator(),
            near.clone(),
            THEIRS_ID,
        )
        .expect_err("not the typed id");
        assert_eq!(err.class(), "declined", "{near:?}: {err}");
        assert_eq!(backend.trust_remove_calls(), 0, "{near:?}");
    }
}

/// **A stale change token refuses**, as every other confirmation does.
#[test]
fn a_stale_preview_refuses_and_writes_nothing() {
    let backend = writing(vec![key(MINE_ID, MINE_KEY), key(THEIRS_ID, THEIRS_KEY)]);
    let token = armed_adopt(&backend, NEW_ID, NEW_KEY);
    let moved = backend
        .clone()
        .with_change_token(stikk_model::ChangeToken::compose(
            [("heads/main", "1".repeat(64).as_str())],
            0,
            None,
            &stikk_model::CurrentBranch::NotReported,
        ));
    let err = adopt_confirm_and_execute(
        &moved,
        Path::new("/repo"),
        token,
        operator(),
        Evidence::TypedName(NEW_ID.to_string()),
        NEW_ID,
        NEW_KEY,
    )
    .expect_err("the repository moved under the preview");
    assert_eq!(err.class(), "stale");
    assert_eq!(moved.trust_add_calls(), 0);
}

// ---------------------------------------------------------------- what a write leaves

/// **prikk's lines verbatim, then the list as it is now** (decision 1) — and a removal adds
/// stikk's pointer at Verify, which an adopt does not.
#[test]
fn a_write_shows_prikks_lines_and_the_list_it_re_reads() {
    let backend = writing(vec![key(MINE_ID, MINE_KEY)]);
    let token = armed_adopt(&backend, NEW_ID, NEW_KEY);
    let outcome = adopt_confirm_and_execute(
        &backend,
        Path::new("/repo"),
        token,
        operator(),
        Evidence::TypedName(NEW_ID.to_string()),
        NEW_ID,
        NEW_KEY,
    )
    .expect("the adopt runs");
    assert_eq!(outcome.operation, ADOPT_OPERATION);
    assert_eq!(
        outcome.result.printed,
        vec![
            "trusted maintainer key: ed25519-6762b6359a650c0a".to_string(),
            "adopted maintainer keys: 2".to_string(),
        ],
        "prikk's own words, parsed by nothing"
    );
    assert!(
        matches!(outcome.result.view, TrustView::Listed { .. }),
        "the list is re-read: {:?}",
        outcome.result.view
    );
    assert_eq!(
        outcome.result.see_verify, None,
        "an adopt un-verifies nothing"
    );
    assert_eq!(backend.trust_add_calls(), 1);

    // **A removal, on a list with something left after it.** The NullBackend's list does not
    // change under the write, so the two halves need their own backends.
    let backend = writing(vec![key(MINE_ID, MINE_KEY), key(THEIRS_ID, THEIRS_KEY)]);
    let token = armed_remove(&backend, MINE_ID);
    let outcome = remove_confirm_and_execute(
        &backend,
        Path::new("/repo"),
        token,
        operator(),
        Evidence::TypedName(MINE_ID.to_string()),
        MINE_ID,
    )
    .expect("the removal runs");
    assert_eq!(outcome.result.see_verify, Some(REMOVED_SEE_VERIFY));
    assert_eq!(backend.trust_remove_calls(), 1);
}

/// **A refused adopt is prikk's words, and stikk matches on none of them** (decision 5).
#[test]
fn a_refused_adopt_carries_prikks_own_sentence() {
    let said = "precondition not met: maintainer key id ed25519-6762b6359a650c0a is already \
                adopted with a different public key";
    let backend = trusting(vec![key(MINE_ID, MINE_KEY)]).with_trust_write_refusal(said);
    let token = armed_adopt(&backend, NEW_ID, NEW_KEY);
    let err = adopt_confirm_and_execute(
        &backend,
        Path::new("/repo"),
        token,
        operator(),
        Evidence::TypedName(NEW_ID.to_string()),
        NEW_ID,
        NEW_KEY,
    )
    .expect_err("prikk refused");
    assert!(
        err.to_string()
            .contains("already adopted with a different public key")
    );
    assert_eq!(PRIKK_REFUSED_ADOPT, "prikk refused to adopt this key");
}

/// **No key material in anything this module produces** — the cards included (`NFR-S03`).
#[test]
fn no_card_carries_a_seed_or_a_path() {
    let backend = writing(vec![key(MINE_ID, MINE_KEY), key(THEIRS_ID, THEIRS_KEY)]);
    let adopt_token = armed_adopt(&backend, NEW_ID, NEW_KEY);
    let remove_token = armed_remove(&backend, THEIRS_ID);
    for card in [adopt_token.summary(), remove_token.summary()] {
        let text = format!("{card:?}");
        for forbidden in ["seed", "SEED", "/home/", ".local/share", "private"] {
            assert!(!text.contains(forbidden), "{forbidden} in {text}");
        }
    }
}
