//! **Rule 3's logic level:** who decides the preset, and what the desktop may not override.
//!
//! `iced_test` cannot answer `system::theme()` — its emulator drops `Action::System` with a `TODO`
//! (`emulator.rs:262-265`) — so the rule's H test goes through the program's own message, which is
//! what a real desktop's answer becomes anyway.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use iced::theme::Mode;
use stikk_gui_design::Preset;
use stikk_state::config::Theme as Configured;

use super::{Choice, Source};

/// **With no preference at all, `light_aaa`** — knotra's *"for many older eyes"*.
#[test]
fn nothing_chosen_and_nothing_reported_opens_light_aaa() {
    let choice = Choice::from_config(Configured::System);
    assert_eq!(choice.preset(), Preset::LightAaa);
    assert_eq!(choice.source(), Source::Desktop);
}

/// **The desktop decides when nothing is chosen**, at launch and while the app is open.
#[test]
fn the_desktop_switches_the_preset_when_none_is_chosen() {
    let mut choice = Choice::from_config(Configured::System);
    assert!(choice.desktop_says(Mode::Dark), "the preset changed");
    assert_eq!(choice.preset(), Preset::DarkAaa);
    assert!(choice.desktop_says(Mode::Light), "and back again");
    assert_eq!(choice.preset(), Preset::LightAaa);
    // **No preference is not light**: it leaves the fallback in place rather than forcing one.
    let mut none = Choice::from_config(Configured::System);
    assert!(!none.desktop_says(Mode::None));
    assert_eq!(none.preset(), Preset::LightAaa);
}

/// **A chosen preset wins, and the desktop never overrides it** (rule 3).
///
/// **This is the one that must fail** if a version lets the desktop through: the assertion is on
/// the preset after the desktop has said the opposite of the configuration, in both directions.
#[test]
fn a_chosen_mode_is_left_alone_however_the_desktop_changes() {
    for (configured, preset) in [
        (Configured::Light, Preset::LightAaa),
        (Configured::Dark, Preset::DarkAaa),
    ] {
        let mut choice = Choice::from_config(configured);
        assert_eq!(choice.preset(), preset);
        assert_eq!(choice.source(), Source::Configured);
        for mode in [Mode::Dark, Mode::Light, Mode::None] {
            assert!(
                !choice.desktop_says(mode),
                "{configured:?}: the desktop saying {mode:?} changed nothing"
            );
            assert_eq!(
                choice.preset(),
                preset,
                "{configured:?} still draws {preset:?} after the desktop said {mode:?}"
            );
        }
    }
}

/// **The run that fails it**, written as the rule's own counterfactual: a choice whose mode the
/// desktop is allowed to replace draws the other preset, which is what the assertion above forbids.
///
/// Written against the same type rather than a second copy of the logic: `desktop_says` records
/// what the desktop said either way, and `preset` is where the precedence lives — so taking the
/// recorded answer instead of the configured one is exactly the mistake, and it is visible here.
#[test]
fn letting_the_desktop_replace_a_chosen_mode_draws_the_other_preset() {
    let mut choice = Choice::from_config(Configured::Dark);
    let _ = choice.desktop_says(Mode::Light);
    // What the rule's own precedence gives:
    assert_eq!(choice.preset(), Preset::DarkAaa);
    // What a version without the precedence would give, from the same recorded answer:
    assert_eq!(
        Preset::of(false, choice.desktop_reported_dark().unwrap_or(false)),
        Preset::LightAaa,
        "the desktop's own answer was Light, so a version that used it would draw light_aaa \
         against a configuration that says dark"
    );
}

/// **High contrast is the level, and the desktop chooses the mode within it** — which is how all
/// four presets are reachable from a configuration key that has no high-contrast-dark value.
#[test]
fn high_contrast_keeps_its_level_and_follows_the_mode() {
    let mut choice = Choice::from_config(Configured::HighContrast);
    assert_eq!(choice.preset(), Preset::HcLight);
    assert_eq!(choice.source(), Source::ConfiguredLevelOnly);
    assert!(choice.desktop_says(Mode::Dark));
    assert_eq!(choice.preset(), Preset::HcDark);
    assert!(choice.desktop_says(Mode::Light));
    assert_eq!(choice.preset(), Preset::HcLight);
}

/// **`plain` is the terminal's, and means nothing in a window** — so it reads as `system` rather
/// than as a surprise for a user who set it for the terminal.
#[test]
fn plain_follows_the_desktop_as_system_does() {
    let mut choice = Choice::from_config(Configured::Plain);
    assert_eq!(choice.source(), Source::Desktop);
    assert!(choice.desktop_says(Mode::Dark));
    assert_eq!(choice.preset(), Preset::DarkAaa);
}
