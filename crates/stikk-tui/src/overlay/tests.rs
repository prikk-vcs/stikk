//! Tests for the overlay layer.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;
use crate::test_util::buffer_text;

#[test]
fn help_overlay_renders_the_key_reference() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render(Overlay::Help, &Palette::default(), f, f.area()))
        .unwrap();
    let text = buffer_text(terminal.backend().buffer());
    assert!(text.contains("Help"));
    assert!(text.contains("refresh"));
    assert!(text.contains("quit"));
    // The read-only assurance is part of the help text.
    assert!(text.contains("never writes"));
}
