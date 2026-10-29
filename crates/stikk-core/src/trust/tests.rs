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

/// **The three answers**, from one read (RFC 054 decision 3).
#[test]
fn a_sealer_is_yours_trusted_or_not_trusted() {
    let backend = trusting(vec![key(MINE_ID, MINE_KEY), key(THEIRS_ID, THEIRS_KEY)]);
    let out = judged(&backend, &[MINE_ID, THEIRS_ID, "ed25519-0000000000000000"]);
    assert_eq!(out[0].trust, SealerTrust::Yours);
    assert_eq!(out[1].trust, SealerTrust::Trusted);
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
