//! The unavailable-control floors, and the room a scrollable reserves.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use snora::design::contrast::contrast_ratio;

use super::{is_high_contrast, label_floor, mark_floor, scrollbar_spacing};
use crate::{Overrides, Preset, tokens};

/// Which pair of presets a token bundle belongs to is read from the tokens themselves.
#[test]
fn the_contrast_level_is_read_from_the_tokens() {
    for preset in Preset::every() {
        assert_eq!(
            is_high_contrast(&tokens(preset, Overrides::stikk())),
            preset.is_high_contrast(),
            "{}",
            preset.name()
        );
    }
}

/// **Rule 3's two floors, one step higher in the high-contrast presets.**
#[test]
fn the_floors_are_rule_3s_table() {
    assert_eq!(label_floor(false), 4.5);
    assert_eq!(label_floor(true), 7.0);
    assert_eq!(mark_floor(false), 3.0);
    assert_eq!(mark_floor(true), 4.5);
}

/// **Rule 4: a scrollable is given its bar's room**, and the override off gives it none — which is
/// the switch rule 4's failing run turns.
#[test]
fn a_scrollable_is_given_its_bars_room() {
    for preset in Preset::every() {
        let tokens = tokens(preset, Overrides::stikk());
        let spacing = scrollbar_spacing(&tokens, Overrides::stikk());
        assert!(
            spacing.is_some_and(|s| s > 0.0),
            "{}: the bar has room",
            preset.name()
        );
        assert_eq!(
            scrollbar_spacing(&tokens, Overrides::stikk().without_scrollbar_spacing()),
            None
        );
    }
}

/// **Snora's own disabled style is below the floors** — the fact stikk's override exists for
/// (RFC 065 M4 measured 1.48–2.90 : 1), asserted here from the tokens rather than from a picture,
/// so the reason is as cheap to see as the fix.
#[test]
fn snoras_own_disabled_alpha_is_below_the_label_floor() {
    for preset in Preset::every() {
        let tokens = tokens(preset, Overrides::stikk());
        let p = &tokens.palette;
        // What `snora_style::button::disabled_alpha` does: every alpha times 0.45, composited over
        // the surface behind it.
        let faded = snora::design::contrast::composite_over(
            snora::design::Color {
                a: p.accent_text.a * 0.45,
                ..p.accent_text
            },
            snora::design::Color { a: 1.0, ..p.accent },
        );
        let over = snora::design::contrast::composite_over(
            snora::design::Color {
                a: p.accent.a * 0.45,
                ..p.accent
            },
            p.background,
        );
        let ratio = contrast_ratio(faded, over);
        assert!(
            ratio < label_floor(preset.is_high_contrast()),
            "{}: Snora's own disabled label measured {ratio:.2} : 1, which is why stikk overrides \
             it",
            preset.name()
        );
        println!(
            "{}: Snora's own disabled label {ratio:.2} : 1",
            preset.name()
        );
    }
}
