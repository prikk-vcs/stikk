//! Lines, the card on its scrim, the start edge, and the two buttons — with the floors Snora's own
//! defaults do not meet (RFC 069 rules 3, 4, 5 and 10).

use iced::widget::{Container, button, container, rule, scrollable};
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

/// **A scrollable's own bar** (rule 3's mark floor, and rule 4's room).
///
/// **The thumb is a mark, not decoration:** it is what tells a reader where they are in a long
/// card and how much is left. iced's own measured **1.45 : 1** against its track and **1.56 : 1**
/// against the card, in `dark_aaa` at 200 %, read off a real capture.
///
/// **The rail is drawn with no background of its own**, so the track *is* the surface behind it.
/// That is one less box on a screen whose rule is *lines, not panels* (rule 4), and it makes the
/// floor one measurement instead of two: the thumb against its track and the thumb against the
/// surface are the same pair.
///
/// **The thumb's colour is chosen from the preset's own palette by measuring**, as the unavailable
/// controls' is, and it must clear the floor against **all three** surfaces it can be drawn on —
/// the page, a card, and a raised one — because one style serves every scrollable in the app.
///
/// With [`Overrides::scrollbar_contrast`] off this is iced's own style, which is rule 3's failing
/// run.
pub fn scrollbar(
    tokens: &Tokens,
    overrides: Overrides,
    theme: &iced::Theme,
    status: scrollable::Status,
) -> scrollable::Style {
    if !overrides.scrollbar_contrast {
        return scrollable::default(theme, status);
    }
    let hovered = matches!(
        status,
        scrollable::Status::Hovered {
            is_vertical_scrollbar_hovered: true,
            ..
        } | scrollable::Status::Dragged {
            is_vertical_scrollbar_dragged: true,
            ..
        }
    );
    let rail = rail(tokens, hovered);
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::default(theme, status).auto_scroll,
    }
}

/// One rail: no background of its own, and a thumb that clears the mark floor.
fn rail(tokens: &Tokens, hovered: bool) -> scrollable::Rail {
    let p = &tokens.palette;
    let floor = mark_floor(is_high_contrast(tokens));
    // Every surface the bar can be drawn on. The thumb clears the floor against all of them,
    // because one style serves the page, a card and a raised card alike.
    let behind = [p.background, p.surface, p.surface_raised];
    // Idle first, then a stronger one for the hovered and dragged states; a state that finds
    // nothing stronger keeps the idle colour, which still clears the floor.
    let idle = [p.text_muted, p.text_secondary, p.border, p.text_primary];
    let stronger = [p.text_primary, p.text_secondary, p.text_muted, p.border];
    let candidates: &[Color] = if hovered { &stronger } else { &idle };
    let thumb = candidates
        .iter()
        .copied()
        .find(|&c| behind.iter().all(|&b| contrast_ratio(c, b) >= floor))
        .unwrap_or(p.text_primary);
    scrollable::Rail {
        background: None,
        border: iced::Border::default(),
        scroller: scrollable::Scroller {
            background: iced_colour(thumb).into(),
            border: iced::Border::default().rounded(tokens.radius.sm),
        },
    }
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

/// **How wide a card is drawn, at most**, in logical pixels: wide enough for a line of prikk's
/// text to wrap rather than be clipped, and narrow enough to stay a card rather than a screen.
///
/// **At most, not exactly.** A fixed width is a layout that depends on the window being big
/// enough for it, and rule 4 forbids that: at 800 × 600 at 150 % the viewport is 533 logical
/// pixels wide and a 560-wide card hangs off the end of it (measured, 2026-11-28). The card takes
/// the room it is given, up to this.
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

/// **Rule 3's floor for content text:** 7 : 1 in every preset.
#[must_use]
pub fn text_floor() -> f32 {
    7.0
}

/// **Rule 3's label floor for an unavailable control:** 4.5 : 1, and 7 : 1 in a high-contrast
/// preset.
pub fn label_floor(high_contrast: bool) -> f32 {
    if high_contrast { 7.0 } else { 4.5 }
}

/// **Rule 3's floor for a line, an outline or a badge against the page:** 3 : 1, and 4.5 : 1 in a
/// high-contrast preset.
pub fn mark_floor(high_contrast: bool) -> f32 {
    if high_contrast { 4.5 } else { 3.0 }
}

/// Which pair of presets these tokens belong to, read from the tokens themselves rather than passed
/// in, so a style function can never be given one preset's colours and another's floors.
///
/// The two high-contrast presets are the ones whose declared text level is 7 : 1 **against their own
/// surface** at every role, which is what `text_contrast` records.
pub fn is_high_contrast(tokens: &Tokens) -> bool {
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
