//! Test-only helpers shared across the crate's render tests.

use ratatui::buffer::Buffer;

/// Flatten a rendered [`Buffer`] into a newline-separated string, for `contains`-style assertions in
/// render tests (design TS-01 uses `ratatui`'s `TestBackend`, so tests never need a real terminal).
pub(crate) fn buffer_text(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut out = String::new();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buffer.cell((x, y)) {
                out.push_str(cell.symbol());
            }
        }
        out.push('\n');
    }
    out
}

/// **A surface already scrolled to `offset`**, for a test that draws one frame at a position.
///
/// The offsets live in the drawing now (RFC 067 decision 4), so a renderer test makes its own
/// instead of reaching into an `App` that no longer holds any.
pub(crate) fn scrolled_to(offset: u16) -> crate::overlay::Scroll {
    let scrolled = crate::overlay::Scroll::default();
    scrolled.set_offset(offset);
    scrolled
}
