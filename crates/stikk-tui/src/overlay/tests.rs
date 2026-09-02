//! Tests for the overlay layer.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;
use crate::test_util::buffer_text;

fn draw(overlay: &Overlay) -> String {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render(overlay, &Palette::default(), f, f.area()))
        .unwrap();
    buffer_text(terminal.backend().buffer())
}

#[test]
fn help_overlay_renders_the_key_reference() {
    let text = draw(&Overlay::Help);
    assert!(text.contains("Help"));
    assert!(text.contains("refresh"));
    assert!(text.contains("back"));
    // The read-only assurance is part of the help text.
    assert!(text.contains("never writes"));
}

#[test]
fn ref_picker_marks_the_highlighted_ref() {
    let overlay = Overlay::RefPicker {
        refs: vec!["heads/main".into(), "tags/v1".into()],
        cursor: 1,
    };
    let text = draw(&overlay);
    assert!(text.contains("Choose ref"));
    assert!(text.contains("heads/main"));
    assert!(text.contains("tags/v1"));
    assert!(text.contains('▶')); // the highlight marker is drawn
}

#[test]
fn ref_picker_neutralizes_hostile_ref_names() {
    // A ref name carrying an escape sequence must reach no cell verbatim (threat model C-T2a).
    let overlay = Overlay::RefPicker {
        refs: vec!["heads/\u{1b}[2Jmain".into()],
        cursor: 0,
    };
    let text = draw(&overlay);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains('\u{FFFD}'));
}

#[test]
fn notice_renders_the_verbatim_message() {
    let overlay = Overlay::Notice("ref does not exist".into());
    let text = draw(&overlay);
    assert!(text.contains("prikk reported"));
    assert!(text.contains("ref does not exist"));
    assert!(text.contains("dismiss"));
}
