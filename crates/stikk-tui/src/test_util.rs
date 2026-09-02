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
