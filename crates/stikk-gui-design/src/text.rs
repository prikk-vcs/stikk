//! **Text by purpose** (RFC 069 rule 2), and **text that is not words** (rule 12).
//!
//! A view calls [`body`], [`meta`], [`title`] or [`heading`]. **No view names a size**, so the
//! role table is one place and the H test can measure it on the rendered tree rather than by
//! scanning names — which is the check knotra's 212 size sites never had.
//!
//! **Bold, never Semibold** (rule 2): Noto Sans CJK has no SemiBold, so Japanese falls back to a
//! serif face. This module exposes [`strong`] and no weight between Regular and Bold.

use iced::widget::Text;
use iced::widget::text::{LineHeight, Shaping};
use snora::design::{TextRole, Tokens};

/// Every size the role table allows, in logical pixels, for the H test that measures the rendered
/// tree (rule 2). **Nothing is below 12**, and nothing a user must read in order to act is below
/// 16, which is why the list has no fifth value: there is no role between `meta` and `body`.
#[must_use]
pub fn role_sizes() -> [f32; 4] {
    let t = snora::design::Typography::default_roles();
    [t.body_small.size, t.body.size, t.title.size, t.heading.size]
}

/// **`body` (16 px): anything read in order to act or understand.** The primary text colour.
pub fn body<'a>(tokens: &Tokens, content: impl iced::widget::text::IntoFragment<'a>) -> Text<'a> {
    role(
        content,
        tokens.typography.body,
        tokens.palette.text_primary,
        tokens,
    )
}

/// **`body` in bold** — emphasis that never carries meaning alone (rule 2).
pub fn strong<'a>(tokens: &Tokens, content: impl iced::widget::text::IntoFragment<'a>) -> Text<'a> {
    body(tokens, content).font(iced::Font {
        weight: iced::font::Weight::Bold,
        ..super::interface_font()
    })
}

/// **`meta` (14 px): incidental metadata only** — times, counts, ids beside their words. The
/// secondary text colour, which every preset holds at or above 7 : 1 (RFC 065 M4).
pub fn meta<'a>(tokens: &Tokens, content: impl iced::widget::text::IntoFragment<'a>) -> Text<'a> {
    role(
        content,
        tokens.typography.body_small,
        tokens.palette.text_secondary,
        tokens,
    )
}

/// **`title` (18 px): a card's or a screen's own name.**
pub fn title<'a>(tokens: &Tokens, content: impl iced::widget::text::IntoFragment<'a>) -> Text<'a> {
    role(
        content,
        tokens.typography.title,
        tokens.palette.text_primary,
        tokens,
    )
}

/// **`heading` (24 px): a page or section heading.**
pub fn heading<'a>(
    tokens: &Tokens,
    content: impl iced::widget::text::IntoFragment<'a>,
) -> Text<'a> {
    role(
        content,
        tokens.typography.heading,
        tokens.palette.text_primary,
        tokens,
    )
}

/// **A control's own label**: `body`'s size and the interface font, and **no colour of its own**.
///
/// A control draws its label in the colour its style says, and a text that sets one overrides it —
/// which is how a filled button came out with the page's dark text on its accent fill, measured
/// from the pixels before it ever reached a window. So the one text in the app that names no colour
/// is this one, and the style names it instead.
pub fn control_label<'a>(
    tokens: &Tokens,
    content: impl iced::widget::text::IntoFragment<'a>,
) -> Text<'a> {
    let role = tokens.typography.body;
    iced::widget::text(content)
        .size(role.size)
        .line_height(LineHeight::Relative(role.line_height))
        .font(super::interface_font())
        .shaping(Shaping::Advanced)
}

/// **The isolating text function** (rule 12): one left-to-right piece, between U+2066 LEFT-TO-RIGHT
/// ISOLATE and U+2069 POP DIRECTIONAL ISOLATE.
///
/// **Why:** RFC 065's R4 measured an id inside Arabic text breaking into pieces without it. It
/// costs nothing in English, and it makes the later right-to-left stage a matter of words rather
/// than of finding every id in the app.
///
/// Every id, ref name, path, key id and line of prikk's text goes through this, and **is never
/// concatenated into a sentence by hand** — a sentence with a value in it is one template with the
/// isolated value placed into it.
#[must_use]
pub fn isolate(value: &str) -> String {
    format!("\u{2066}{value}\u{2069}")
}

/// An id, ref, path or key id **read in order to act**: `body`'s size, isolated (rule 12).
pub fn identifier<'a>(tokens: &Tokens, value: &str) -> Text<'a> {
    body(tokens, isolate(value))
}

/// An id, ref, path or key id **beside its own words**: `meta`'s size, isolated (rules 2 and 12).
pub fn meta_identifier<'a>(tokens: &Tokens, value: &str) -> Text<'a> {
    meta(tokens, isolate(value))
}

/// One role, applied: the size, its line height, its colour, and the interface font.
///
/// **`Shaping::Advanced` for every text, deliberately.** iced's default is `Auto`, which picks
/// basic shaping for an ASCII string and advanced shaping otherwise — so two labels in one screen
/// could be measured by two different shapers, and the label-fit guard would be measuring something
/// the next release's wording changes. Advanced is also what rule 12's isolates and the later
/// multilingual stage need.
fn role<'a>(
    content: impl iced::widget::text::IntoFragment<'a>,
    role: TextRole,
    colour: snora::design::Color,
    _tokens: &Tokens,
) -> Text<'a> {
    iced::widget::text(content)
        .size(role.size)
        .line_height(LineHeight::Relative(role.line_height))
        .color(iced::Color::from_rgb(colour.r, colour.g, colour.b))
        .font(super::interface_font())
        .shaping(Shaping::Advanced)
}

#[cfg(test)]
mod tests;
