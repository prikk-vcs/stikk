//! The one title builder's own rules (RFC 060 review v1 C1).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

/// The position words, and when there is no position at all.
#[test]
fn a_position_is_only_said_when_something_is_hidden() {
    assert_eq!(Position::of(0, 20, 20), None, "everything fits");
    assert_eq!(Position::of(0, 20, 12), None);
    let some = Position::of(3, 11, 29).expect("more than fits");
    assert_eq!((some.first, some.last, some.total), (4, 14, 29));
    assert_eq!(some.words(), "lines 4–14 of 29");
}

/// **The whole title where it fits**, in the shape every frame has used.
#[test]
fn the_whole_title_where_it_fits() {
    let title = Title::of("Verify · this run")
        .keys(&["v: raw", "r: re-run", "↑/↓"])
        .position(Position::of(0, 11, 24));
    assert_eq!(
        title.render(120),
        " Verify · this run — v: raw · r: re-run · ↑/↓ · lines 1–11 of 24 "
    );
}

/// **The keys go first, as one group** — never one of them at a time.
#[test]
fn the_keys_are_dropped_as_one_group() {
    let title = Title::of("Verify · this run")
        .keys(&["v: raw", "r: re-run", "↑/↓"])
        .position(Position::of(0, 11, 24));
    let at_60 = title.render(60);
    assert_eq!(at_60, " Verify · this run — lines 1–11 of 24 ");
    assert!(!at_60.contains("v: raw"), "{at_60:?}");
    assert!(!at_60.contains("r: re-run"), "{at_60:?}");
    assert!(!at_60.contains('↑'), "{at_60:?}");
}

/// **Then the name, from its end and marked** — and **the position is never cut**.
#[test]
fn the_name_is_shortened_and_the_position_is_never_cut() {
    let title = Title::of("Compare · prikk's own fields")
        .keys(&["v: rendered"])
        .position(Position::of(40, 11, 2999));
    for width in 20..=60 {
        let drawn = title.render(width);
        assert!(
            drawn.contains("lines 41–51 of 2999"),
            "at {width}: the position is whole: {drawn:?}"
        );
        // **Whatever is given up is given up in the right order**: the keys first, then the name
        // from its end — and a name that lost anything says so with the mark. At the narrowest
        // widths the name is the mark alone, which is still a name that says it was shortened.
        if !drawn.contains("v: rendered") && !drawn.contains("Compare · prikk's own fields") {
            assert!(
                drawn.contains('…'),
                "at {width}: a shortened name says so: {drawn:?}"
            );
        }
    }
}

/// A frame that is only a name keeps the shape it always had.
#[test]
fn a_name_on_its_own_is_unchanged() {
    assert_eq!(Title::of("Orientation").render(80), " Orientation ");
    assert_eq!(Title::of("Block · tip").render(80), " Block · tip ");
}

/// **RFC 024 §4's em dash where there are no keys**, and the keys' own separator where there are.
#[test]
fn the_position_joins_with_the_right_separator() {
    let bare = Title::of("Confirm").position(Position::of(0, 11, 29));
    assert_eq!(bare.render(80), " Confirm — lines 1–11 of 29 ");
    let with_keys = Title::of("Trust")
        .keys(&["a adopts"])
        .position(Position::of(0, 11, 29));
    assert_eq!(
        with_keys.render(80),
        " Trust — a adopts · lines 1–11 of 29 "
    );
}
