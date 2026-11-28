//! Lines, the card on its scrim, the start edge, and the two buttons — with the floors Snora's own
//! defaults do not meet (RFC 069 rules 3, 4, 5 and 10).

use iced::widget::{Container, button, container, rule};
use iced::{Element, Length};
use snora::design::{Color, Tokens, contrast::contrast_ratio};

use crate::Overrides;

/// **Content is aligned to the start edge by this crate, and by no view** (rule 4).
///
/// RFC 065's R5 measured that Snora mirrors its regions under `LayoutDirection::Rtl` while the text
/// inside them stays where it was. So alignment is one function: when the right-to-left stage comes,
/// this is the body of code that changes, and no view has to be found.
///
/// **It is `Start`, not `Left`,** in the API and in the one call it makes: the vocabulary is the
/// thing that survives into the mirrored layout.
pub fn start<'a, Message>(content: impl Into<Element<'a, Message>>) -> Container<'a, Message> {
    container(content)
        .align_x(iced::alignment::Horizontal::Left)
        .width(Length::Fill)
}

/// **A line, not a panel** (rule 4): 1 px in the palette's border colour.
///
/// Lists are separated by lines. A raised surface means only *"this floats above the screen"*,
/// which in this app is a card on a scrim and nothing else (rule 10).
pub fn line<'a, Message: 'a>(tokens: &Tokens) -> Element<'a, Message> {
    let colour = iced_colour(tokens.palette.border);
    iced::widget::rule::horizontal(1)
        .style(move |_theme| rule::Style {
            color: colour,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
        .into()
}

/// **The room a scrollable reserves for its own bar** (rule 4), in logical pixels.
///
/// iced's `Scrollbar::default()` has `spacing: None`, and `Scrollable::layout` then falls through
/// to `layout(0.0, 0.0)` and paints the bar **over** the content
/// (`iced_widget-0.14.2/src/scrollable.rs:474-534`). RFC 065's M5 measured it covering the end of a
/// line at 200 %. Every scrollable in the app is given this, so no text is ever drawn under the bar.
///
/// `None` when [`Overrides::scrollbar_spacing`] is off, which is rule 4's failing run.
#[must_use]
pub fn scrollbar_spacing(tokens: &Tokens, overrides: Overrides) -> Option<f32> {
    overrides.scrollbar_spacing.then_some(tokens.spacing.sm)
}

/// **The shell and the card on its scrim** (rule 10, RFC 070 decision 3).
///
/// Snora's design-path renderer: it centres the dialog, paints the dim backdrop around it, and
/// draws the one card chrome — including the one shadow this app has (rule 10's exception). The
/// backdrop blocks the screen beneath for input and for the pointer, which is knotra's RFC-034
/// lesson.
pub fn render<'a, Message: Clone + 'a>(
    layout: snora::AppLayout<Element<'a, Message>, Message>,
    tokens: &Tokens,
) -> Element<'a, Message> {
    snora::design::render(layout, tokens)
}

/// How wide a card is drawn, in logical pixels: wide enough for a line of prikk's text to wrap
/// rather than be clipped, and narrow enough to stay a card at 1280 × 720 at 150 %.
#[must_use]
pub fn card_width() -> f32 {
    560.0
}

/// **The primary control: filled, and the strongest call to action on a surface** (rule 10's *one
/// primary action per area*).
///
/// `on_press` of `None` draws it unavailable — **readable, and the same shape** (rule 3). `id` is
/// what a focus step names when it puts focus here (rule 6).
pub fn button_primary<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    overrides: Overrides,
    id: &'static str,
    label: &'a str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let styling = tokens.clone();
    let high_contrast = is_high_contrast(tokens);
    snora::design::control(tokens, crate::control_label(tokens, label))
        .id(id)
        .on_press_maybe(on_press)
        .style(
            move |_theme, status| match (status, overrides.disabled_contrast) {
                (button::Status::Disabled, true) => disabled_filled(&styling, high_contrast),
                _ => snora::design::style::button::primary(&styling, status),
            },
        )
        .into()
}

/// **The secondary control: outlined.** The way out of a card, and never the control that acts.
pub fn button_secondary<'a, Message: Clone + 'a>(
    tokens: &Tokens,
    overrides: Overrides,
    id: &'static str,
    label: &'a str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let styling = tokens.clone();
    let high_contrast = is_high_contrast(tokens);
    snora::design::control(tokens, crate::control_label(tokens, label))
        .id(id)
        .on_press_maybe(on_press)
        .style(
            move |_theme, status| match (status, overrides.disabled_contrast) {
                (button::Status::Disabled, true) => disabled_outlined(&styling, high_contrast),
                _ => snora::design::style::button::secondary(&styling, status),
            },
        )
        .into()
}

/// **Rule 3's label floor for an unavailable control:** 4.5 : 1, and 7 : 1 in a high-contrast
/// preset.
pub(crate) fn label_floor(high_contrast: bool) -> f32 {
    if high_contrast { 7.0 } else { 4.5 }
}

/// **Rule 3's floor for a line, an outline or a badge against the page:** 3 : 1, and 4.5 : 1 in a
/// high-contrast preset.
pub(crate) fn mark_floor(high_contrast: bool) -> f32 {
    if high_contrast { 4.5 } else { 3.0 }
}

/// Which pair of presets these tokens belong to, read from the tokens themselves rather than passed
/// in, so a style function can never be given one preset's colours and another's floors.
///
/// The two high-contrast presets are the ones whose declared text level is 7 : 1 **against their own
/// surface** at every role, which is what `text_contrast` records.
pub(crate) fn is_high_contrast(tokens: &Tokens) -> bool {
    contrast_ratio(tokens.palette.text_muted, tokens.palette.background) >= 7.0
        && contrast_ratio(tokens.palette.border, tokens.palette.background) >= 4.5
}

/// **An unavailable filled control** (rule 3): still filled, so it gains no shape its available
/// form lacks, with a label that meets the floor against that fill.
///
/// Snora's own disabled style multiplies every alpha by 0.45
/// (`snora-style-0.65.0/src/button.rs`, `disabled_alpha`), which RFC 065's M4 measured at
/// 1.48–2.90 : 1. The replacement takes **no colour of stikk's own**: it chooses from the preset's
/// palette, by measuring, so a preset that changes carries its own answer with it.
fn disabled_filled(tokens: &Tokens, high_contrast: bool) -> button::Style {
    let p = &tokens.palette;
    let floor = label_floor(high_contrast);
    let fills = [p.border, p.surface_raised, p.text_muted];
    let labels = [p.background, p.surface, p.text_primary, p.text_secondary];
    let (fill, label) = first_pair(&fills, &labels, floor).unwrap_or((p.text_muted, p.background));
    button::Style {
        background: Some(iced_colour(fill).into()),
        text_color: iced_colour(label),
        border: iced::Border::default().rounded(tokens.radius.md),
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// **An unavailable outlined control** (rule 3): still outlined, at the same width, with a label
/// that meets the label floor and an outline that meets the mark floor — both against the page.
fn disabled_outlined(tokens: &Tokens, high_contrast: bool) -> button::Style {
    let p = &tokens.palette;
    let label = first_against(
        &[p.text_secondary, p.text_muted, p.text_primary],
        p.background,
        label_floor(high_contrast),
    )
    .unwrap_or(p.text_primary);
    let outline = first_against(
        &[p.border, p.text_muted, p.text_secondary, p.text_primary],
        p.background,
        mark_floor(high_contrast),
    )
    .unwrap_or(p.text_primary);
    button::Style {
        background: Some(iced::Color::TRANSPARENT.into()),
        text_color: iced_colour(label),
        border: iced::Border::default()
            .rounded(tokens.radius.md)
            .color(iced_colour(outline))
            .width(1.0),
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// The first `(fill, label)` pair whose contrast meets `floor`, trying each fill in turn.
fn first_pair(fills: &[Color], labels: &[Color], floor: f32) -> Option<(Color, Color)> {
    fills
        .iter()
        .find_map(|&fill| first_against(labels, fill, floor).map(|label| (fill, label)))
}

/// The first candidate whose contrast against `behind` meets `floor`.
fn first_against(candidates: &[Color], behind: Color, floor: f32) -> Option<Color> {
    candidates
        .iter()
        .copied()
        .find(|&c| contrast_ratio(c, behind) >= floor)
}

/// Snora's colour as iced's. **The one place the two types meet**, which is why no view may name
/// `iced::Color` (rule 1).
fn iced_colour(colour: Color) -> iced::Color {
    iced::Color::from_rgb(colour.r, colour.g, colour.b)
}

#[cfg(test)]
mod tests;
