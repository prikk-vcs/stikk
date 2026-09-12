//! Tests for the key-id module, including its own source-level guard.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::ffi::OsString;

use super::*;

/// **The load-bearing test for this module** — the mirror of `env.rs`'s `TS-04` guard, pointing the
/// other way: `env.rs` must materialize no value; this module must never so much as *name* a secret
/// variable.
///
/// Reading a seed is forbidden by `C-I1a` regardless of which module does it, and this module is the
/// one that has the value-reading machinery sitting right there — `key_id_with` would read
/// `PRIKK_AUTHOR_SEED` just as happily as it reads an id if someone passed it that name. A source scan
/// for the substring is stronger than any comment, and stronger than reviewing the call: it catches the
/// constant, the literal, and a helper that builds the name by hand.
#[test]
fn key_id_module_never_names_a_seed_variable() {
    const SRC: &str = include_str!("../key_id.rs");
    // Scan code only: the module doc legitimately *names* the rule to explain it, the same convention
    // `env/tests.rs` and `cli_backend/tests.rs`'s `C-I1e` scan use.
    let code: String = SRC
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !code.contains("_SEED"),
        "the key-id module must never name a signing-seed variable (C-I1a); it reads ids, and the \
         module that may touch seed variables at all reads presence only"
    );
    // And the sanity check that the scan is looking at the right file: the ids it *does* read are here.
    assert!(
        code.contains("PRIKK_AUTHOR_KEY_ID") && code.contains("PRIKK_MAINTAINER_KEY_ID"),
        "the guard is scanning the wrong source — it found neither id variable"
    );
}

/// A lookup backed by a fixed map — the injection point that replaces the real environment.
fn env_of(pairs: &[(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> + use<> {
    let pairs: Vec<(String, String)> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    move |name: &str| {
        pairs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| OsString::from(v))
    }
}

#[test]
fn a_configured_id_is_returned() {
    let env = env_of(&[("PRIKK_AUTHOR_KEY_ID", "alice-2026")]);
    assert_eq!(
        key_id_with(&env, "PRIKK_AUTHOR_KEY_ID"),
        Some("alice-2026".to_string())
    );
}

#[test]
fn an_unset_id_is_an_absence_not_a_placeholder() {
    let env = env_of(&[]);
    assert_eq!(key_id_with(&env, "PRIKK_AUTHOR_KEY_ID"), None);
}

#[test]
fn an_empty_id_is_also_an_absence() {
    // Set-but-empty is not an id. Returning `Some("")` would put a blank where a key id belongs, which
    // reads as a rendering bug rather than as "no key configured".
    let env = env_of(&[("PRIKK_AUTHOR_KEY_ID", "")]);
    assert_eq!(key_id_with(&env, "PRIKK_AUTHOR_KEY_ID"), None);
}

#[test]
fn the_two_roles_do_not_read_each_others_variable() {
    let env = env_of(&[("PRIKK_AUTHOR_KEY_ID", "author-id")]);
    assert_eq!(
        key_id_with(&env, "PRIKK_MAINTAINER_KEY_ID"),
        None,
        "an AUTHOR id must never be reported as the MAINTAINER's — the seal ceremony would then name \
         the wrong key as the one about to sign"
    );
}

#[test]
fn a_non_utf8_id_is_kept_lossily_rather_than_vanishing() {
    // Discarding it would have stikk say "no signing id" while prikk signs with one. The replacement
    // character is honest; silence is not.
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let raw = OsString::from_vec(vec![b'a', 0xff, b'b']);
        let env = move |name: &str| (name == "PRIKK_AUTHOR_KEY_ID").then(|| raw.clone());
        let got = key_id_with(env, "PRIKK_AUTHOR_KEY_ID").expect("must not vanish");
        assert!(got.starts_with('a') && got.ends_with('b'), "{got:?}");
        assert!(got.contains('\u{FFFD}'), "{got:?}");
    }
}
