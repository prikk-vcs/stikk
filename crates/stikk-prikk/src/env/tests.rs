//! Tests for presence-only key readiness (threat model C-I1, design TS-04).
//!
//! Readiness logic is tested through the injectable presence lookup, so these tests are hermetic and
//! never mutate the process environment. The security invariant — that the module reads no signing
//! value — is checked at the source level.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;

use super::*;

/// The load-bearing security test (design TS-04): the seam's environment module materializes no
/// variable *value* — only presence. A regression that started reading a seed value (`env::var(...)`,
/// or converting an environment `OsString` to an inspectable string) fails here. This is the
/// greppable invariant the threat model's C-I1c relies on.
#[test]
fn env_module_never_materializes_a_variable_value() {
    const SRC: &str = include_str!("../env.rs");
    // Scan code only: comment/doc lines legitimately *name* the forbidden patterns to explain the
    // rule, so the invariant is enforced against non-comment source lines.
    let code: String = SRC
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    // The value-returning form of the env API must not appear in code.
    assert!(
        !code.contains("env::var("),
        "env.rs must never read a variable value; use presence-only var_os().is_some()"
    );
    // Nor may code convert an environment OsString into an inspectable owned string.
    for forbidden in ["into_string", "to_string_lossy", "to_str(", "to_owned("] {
        assert!(
            !code.contains(forbidden),
            "env.rs must not materialize an environment value ({forbidden} found)"
        );
    }
}

/// A presence lookup backed by a fixed set — the injection point that replaces the real environment.
fn present(names: &[&'static str]) -> impl Fn(&str) -> bool {
    let set: BTreeSet<&'static str> = names.iter().copied().collect();
    move |name: &str| set.contains(name)
}

#[test]
fn no_variables_means_no_readiness() {
    let r = read_readiness_with(present(&[]), false);
    assert!(!r.author_ready);
    assert!(!r.maintainer_ready);
}

#[test]
fn author_ready_needs_both_key_id_and_seed() {
    // Only the key id: not ready.
    let r = read_readiness_with(present(&["PRIKK_AUTHOR_KEY_ID"]), false);
    assert!(!r.author_ready);
    // Both present: ready — and the seed's bytes are never inspected, only its presence.
    let r = read_readiness_with(
        present(&["PRIKK_AUTHOR_KEY_ID", "PRIKK_AUTHOR_SEED"]),
        false,
    );
    assert!(r.author_ready);
    assert!(!r.maintainer_ready);
}

#[test]
fn maintainer_ready_is_independent_of_author() {
    let r = read_readiness_with(
        present(&["PRIKK_MAINTAINER_KEY_ID", "PRIKK_MAINTAINER_SEED"]),
        false,
    );
    assert!(r.maintainer_ready);
    assert!(!r.author_ready);
}

#[test]
fn read_only_flag_is_carried_through() {
    assert!(read_readiness_with(present(&[]), true).read_only);
    assert!(!read_readiness_with(present(&[]), false).read_only);
}
