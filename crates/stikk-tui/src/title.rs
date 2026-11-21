//! **Every frame's title, built in one place** (RFC 060 review v1 C1).
//!
//! # What went wrong
//!
//! Each view and each panel joined a name, its key hints and its scroll position into one string and
//! handed it to `Block::title`, which **clips**. At 60 columns Verify's read
//! `┌ Verify · this run — v: raw · r: re-run · ↑/↓ · lines 1–11┐`: the cut fell inside the position,
//! so the reader was told *"lines 1–11"* with no *"of 24"*. That is not a missing word, it is a wrong
//! statement — and RFC 060 exists so that nothing is cut in silence.
//!
//! # The rule
//!
//! **A title is a name and a position** (RFC 061 decision 5):
//!
//! 1. **the name** — what the frame is: *"Verify · this run"*, *"Choose ref"*, *"Confirm"*;
//! 2. **the position** — *"lines a–b of c"*, RFC 024 §4's idiom.
//!
//! When it does not fit its frame, **the name is shortened from its end, marked with `…`**. **The
//! position is never cut and never shortened**: it is the one part of a title that is a statement
//! about what the reader cannot see, and a half of it is worse than none of it. If even the position
//! cannot fit, the frame is below stikk's minimum, which `shell`'s own guard rules out.
//!
//! **No title names a key.** RFC 060 had taken the scroll key out of an overlay's title, leaving one
//! foot line to name it (decision 3); RFC 061 decision 5 takes the rest out of the views' titles,
//! because the key line names every key a screen has, and a reader who has to look in two places to
//! learn one key has been told twice and trusts neither.

/// Where the reader is in a surface taller than its frame: RFC 024 §4's idiom, *"lines a–b of c"*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Position {
    /// The first row on screen, counting from one.
    pub(crate) first: usize,
    /// The last row on screen.
    pub(crate) last: usize,
    /// How many rows there are in all.
    pub(crate) total: usize,
}

impl Position {
    /// The position of a `viewport`-row window at `offset` over `total` rows, or `None` when
    /// everything fits and there is nothing to say.
    pub(crate) fn of(offset: u16, viewport: u16, total: usize) -> Option<Self> {
        if total <= usize::from(viewport) {
            return None;
        }
        let first = usize::from(offset) + 1;
        let last = (usize::from(offset) + usize::from(viewport)).min(total);
        Some(Self { first, last, total })
    }

    /// The words, without their separator.
    fn words(self) -> String {
        format!(
            "lines {}\u{2013}{} of {}",
            self.first, self.last, self.total
        )
    }
}

/// A frame's title — see the module doc.
#[derive(Debug, Clone)]
pub(crate) struct Title<'a> {
    name: &'a str,
    position: Option<Position>,
}

impl<'a> Title<'a> {
    /// A frame that is only a name: *"Orientation"*, *"Loading"*, *"Confirm"*.
    pub(crate) fn of(name: &'a str) -> Self {
        Self {
            name,
            position: None,
        }
    }

    /// Where the reader is, when there is more than the frame shows.
    pub(crate) fn position(mut self, position: Option<Position>) -> Self {
        self.position = position;
        self
    }

    /// The title string for a frame `frame_width` columns wide, border included.
    ///
    /// Records an overflow in tests if even the shortest form does not fit, so a title is held to its
    /// frame exactly as a row is (RFC 060 review v1 C1, rule 5).
    pub(crate) fn render(&self, frame_width: u16) -> String {
        // The two corner characters are the title's own frame; everything between them is its room.
        let available = usize::from(frame_width.saturating_sub(2));
        let chosen = self.fit(available);
        crate::text::record_overflow_text(self.name, &chosen, available);
        chosen
    }

    /// The longest form that fits `available` columns: the name whole, then shortened from its end
    /// and marked. **The position is never cut** (RFC 060 review v1 C1).
    fn fit(&self, available: usize) -> String {
        let whole = self.compose(self.name);
        if whole.chars().count() <= available {
            return whole;
        }
        // The name, shortened from its end and marked.
        let chars: Vec<char> = self.name.chars().collect();
        for keep in (1..chars.len()).rev() {
            let shortened: String = chars.iter().take(keep).collect::<String>() + "\u{2026}";
            let candidate = self.compose(&shortened);
            if candidate.chars().count() <= available {
                return candidate;
            }
        }
        // The position alone, whole. Below the minimum, which `shell` does not draw a frame at.
        self.compose("\u{2026}")
    }

    /// ` name — position `, with whichever parts there are.
    fn compose(&self, name: &str) -> String {
        let mut out = name.to_string();
        if let Some(position) = self.position {
            out.push_str(" \u{2014} ");
            out.push_str(&position.words());
        }
        format!(" {out} ")
    }
}

#[cfg(test)]
mod tests;
