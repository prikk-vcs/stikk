//! Tests for the Trust view (RFC 054 decision 2).
//!
//! The keys are prikk 0.54.1's own, from
//! `.git-exclude/reports/054-trust-read/measure054-0.54.txt`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use stikk_core::{TrustRow, TrustView};

use super::*;
use crate::test_util::buffer_text;

const MINE_ID: &str = "ed25519-31f369d3abb7f64c";
const MINE_KEY: &str = "31f369d3abb7f64cd2cf70be31158181e361f25444d8de122cdff59389e7e946";
const THEIRS_ID: &str = "ed25519-9c0a44b1d2e3f501";
const THEIRS_KEY: &str = "9c0a44b1d2e3f5012b7c0a4e55d2117ab3340ef6627c118e2b4a9d0cf3e21847";

fn row(id: &str, public: &str, yours: bool) -> TrustRow {
    TrustRow {
        key_id: id.to_string(),
        public_key: public.to_string(),
        yours,
    }
}

fn drawn(view: &TrustView, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let offset = std::cell::Cell::new(0);
    terminal
        .draw(|f| render(view, &offset, &Palette::default(), f, f.area()))
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

fn listed(rows: Vec<TrustRow>) -> TrustView {
    TrustView::Listed {
        rows,
        own_key_unknown: None,
    }
}

/// **Capture: nothing adopted, at 80 columns.** The answer, what to run, and where the key
/// should come from.
#[test]
fn capture_the_empty_trust_view_at_80_columns() {
    let text = drawn(&listed(Vec::new()), 80, 24);
    println!("{text}");
    assert!(text.contains("Keys this repository trusts"), "{text}");
    assert!(text.contains("trusts no maintainer key"), "{text}");
    // **prikk's command, as text.** stikk runs nothing.
    assert!(
        text.contains("prikk trust maintainer add --key-id ID --public-key HEX"),
        "{text}"
    );
    assert!(
        text.contains("not from a bundle they sent"),
        "the one thing a newcomer gets wrong: {text}"
    );
    // And what being on the list would mean, which is the point of the view.
    assert!(text.contains("never"), "the sentence is there: {text}");
}

/// **Capture: two keys, with yours marked, at 80 columns.**
#[test]
fn capture_two_keys_with_yours_marked_at_80_columns() {
    let text = drawn(
        &listed(vec![
            row(MINE_ID, MINE_KEY, true),
            row(THEIRS_ID, THEIRS_KEY, false),
        ]),
        80,
        24,
    );
    println!("{text}");
    assert!(text.contains(MINE_ID), "{text}");
    assert!(text.contains(THEIRS_ID), "{text}");
    // **One row marked, and only one.**
    assert_eq!(text.matches("yours").count(), 1, "{text}");
    let marked = text
        .lines()
        .find(|l| l.contains("yours"))
        .unwrap_or_else(|| panic!("a marked row: {text}"));
    assert!(
        marked.contains(MINE_ID),
        "the right row is marked: {marked}"
    );
}

/// **A 64-hex public key is hard-split to its column**, not word-wrapped onto nothing — the
/// rule Block detail's ids follow.
#[test]
fn a_long_public_key_is_split_rather_than_wrapped_on_whitespace() {
    // Narrow enough that the key cannot fit on one row.
    let text = drawn(&listed(vec![row(MINE_ID, MINE_KEY, false)]), 40, 16);
    println!("{text}");
    let flat: String = text
        .chars()
        .filter(|c| !matches!(c, '│' | '┌' | '┐' | '└' | '┘' | '─' | ' ' | '\n'))
        .collect();
    assert!(
        flat.contains(MINE_KEY),
        "every byte of the key is on screen, in order: {text}"
    );
    assert!(
        text.lines().all(|l| l.chars().count() <= 40),
        "and nothing ran past the panel: {text}"
    );
}

/// **No key material, asserted against the rendered buffer** (`NFR-S03`, §7 trap 1) — not
/// against the model, which is where a leak would be invisible.
#[test]
fn nothing_in_the_view_carries_a_seed_or_a_path() {
    for view in [
        listed(vec![row(MINE_ID, MINE_KEY, true)]),
        listed(Vec::new()),
        TrustView::Unavailable(stikk_core::trust_unavailable(53)),
        TrustView::Failed("integrity error: the trust store is unreadable".to_string()),
    ] {
        let text = drawn(&view, 80, 24);
        for forbidden in ["seed", "Seed", ".config", "/home/", "maintainer.seed"] {
            assert!(
                !text.contains(forbidden),
                "the rendered view must carry no {forbidden}: {text}"
            );
        }
    }
}

/// **Below the band, and a refused list**: stikk's own sentence or prikk's, and nothing else —
/// no heading over an empty list, which would read as *nothing is adopted*.
#[test]
fn the_band_and_a_refusal_each_say_one_thing() {
    let text = drawn(
        &TrustView::Unavailable(stikk_core::trust_unavailable(53)),
        80,
        24,
    );
    assert!(text.contains("0.54"), "the band is named: {text}");
    assert!(
        !text.contains("trusts no maintainer key"),
        "an unread list is not an empty one: {text}"
    );

    let text = drawn(
        &TrustView::Failed("integrity error: the trust store is unreadable".to_string()),
        80,
        24,
    );
    assert!(text.contains("the trust store is unreadable"), "{text}");
    assert!(!text.contains("trusts no maintainer key"), "{text}");
}

/// When `key status` could not be read, the list stands and one line says why nothing is
/// marked.
#[test]
fn an_unknown_own_key_is_said_once_above_the_list() {
    let text = drawn(
        &TrustView::Listed {
            rows: vec![row(MINE_ID, MINE_KEY, false)],
            own_key_unknown: Some(stikk_core::OWN_KEY_UNKNOWN.to_string()),
        },
        80,
        24,
    );
    println!("{text}");
    assert!(text.contains("could not read which key is yours"), "{text}");
    assert!(text.contains(MINE_ID), "the list is still shown: {text}");
    assert!(!text.contains("— yours"), "and nothing is marked: {text}");
}
