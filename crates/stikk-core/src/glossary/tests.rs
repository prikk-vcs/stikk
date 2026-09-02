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
