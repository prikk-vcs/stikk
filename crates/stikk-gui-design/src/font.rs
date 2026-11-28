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
/// - **macOS:** the system's UI font, from [`MACOS_CANDIDATES`]. **Which one, if any, is
///   measured and not assumed** — see below.
///
/// **Every candidate is put to the font database before it is named** (`honoured`). A name iced
/// cannot resolve is worse than no name: iced would draw its own fallback either way, and the
/// record would claim a face that never drew. Measured on GitHub's macOS runner, 2026-11-28:
/// `.AppleSystemUIFont` — the family the system UI font is enumerated under, and the obvious
/// name to write — **resolves to nothing** in the database iced uses. That is exactly the shape
/// RFC 069 rule 2 called *unverified* for macOS, now verified, and it is why this function asks
/// rather than states.
///
/// `None` means iced's own fallback draws, and **the status area says nothing**: it is a rendering
/// matter and not the user's (rule 2).
#[must_use]
pub fn interface_font_name() -> Option<&'static str> {
    static NAME: OnceLock<Option<&'static str>> = OnceLock::new();
    *NAME.get_or_init(|| match named_for_a_measurement() {
        // A measurement names its own font and the run says which; it is not put to the database,
        // because a guard that silently fell back to another face would measure the wrong one.
        Some(name) => Some(name),
        None => candidates().into_iter().find(|name| honoured(name)),
    })
}

/// **`STIKK_GUI_FONT` names the font for a measured run**, and nothing else does.
///
/// RFC 069 rule 2 runs the label-fit guard with **two fonts installed by name** — Liberation Sans
/// as the narrow bound and DejaVu Sans as the wide one — because the CI runner's image documents
/// no text font, and a label that fits both fits any reasonable interface font. A guard cannot
/// name a font the app resolves for itself, so the run names it here, exactly as RFC 065's
/// prototype took `STIKK_PROTO_FONT`.
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

/// **Whether the font database iced draws with can honour this family by name.**
///
/// `iced_graphics::text::font_system()` is the one `FontSystem` every iced program draws with, and
/// its `fontdb` is what resolves a family name to a face. A query that answers *some other*
/// family is a fallback, not an answer, so the resolved name is compared with the one asked for.
fn honoured(family: &str) -> bool {
    let Ok(mut system) = iced_graphics::text::font_system().write() else {
        return false;
    };
    let db = system.raw().db();
    let query = iced_graphics::text::cosmic_text::fontdb::Query {
        families: &[iced_graphics::text::cosmic_text::fontdb::Family::Name(
            family,
        )],
        ..Default::default()
    };
    db.query(&query)
        .and_then(|id| db.face(id))
        .and_then(|face| face.families.first().cloned())
        .is_some_and(|(resolved, _)| resolved.eq_ignore_ascii_case(family))
}

/// **The names to try on macOS, in order.**
///
/// There is no Mac in this project (RFC 069 rule 2 says so), so this is a list and not a claim:
/// the first one the database honours is the one named, and a CI run's own log says which.
/// `.AppleSystemUIFont` is first because it is the family the system UI font is enumerated under;
/// it did not resolve on GitHub's runner, which is why the list has more than one entry.
///
/// **Public, and named on every platform**, so a reader of this crate's API can see what the app
/// would try on a Mac without having one, and a review can argue with the list. `candidates` is
/// what is compiled per platform; this is the fact.
pub const MACOS_CANDIDATES: &[&str] = &[
    ".AppleSystemUIFont",
    "SF Pro Text",
    "SF Pro",
    "Helvetica Neue",
    "Helvetica",
];

/// The iced font the app draws every text with.
#[must_use]
pub fn interface_font() -> iced::Font {
    match interface_font_name() {
        Some(name) => iced::Font::with_name(name),
        None => iced::Font::DEFAULT,
    }
}

#[cfg(target_os = "linux")]
fn candidates() -> Vec<&'static str> {
    // `%{family[0]}`: the first family of the best match, which is the name iced must be given.
    // A missing `fc-match` — or any non-zero exit — leaves the font unnamed, which rule 2 states
    // as the fallback rather than treating it as an error.
    let Ok(output) = std::process::Command::new("fc-match")
        .args(["-f", "%{family[0]}", "sans-serif"])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let Ok(name) = String::from_utf8(output.stdout) else {
        return Vec::new();
    };
    let name = name.trim().to_string();
    if name.is_empty() {
        return Vec::new();
    }
    // Leaked once, at start-up, so the name is `'static` for every `Font::with_name` after it.
    // `iced::Font` borrows its name for the program's whole life, which is what makes this the
    // simplest honest shape rather than a cache every caller has to thread.
    vec![Box::leak(name.into_boxed_str())]
}

#[cfg(target_os = "windows")]
fn candidates() -> Vec<&'static str> {
    vec!["Segoe UI"]
}

#[cfg(target_os = "macos")]
fn candidates() -> Vec<&'static str> {
    MACOS_CANDIDATES.to_vec()
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn candidates() -> Vec<&'static str> {
    Vec::new()
}

#[cfg(test)]
mod tests;
