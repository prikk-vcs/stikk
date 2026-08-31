//! The user-authored configuration file (design `stikk-04` MOD-03, external design CF-02).
//!
//! One human-editable, line-oriented file: `key = value`, `#` comments, blank lines. The parser is
//! deliberately forgiving (data model LC-1): an unknown key is preserved and reported, never dropped
//! (INV-4), so a user's newer key survives an older stikk; an unparsable line is warned about and
//! skipped, never a launch blocker. Honesty controls are **not** configurable (external design
//! CF-02/HO-5): this type exposes no field that could turn off derived-marking, the loss boundary,
//! `Unverifiable`, or the fidelity of a refusal — those are not settings.

use std::collections::BTreeMap;

/// The UI theme (external design GU-07). `System` follows the host; the explicit choices win.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// Follow the host's light/dark setting.
    #[default]
    System,
    /// Force the light palette.
    Light,
    /// Force the dark palette.
    Dark,
}

/// The UI locale (requirement NFR-I01). prikk's own diagnostics stay English regardless (NFR-I03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Locale {
    /// English.
    #[default]
    En,
    /// Japanese.
    Ja,
    /// Norwegian Bokmål.
    Nb,
}

/// Confirmation strictness (external design CF-03). May only *tighten* over the requirement's tiers
/// (FR-121), never loosen below them — so there is no "off" value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Confirmation {
    /// The requirement's default tiering.
    #[default]
    Default,
    /// Every mutating operation uses the strictest (typed) confirmation.
    Strict,
}

/// The parsed configuration plus any warnings raised while reading it.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// The chosen theme.
    pub theme: Theme,
    /// The chosen locale.
    pub locale: Locale,
    /// Whether advanced view depth is on by default (external design TU-12/GU-09, NFR-U03).
    pub advanced_mode: bool,
    /// Confirmation strictness.
    pub confirmation: Confirmation,
    /// Keys stikk did not recognize, preserved verbatim so a rewrite keeps them (INV-4).
    pub unknown: BTreeMap<String, String>,
    /// Human-readable warnings raised while parsing (unknown keys, unparsable lines). Shown as a
    /// non-blocking notice on launch (external design OP-01).
    pub warnings: Vec<String>,
}

impl Config {
    /// Parse configuration text. Never fails: an unreadable line becomes a warning and is skipped,
    /// and the result always at least equals [`Config::default`] with the recognized keys applied.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut config = Self::default();
        for (index, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                config.warnings.push(format!(
                    "line {}: ignored, expected `key = value`: {line:?}",
                    index + 1
                ));
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            config.apply(key, value, index + 1);
        }
        config
    }

    fn apply(&mut self, key: &str, value: &str, line: usize) {
        match key {
            "theme" => match value {
                "system" => self.theme = Theme::System,
                "light" => self.theme = Theme::Light,
                "dark" => self.theme = Theme::Dark,
                other => self.warnings.push(format!(
                    "line {line}: unknown theme {other:?}, keeping default"
                )),
            },
            "locale" => match value {
                "en" => self.locale = Locale::En,
                "ja" => self.locale = Locale::Ja,
                "nb" => self.locale = Locale::Nb,
                other => self.warnings.push(format!(
                    "line {line}: unknown locale {other:?}, keeping default"
                )),
            },
            "advanced_mode" => match parse_bool(value) {
                Some(b) => self.advanced_mode = b,
                None => self.warnings.push(format!(
                    "line {line}: advanced_mode expects true/false, got {value:?}"
                )),
            },
            "confirmation" => match value {
                "default" => self.confirmation = Confirmation::Default,
                "strict" => self.confirmation = Confirmation::Strict,
                other => self.warnings.push(format!(
                    "line {line}: confirmation expects default/strict, got {other:?}"
                )),
            },
            other => {
                // Unknown key: preserve it verbatim and note it. A future stikk may know it, and an
                // older stikk must not silently delete a user's newer setting (INV-4).
                self.unknown.insert(other.to_string(), value.to_string());
                self.warnings
                    .push(format!("line {line}: unknown key {other:?}, preserved"));
            }
        }
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" | "yes" | "1" | "on" => Some(true),
        "false" | "no" | "0" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
