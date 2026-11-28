//! The interface font's name, resolved once.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{interface_font, interface_font_name};

/// **The name is resolved once and answers the same every time** — `iced::Font` borrows it for the
/// program's whole life, which is why it is leaked rather than cached per caller.
#[test]
fn the_interface_font_is_resolved_once() {
    let first = interface_font_name();
    let second = interface_font_name();
    assert_eq!(first, second);
    println!("the interface font on this platform: {first:?}");
}

/// **A name is only given to iced when the font database can honour it** (rule 2), and a platform
/// stikk cannot ask falls back to iced's own, saying nothing to the user: it is a rendering matter
/// and not theirs.
#[test]
fn an_unknown_font_falls_back_to_iceds_own() {
    let font = interface_font();
    match interface_font_name() {
        Some(_) => assert_ne!(
            font,
            iced::Font::DEFAULT,
            "a named font is not the generic family"
        ),
        None => assert_eq!(font, iced::Font::DEFAULT),
    }
}

/// **Windows is Segoe UI, by name** (rule 2) — asserted where it is compiled, so the Windows CI
/// job holds it.
#[cfg(target_os = "windows")]
#[test]
fn windows_names_segoe_ui() {
    assert_eq!(interface_font_name(), Some("Segoe UI"));
}

/// **macOS names whichever of its candidates the database honours**, and the record says which.
///
/// There is no Mac in this project, so this asserts the shape and prints the answer rather than
/// naming a face nobody here can check (rule 2's *unverified*). `None` is a legitimate outcome:
/// iced's own fallback then draws, which rule 2 allows.
#[cfg(target_os = "macos")]
#[test]
fn macos_names_a_font_the_database_honours() {
    let name = interface_font_name();
    println!("macOS resolved the interface font to {name:?}");
    if let Some(name) = name {
        assert!(
            super::MACOS_CANDIDATES.contains(&name),
            "{name:?} is one of the candidates"
        );
    }
}

/// **Linux is whatever fontconfig resolves for `sans-serif`** — the user's own choice, which is
/// why no name is written here. The assertion is that something was learned, since a Linux desktop
/// that draws at all has fontconfig.
#[cfg(target_os = "linux")]
#[test]
fn linux_takes_fontconfigs_own_answer() {
    let name = interface_font_name();
    assert!(
        name.is_some(),
        "fc-match answered nothing; iced's fallback would draw, and rule 2 allows that — but on a \
         machine with fontconfig this is a finding worth seeing"
    );
}
