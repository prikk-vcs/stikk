//! **The system's own interface font, named explicitly** (RFC 069 rule 2, the owner's ruling B).
//!
//! Naming the family is the whole point. Unset, iced uses `Font::DEFAULT` — the generic
//! `Family::SansSerif` — and cosmic-text 0.15 resolves that to **"Open Sans"** by name
//! (`src/font/system.rs:157-159`, *"TODO: configurable default fonts"*), unconditionally, after the
//! font database has already been loaded. So the generic family does not reach the user's own
//! choice on any platform, and a front end that wants the user's font has to say its name.
//!
//! **No font is bundled** (rule 2), so there is no licence question and no face of stikk's own.

use std::sync::OnceLock;

/// **The interface font's family name on this platform**, or `None` when stikk cannot learn it.
///
/// - **Linux:** the family fontconfig resolves for `sans-serif`, which is the user's own choice,
///   asked through `fc-match`. The font database iced builds cannot answer this: cosmic-text
///   overwrites its sans-serif family after loading (see the module's documentation), so
///   fontconfig has to be asked directly. One short process at start-up, in a project whose whole
///   seam to prikk is already a process.
/// - **Windows:** `Segoe UI`.
/// - **macOS:** `.AppleSystemUIFont`, the family the system's UI font is enumerated under.
///   **Unverified:** there is no Mac in this project, and the documentation says so (rule 2).
///
/// `None` means iced's own fallback draws, and **the status area says nothing**: it is a rendering
/// matter and not the user's (rule 2).
#[must_use]
pub fn interface_font_name() -> Option<&'static str> {
    static NAME: OnceLock<Option<&'static str>> = OnceLock::new();
    *NAME.get_or_init(|| match named_for_a_measurement() {
        Some(name) => Some(name),
        None => resolve(),
    })
}

/// **`STIKK_GUI_FONT` names the font for a measured run**, and nothing else does.
///
/// RFC 069 rule 2 runs the label-fit guard with **two fonts installed by name** — Liberation Sans
/// as the narrow bound and DejaVu Sans as the wide one — because the CI runner's image documents
/// no text font, and a label that fits both fits any reasonable interface font. A guard cannot name
/// a font the app resolves for itself, so the run names it here, exactly as RFC 065's prototype
/// took `STIKK_PROTO_FONT`.
///
/// **It is an instrument, not a setting.** Unset — which is every ordinary launch — the font is
/// the system's own (rule 2, the owner's ruling B), and nothing reads this.
fn named_for_a_measurement() -> Option<&'static str> {
    let name = std::env::var("STIKK_GUI_FONT").ok()?;
    if name.is_empty() {
        return None;
    }
    Some(Box::leak(name.into_boxed_str()))
}

/// The iced font the app draws every text with.
#[must_use]
pub fn interface_font() -> iced::Font {
    match interface_font_name() {
        Some(name) => iced::Font::with_name(name),
        None => iced::Font::DEFAULT,
    }
}

#[cfg(target_os = "linux")]
fn resolve() -> Option<&'static str> {
    // `%{family[0]}`: the first family of the best match, which is the name iced must be given.
    // A missing `fc-match` — or any non-zero exit — leaves the font unnamed, which rule 2 states
    // as the fallback rather than treating it as an error.
    let output = std::process::Command::new("fc-match")
        .args(["-f", "%{family[0]}", "sans-serif"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if name.is_empty() {
        return None;
    }
    // Leaked once, at start-up, so the name is `'static` for every `Font::with_name` after it.
    // `iced::Font` borrows its name for the program's whole life, which is what makes this the
    // simplest honest shape rather than a cache every caller has to thread.
    Some(Box::leak(name.into_boxed_str()))
}

#[cfg(target_os = "windows")]
fn resolve() -> Option<&'static str> {
    Some("Segoe UI")
}

#[cfg(target_os = "macos")]
fn resolve() -> Option<&'static str> {
    Some(".AppleSystemUIFont")
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn resolve() -> Option<&'static str> {
    None
}

#[cfg(test)]
mod tests;
