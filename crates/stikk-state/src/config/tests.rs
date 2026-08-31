//! Tests for config parsing (design TS-06; data model LC-1/INV-4).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn empty_input_yields_defaults_with_no_warnings() {
    let c = Config::parse("");
    assert_eq!(c.theme, Theme::System);
    assert_eq!(c.locale, Locale::En);
    assert!(!c.advanced_mode);
    assert_eq!(c.confirmation, Confirmation::Default);
    assert!(c.warnings.is_empty());
}

#[test]
fn recognized_keys_apply() {
    let c = Config::parse(
        "\
# stikk config
theme = dark
locale = ja
advanced_mode = true
confirmation = strict
",
    );
    assert_eq!(c.theme, Theme::Dark);
    assert_eq!(c.locale, Locale::Ja);
    assert!(c.advanced_mode);
    assert_eq!(c.confirmation, Confirmation::Strict);
    assert!(c.warnings.is_empty());
}

#[test]
fn unknown_keys_are_preserved_not_dropped() {
    // INV-4: a newer stikk's key must survive an older stikk reading the file.
    let c = Config::parse("future_feature = enabled\ntheme = light\n");
    assert_eq!(c.theme, Theme::Light);
    assert_eq!(
        c.unknown.get("future_feature").map(String::as_str),
        Some("enabled")
    );
    assert!(
        c.warnings.iter().any(|w| w.contains("future_feature")),
        "an unknown key must be reported"
    );
}

#[test]
fn a_malformed_line_warns_and_is_skipped_never_blocks() {
    // LC-1: a syntax problem never blocks launch; it degrades to a warning + defaults.
    let c = Config::parse("this line has no equals sign\ntheme = dark\n");
    assert_eq!(c.theme, Theme::Dark);
    assert!(
        c.warnings
            .iter()
            .any(|w| w.contains("expected `key = value`"))
    );
}

#[test]
fn an_unknown_enum_value_keeps_the_default_and_warns() {
    let c = Config::parse("theme = ultraviolet\n");
    assert_eq!(c.theme, Theme::System);
    assert!(c.warnings.iter().any(|w| w.contains("unknown theme")));
}

#[test]
fn there_is_no_setting_that_disables_honesty() {
    // CF-02/HO-5: honesty controls are not configurable. This test documents that the Config type
    // exposes no such field; the compiler enforces the shape. Setting a bogus honesty key is treated
    // as an unknown key (preserved, inert), never honored.
    let c = Config::parse("disable_unverifiable = true\n");
    // It landed in `unknown`, i.e. it changed no behavior.
    assert!(c.unknown.contains_key("disable_unverifiable"));
}
