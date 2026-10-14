//! One piece of a composed sentence (RFC 036 review v1 §2.3).

/// One piece of a sentence a view composes: **stikk's own words, or a name the repository chose**
/// (RFC 036 review v1 §2.3).
///
/// **Why core stops composing these sentences.** Every user-facing word is decided here so a GUI
/// says the same thing as the TUI — but *how* a repository name is rendered is the frontend's, and
/// the two rules it has (prose and identifier) differ in exactly the way that matters for a path or
/// a ref. A sentence returned as one `String` forced the frontend to choose one rule for the whole
/// of it, and the rule it had to choose was prose, which cannot tell `a  b.txt` from `a b.txt`.
///
/// So core returns the sentence's **structure** and keeps its words: `Said` for what stikk or prikk
/// wrote, `Name` for what the repository did. The frontend renders each piece under its own rule and
/// joins them. `stikk-tui`'s `text::compose` is that one function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    /// stikk's own words, or prikk's — rendered by the frontend's **prose** rule.
    Said(String),
    /// A name the repository chose: a path, a ref, an object id, or prikk's kind word where it tags
    /// a row. Rendered by the frontend's **identifier** rule, so that two different names can never
    /// look alike.
    Name(String),
}

/// The pieces' text joined, with neither rule applied — **for a test that is about the words, and
/// for a caller that genuinely only needs the bytes**.
///
/// Never for rendering: it throws away which pieces are names, which is the whole point of
/// [`Piece`]. A frontend renders with its own two rules (`stikk_tui::text::compose`).
#[must_use]
pub fn flatten(pieces: &[Piece]) -> String {
    pieces.iter().map(Piece::text).collect()
}

impl Piece {
    /// stikk's own words, borrowed.
    #[must_use]
    pub fn said(text: impl Into<String>) -> Self {
        Self::Said(text.into())
    }

    /// A repository name, borrowed.
    #[must_use]
    pub fn name(text: impl Into<String>) -> Self {
        Self::Name(text.into())
    }

    /// The piece's text, whichever kind it is. **For tests and for a caller that only needs the
    /// bytes** — never for rendering, which must distinguish the two.
    #[must_use]
    pub fn text(&self) -> &str {
        match self {
            Self::Said(text) | Self::Name(text) => text,
        }
    }
}
