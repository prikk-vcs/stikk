//! **Where every surface is scrolled to** (RFC 067 decision 4) — the offsets, in the drawing.
//!
//! Until RFC 067 the program held them: ten `Cell<u16>`s on `App` and one `Scroll` for the overlay
//! on top. But an offset is a fact about a window — about how a text wrapped at a width, and how
//! many rows were left over — and the program has no window. So the offsets live here, beside the
//! renderer that is the only thing able to clamp them, and the program is told what the reader saw
//! through an [`crate::app::Reading`] after each draw.
//!
//! **Keyed by [`Surface`]**, which the program mints and hands out. A surface that is no longer
//! current simply never comes up again, and a new one has no entry, so a new card starts at the top
//! with nothing inherited — which is what stamping the old `Scroll` with a generation was for.

use std::collections::HashMap;

use crate::app::{Direction, Effect, Reading, Surface};
use crate::overlay::Scroll;

/// Every surface's offset, by surface.
#[derive(Debug, Default)]
pub struct Viewports {
    by_surface: HashMap<Surface, Scroll>,
}

impl Viewports {
    /// The offsets for `surface`, created at the top of the text if this is the first draw of it.
    pub(crate) fn of(&mut self, surface: Surface) -> &Scroll {
        self.by_surface.entry(surface).or_default()
    }

    /// **Carry out what the program asked for** (RFC 067 decision 4).
    ///
    /// Unclamped, as the key handlers were before: the renderer clamps on the next draw, against a
    /// height only it knows. An effect for a surface never drawn is applied to a fresh entry, which
    /// the next draw clamps like any other.
    pub(crate) fn apply(&mut self, effect: Effect) {
        let Effect::Scroll { surface, direction } = effect else {
            return;
        };
        let scroll = self.of(surface);
        let offset = scroll.offset();
        scroll.set_offset(match direction {
            Direction::Up => offset.saturating_sub(1),
            Direction::Down => offset.saturating_add(1),
        });
    }

    /// **Forget every surface the reader can no longer return to** (review v1 F1).
    ///
    /// The map gains an entry for every surface drawn, and surfaces are minted afresh by every card
    /// opened, every view replaced and every reload — 69 places. Without this a long session's map
    /// would grow without bound, holding offsets for text nobody can ever see again.
    ///
    /// **It must not forget what is still live**, which is why the program is asked rather than
    /// guessed at: a view deeper in the screen stack keeps its offset, so a reader who opens a card
    /// over it, or walks into a deeper screen and comes back, finds it where they left it.
    pub(crate) fn retain(&mut self, live: &[Surface]) {
        self.by_surface.retain(|surface, _| live.contains(surface));
    }

    /// How many surfaces are held. **For the test that this map does not grow without bound.**
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.by_surface.len()
    }

    /// **What the last draw of `surface` found**, to report to the program.
    ///
    /// A surface with no entry has not been drawn, and reads as *"nothing above, and the end is on
    /// screen"* — the same answer an undrawn card gave when the program held the offsets.
    pub(crate) fn reading(&self, surface: Surface) -> Reading {
        let scroll = self.by_surface.get(&surface);
        Reading {
            surface,
            more_above: scroll.is_some_and(Scroll::more_above),
            at_end: scroll.is_none_or(Scroll::at_end),
        }
    }
}
