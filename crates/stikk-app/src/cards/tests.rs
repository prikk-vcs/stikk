//! See [`super`]: every way a card reaches the top mints a surface, and nothing else does.

use super::*;

fn glossary() -> Overlay {
    Overlay::Glossary
}

fn refusals() -> Overlay {
    Overlay::Refusals {
        records: Vec::new(),
        cursor: 0,
    }
}

fn loading() -> Overlay {
    Overlay::Loading {
        what: "a preview",
        seq: 7,
    }
}

/// **A push is a new card**, so it cannot inherit the reading of the one it covered.
#[test]
fn a_pushed_card_has_a_surface_of_its_own() {
    let mut cards = Cards::default();
    cards.push(glossary());
    let first = cards.surface();
    cards.push(refusals());
    assert_ne!(cards.surface(), first);
}

/// **A replace at the top is a new card** — the 0.22.0 regression, in one test.
///
/// A preview pushes a short `Loading` card, whose first draw reports *"the end is on screen"*. When
/// the answer replaces it in its slot, the card that appears must not be holding that report.
#[test]
fn a_card_that_replaces_the_top_has_a_surface_of_its_own() {
    let mut cards = Cards::default();
    cards.push(loading());
    let was = cards.surface();
    cards.replace(0, refusals());
    assert_ne!(
        cards.surface(),
        was,
        "a card that takes another's place is a new card"
    );
}

/// **A replace below the top leaves the reader's surface alone** (trap 1): minting there would throw
/// away the latch of the card they are actually reading.
#[test]
fn a_card_replaced_below_the_top_does_not_touch_the_tops_surface() {
    let mut cards = Cards::default();
    cards.push(loading());
    cards.push(glossary());
    let top = cards.surface();
    cards.replace(0, refusals());
    assert_eq!(cards.surface(), top, "the reader is still reading the top");
}

/// A pop brings a different card to the top, which gets its own reader rather than the one it had
/// before it was covered.
#[test]
fn a_pop_gives_the_card_beneath_a_new_surface() {
    let mut cards = Cards::default();
    cards.push(glossary());
    cards.push(refusals());
    let top = cards.surface();
    let _ = cards.pop();
    assert_ne!(cards.surface(), top);
}

/// A remove mints wherever it takes the card from, which is what the fifty-three hand-written lines
/// did and errs closed.
#[test]
fn a_remove_mints_wherever_it_removes_from() {
    let mut cards = Cards::default();
    cards.push(glossary());
    cards.push(refusals());
    let top = cards.surface();
    let _ = cards.remove(0);
    assert_ne!(cards.surface(), top);
}

/// Truncating to the current length changes no card, so it mints nothing.
#[test]
fn truncating_to_the_same_length_mints_nothing() {
    let mut cards = Cards::default();
    cards.push(glossary());
    let top = cards.surface();
    cards.truncate(1);
    assert_eq!(cards.surface(), top);
    cards.truncate(0);
    assert_ne!(cards.surface(), top, "but dropping the top is a change");
}

/// **One counter for overlays and screens**, so a screen's surface can never equal a card's — the
/// front end keys its offsets by surface, and a collision would share one reader's place in two
/// different texts.
#[test]
fn a_screens_surface_never_equals_a_cards() {
    let mut cards = Cards::default();
    let mut seen = std::collections::HashSet::new();
    for n in 0..50 {
        if n % 2 == 0 {
            cards.push(glossary());
            assert!(seen.insert(cards.surface()), "a card's surface was reused");
        } else {
            assert!(
                seen.insert(cards.mint_for_screen()),
                "a screen's surface collided with a card's"
            );
        }
    }
}
