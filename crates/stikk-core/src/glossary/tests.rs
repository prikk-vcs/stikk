//! Tests for the glossary asset (design DM-09, RR-5; RFC 007).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn a_seeded_code_resolves() {
    let entry = lookup("unverifiable-author-signature").expect("seeded code");
    assert!(entry.explanation.contains("NOT a failure"));
}

#[test]
fn an_unknown_code_degrades_to_none() {
    // RR-5 / NFR-I03: a missing code is None; the caller shows prikk's message, never hides it.
    assert!(lookup("brand-new-prikk-code-99").is_none());
}

#[test]
fn the_terminology_mapping_is_seeded_and_redirects_git_expectations() {
    let terms = terminology();
    assert!(terms.len() >= 10);
    let head = terms
        .iter()
        .find(|t| t.git == "HEAD")
        .expect("HEAD mapping");
    assert!(head.prikk.contains("none"));
    // A couple of the load-bearing redirects the copy relies on.
    assert!(
        terms
            .iter()
            .any(|t| t.git.contains("revert") && t.prikk.contains("rollback"))
    );
    assert!(terms.iter().any(|t| t.git.contains("switch branch")));
}

#[test]
fn codes_in_links_only_named_codes() {
    let named = codes_in("verify finding: unverifiable-author-signature on block 76cee1dc");
    assert_eq!(named, vec!["unverifiable-author-signature"]);
    let none = codes_in("a plain refusal with no code in it");
    assert!(none.is_empty());
}

#[test]
fn the_prikkignore_code_resolves_and_links_from_a_real_refusal() {
    // RFC 009 F5.
    let entry = lookup(".prikkignore").expect("seeded code");
    assert!(entry.explanation.contains("CON-1"));
    let named = codes_in(
        "error: invalid name: .prikkignore line 1: invalid name: absolute paths are not allowed",
    );
    assert_eq!(named, vec![".prikkignore"]);
}

#[test]
fn the_schema_skew_code_resolves_and_links_from_a_real_refusal() {
    // RFC 012 F-e — captured live from a real prikk 0.30.0 reading a repository a real prikk 0.31.0
    // had written.
    let entry = lookup(SCHEMA_SKEW_CODE).expect("seeded code");
    assert!(entry.explanation.contains("newer prikk"));
    let named = codes_in(
        "integrity error: format-2 patch does not accept envelope schema 3 (accepted: [1, 2])",
    );
    assert_eq!(named, vec![SCHEMA_SKEW_CODE]);
}

#[test]
fn the_bundle_decode_skew_code_resolves_and_links_from_a_real_refusal() {
    // RFC 015 F5 — captured live: a real prikk 0.31.1 verifying a bundle a real prikk 0.32.0 exported.
    let entry = lookup(BUNDLE_DECODE_SKEW_CODE).expect("seeded code");
    assert!(entry.explanation.contains("newer prikk"));
    let named = codes_in(
        "error: malformed persisted data: invalid PatchPurpose canonical form: canonical encoding \
         error: unknown PatchPayload field tag: 6",
    );
    assert_eq!(named, vec![BUNDLE_DECODE_SKEW_CODE]);
}

#[test]
fn the_full_queue_code_resolves_and_links_from_a_real_refusal() {
    // RFC 017 F4 — captured live against a real prikk 0.33.0 binary on the commit path.
    let entry = lookup(FULL_QUEUE_CODE).expect("seeded code");
    assert!(!entry.explanation.contains("another writer is active"));
    assert!(entry.explanation.to_ascii_lowercase().contains("seal"));
    let named = codes_in(
        "lock conflict: active WAL has 1 queued patches, at or above the configured limit (1); run \
         `prikk seal` before committing again",
    );
    assert_eq!(named, vec![FULL_QUEUE_CODE]);
}

#[test]
fn the_trust_refusal_code_resolves_and_links_from_both_captured_wordings() {
    // RFC 016 §9/v2 — captured live against real prikk 0.28.0 and 0.33.0 binaries on the seal path.
    let entry = lookup(TRUST_REFUSAL_CODE).expect("seeded code");
    assert!(entry.explanation.contains("object trust"));
    assert!(
        !entry
            .explanation
            .to_ascii_lowercase()
            .contains("may publish")
    );
    assert!(entry.explanation.contains("cannot verify"));
    for message in [
        "invalid signature: maintainer signer key id different-maintainer is not trusted by policy",
        "invalid signature: maintainer signer public key does not match trusted key maintainer",
    ] {
        assert_eq!(codes_in(message), vec![TRUST_REFUSAL_CODE]);
    }
}

/// **The notation entry says what a reader actually meets** (RFC 035 review v2 §2): the quotes, the
/// short escapes, and the `\u` form — which is the one nobody guesses. Asserted against the escapes
/// `stikk_prikk::escape` really produces, so a widened table cannot leave the Glossary describing an
/// older one.
#[test]
fn the_content_notation_names_every_escape_a_reader_can_meet() {
    let notation = notations().first().expect("the notation ships");
    assert!(notation.title.contains("Patch detail"));
    for spelling in ["\\n", "\\t", "\\uXXXX"] {
        assert!(
            notation.note.contains(spelling),
            "the notation must name {spelling}"
        );
    }
    // Every escape class the shared table produces is one of the spellings named above.
    for ch in ['\n', '\t', '\u{202e}', '\u{200b}', '\u{a0}'] {
        let rendered = stikk_prikk::escape::quoted(&ch.to_string());
        let named =
            rendered.contains("\\n") || rendered.contains("\\t") || rendered.contains("\\u");
        assert!(named, "U+{:04X} renders as {rendered}", u32::from(ch));
    }
    assert!(
        notation.note.contains("never look alike"),
        "the invariant is the reason the notation exists, so it is stated"
    );
}

/// **RFC 036 decision 7: the Glossary names the quoted-name convention too.** A reader who meets a
/// quoted path with escaped spaces has somewhere to look it up — the same reason the content-row
/// notation exists.
#[test]
fn the_notation_names_quoted_names_as_well_as_quoted_content() {
    let names: Vec<&str> = notations().iter().map(|n| n.title).collect();
    assert_eq!(names.len(), 2, "two notations ship: {names:?}");
    let quoted_names = notations()
        .iter()
        .find(|n| n.title.contains("Quoted names"))
        .expect("the quoted-name notation ships");
    for wanted in ["u0020", "quote", "backslash", "never look alike"] {
        assert!(
            quoted_names.note.contains(wanted),
            "the notation must name {wanted}"
        );
    }
    assert!(
        quoted_names.note.contains("never quoted"),
        "and must say that prose is not quoted, which is the distinction a reader needs"
    );
}
