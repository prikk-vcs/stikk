//! See [`super`]: a report is made only for a surface a renderer actually laid out.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use super::*;
use stikk_app::{Direction, Effect, Surface};

/// **No entry is not "at the end"** — it is nothing to report (Handoff C §2.2).
#[test]
fn a_surface_with_no_entry_is_not_reported() {
    let viewports = Viewports::default();
    assert!(
        viewports.reading(Surface::for_test(1)).is_none(),
        "a surface nothing has drawn has nothing to report"
    );
}

/// **Having an entry is not having been drawn** (trap 3).
///
/// `apply` creates an entry for a surface it has only scrolled. In 0.22.0 `reading` answered from
/// whether the entry existed, so a card that had been scrolled but never laid out — or never
/// scrolled and never laid out — was reported as having its end on screen, and the program latched
/// it as read.
#[test]
fn a_surface_only_scrolled_is_not_reported() {
    let mut viewports = Viewports::default();
    let surface = Surface::for_test(2);
    viewports.apply(Effect::Scroll {
        surface,
        direction: Direction::Down,
    });
    assert_eq!(
        viewports.of(surface).offset(),
        1,
        "the scroll was carried out"
    );
    assert!(
        viewports.reading(surface).is_none(),
        "but nothing has drawn it, so there is nothing to report"
    );
}

/// Asking for a surface's offsets is not drawing it either.
#[test]
fn a_surface_merely_asked_for_is_not_reported() {
    let mut viewports = Viewports::default();
    let surface = Surface::for_test(3);
    let _ = viewports.of(surface);
    assert!(viewports.reading(surface).is_none());
}

/// **Once a renderer has measured it, it is reported** — and the report is what that renderer found.
#[test]
fn a_surface_a_renderer_measured_is_reported() {
    let mut viewports = Viewports::default();
    let surface = Surface::for_test(4);
    viewports.of(surface).note_max(9);
    let reading = viewports
        .reading(surface)
        .expect("a measured surface is reported");
    assert_eq!(reading.surface, surface);
    assert!(!reading.more_above, "it is at the top");
    assert!(
        !reading.at_end,
        "and nine rows below the fold, so the end is not on screen"
    );
}
