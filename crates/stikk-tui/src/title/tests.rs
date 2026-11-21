//! The one title builder's own rules (RFC 060 review v1 C1, as RFC 061 decision 5 narrows them).

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

/// **A name and a position, and nothing else** (RFC 061 decision 5).
#[test]
fn the_whole_title_where_it_fits() {
    let title = Title::of("Verify · this run").position(Position::of(0, 11, 24));
    assert_eq!(title.render(120), " Verify · this run — lines 1–11 of 24 ");
    // At 60 it is the same title: there are no keys left to give up, which is the point.
    assert_eq!(title.render(60), " Verify · this run — lines 1–11 of 24 ");
}

/// **The name is shortened from its end and marked** — and **the position is never cut**.
#[test]
fn the_name_is_shortened_and_the_position_is_never_cut() {
    let title = Title::of("Compare · prikk's own fields").position(Position::of(40, 11, 2999));
    for width in 20..=60 {
        let drawn = title.render(width);
        assert!(
            drawn.contains("lines 41–51 of 2999"),
            "at {width}: the position is whole: {drawn:?}"
        );
        // A name that lost anything says so with the mark. At the narrowest widths the name is the
        // mark alone, which is still a name that says it was shortened.
        if !drawn.contains("Compare · prikk's own fields") {
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

/// **RFC 024 §4's em dash**, which is now the only separator a title has.
#[test]
fn the_position_joins_with_an_em_dash() {
    let bare = Title::of("Confirm").position(Position::of(0, 11, 29));
    assert_eq!(bare.render(80), " Confirm — lines 1–11 of 29 ");
    let trust = Title::of("Trust").position(Position::of(0, 11, 29));
    assert_eq!(trust.render(80), " Trust — lines 1–11 of 29 ");
}

/// **No title names a key** (RFC 061 decision 5): there is no builder method that could.
#[test]
fn a_title_cannot_name_a_key() {
    // The type carries a name and a position, and the key line names the keys. This test is the
    // reminder; `every_title_names_no_key` in `shell::tests` holds the drawn frames to it.
    let title = Title::of("History · heads/main");
    assert_eq!(title.render(80), " History · heads/main ");
}
