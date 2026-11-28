//! The four presets, and stikk's overrides over Snora's own values.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{Overrides, Preset, theme, tokens};

/// The four are a two-by-two, and every pair is reachable from every other.
#[test]
fn the_four_presets_are_a_contrast_level_and_a_mode() {
    for preset in Preset::every() {
        assert_eq!(
            Preset::of(preset.is_high_contrast(), preset.is_dark()),
            preset
        );
        assert_eq!(
            preset.with_dark(!preset.is_dark()).is_dark(),
            !preset.is_dark()
        );
        assert_eq!(
            preset.with_dark(!preset.is_dark()).is_high_contrast(),
            preset.is_high_contrast(),
            "changing the mode keeps the contrast level"
        );
        assert_eq!(
            preset.with_contrast(!preset.is_high_contrast()).is_dark(),
            preset.is_dark(),
            "and changing the level keeps the mode"
        );
    }
    let names: Vec<&str> = Preset::every().iter().map(|p| p.name()).collect();
    assert_eq!(names, vec!["light_aaa", "dark_aaa", "hc_light", "hc_dark"]);
}

/// **Rule 5: `targets.min = 44`, set explicitly**, where every Snora preset says 24.
#[test]
fn the_target_override_raises_snoras_own_floor() {
    for preset in Preset::every() {
        assert_eq!(
            tokens(preset, Overrides::stikk()).targets.min,
            44.0,
            "{}: stikk sets the floor",
            preset.name()
        );
        assert_eq!(
            tokens(preset, Overrides::stikk().without_targets_44())
                .targets
                .min,
            24.0,
            "{}: Snora's own floor, which its buttons measure 27.5–28.3 px against",
            preset.name()
        );
    }
}

/// Every preset yields a theme, so stock iced widgets and the window's background follow it.
#[test]
fn every_preset_yields_a_theme() {
    for preset in Preset::every() {
        let _ = theme(preset, Overrides::stikk());
    }
}

/// The two high-contrast presets are the ones that say so.
#[test]
fn the_high_contrast_pair_names_itself() {
    assert!(!Preset::LightAaa.is_high_contrast());
    assert!(!Preset::DarkAaa.is_high_contrast());
    assert!(Preset::HcLight.is_high_contrast());
    assert!(Preset::HcDark.is_high_contrast());
}
