//! **The reading contract's front-end half** (RFC 069 rule 8, RFC 070 decision 5).
//!
//! The program owns the latch: `stikk_app` mints a new [`Surface`] for every card that reaches the
//! top, and `seen` belongs to the surface (RFC 067 Handoff C). **The front end's half is to report
//! a [`Reading`] only for what it laid out, in a viewport it has confirmed from the window
//! itself** — and to report nothing until then.
//!
//! **Why the condition exists.** RFC 065's W2 measured a card reporting its end visible on
//! arrival, on Windows, before the window's scaled size had arrived. iced's layout, its sensors and
//! its window size all read one viewport, so the evidence could not show whether that viewport was
//! the window on screen. The fix is not to trust the first frame: nothing is reported until the
//! window has answered for itself.
//!
//! It is the desktop counterpart of 0.22.1's terminal fix, where a surface that was never drawn
//! reports nothing.

use stikk_app::{Reading, Surface};

/// **What the front end knows about the window it is drawing in.**
///
/// Three facts, from three places, and none of them stikk's own request:
/// - `window` — the window's logical size, from `iced::window::size`, which `iced_winit` answers
///   from the window's own viewport (`window/state.rs:98-100`), not from the size the app asked
///   for at launch;
/// - `scale` — the display's scale factor, from `iced::window::scale_factor`, which `iced_winit`
///   answers from winit's own `scale_factor()` (`lib.rs:1546-1552`);
/// - `laid_out` — the size the shell's own root was given, from the sensor around it, which is the
///   viewport the app actually laid out against.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Viewport {
    /// The window's logical size, as the window reports it.
    pub window: Option<iced::Size>,
    /// The display's scale factor, as the window reports it.
    pub scale: Option<f32>,
    /// The size the shell's root was laid out at.
    pub laid_out: Option<iced::Size>,
    /// The app's own zoom, which stikk sets and so may not confirm anything by itself.
    pub app_scale: f32,
}

impl Viewport {
    /// A viewport that knows nothing yet, at the given zoom.
    #[must_use]
    pub fn new(app_scale: f32) -> Self {
        Self {
            window: None,
            scale: None,
            laid_out: None,
            app_scale,
        }
    }

    /// **Whether the viewport is confirmed** (rule 8): all three facts are in, and the size the app
    /// laid out against is the size the window says it has.
    ///
    /// The comparison has a one-pixel tolerance, because the two numbers travel different paths —
    /// the window's is `physical / scale` in floating point, and the root's comes out of the
    /// layout — and a sub-pixel difference is not a disagreement about which window this is.
    #[must_use]
    pub fn confirmed(&self) -> bool {
        match (self.window, self.scale, self.laid_out) {
            (Some(window), Some(_), Some(laid_out)) => {
                (window.width - laid_out.width).abs() <= 1.0
                    && (window.height - laid_out.height).abs() <= 1.0
            }
            _ => false,
        }
    }

    /// The window's physical size, derived: the logical size times both scales.
    ///
    /// `iced::window::size` is already divided by the app's own zoom
    /// (`iced_winit`'s viewport is built with `window.scale_factor() * app scale_factor`), so both
    /// go back in here. Recorded rather than used: it is what the owner's rules ask a run to state.
    #[must_use]
    pub fn physical(&self) -> Option<iced::Size> {
        let window = self.window?;
        let scale = self.scale?;
        Some(iced::Size::new(
            window.width * scale * self.app_scale,
            window.height * scale * self.app_scale,
        ))
    }

    /// One line naming every number, for the state log and for the C test's record.
    #[must_use]
    pub fn line(&self) -> String {
        let size = |s: Option<iced::Size>| match s {
            Some(s) => format!("{}x{}", s.width, s.height),
            None => "?".to_string(),
        };
        format!(
            "window {} scale {} app_scale {} laid_out {} physical {} confirmed {}",
            size(self.window),
            self.scale
                .map_or_else(|| "?".to_string(), |s| s.to_string()),
            self.app_scale,
            size(self.laid_out),
            size(self.physical()),
            self.confirmed(),
        )
    }
}

/// **The one place a `Reading` is made** (RFC 070 decision 5).
///
/// Every card and every scrolling screen reports through [`Report::measure`]. It holds what a
/// drawing learned about the surface on top — whether its end is in view, and whether anything is
/// scrolled off its top — and it carries rule 8's condition, so there is no second path that could
/// report without it.
#[derive(Debug, Clone, Copy, Default)]
pub struct Report {
    measured: Option<(Surface, bool, bool)>,
}

impl Report {
    /// **What to tell the program, if anything.**
    ///
    /// `end_visible` and `more_above` are each `Some` only when this message carried that fact, so
    /// a scroll that moves the top does not claim anything about the end, and a sensor that fires
    /// for the end does not claim anything about the top.
    ///
    /// **`None` until the viewport is confirmed.** Nothing is remembered either: a measurement
    /// taken before the window answered is a measurement of a layout that may not have been on
    /// screen, and keeping it would only let it be reported one message later.
    ///
    /// Each report names the surface it was measured for, so a late event from a card that has
    /// closed names a surface that is no longer current and the program ignores it.
    pub fn measure(
        &mut self,
        viewport: &Viewport,
        surface: Surface,
        end_visible: Option<bool>,
        more_above: Option<bool>,
    ) -> Option<Reading> {
        if !viewport.confirmed() {
            return None;
        }
        let (mut at_end, mut above) = match self.measured {
            Some((s, e, a)) if s == surface => (e, a),
            _ => (false, false),
        };
        if let Some(v) = end_visible {
            at_end = v;
        }
        if let Some(a) = more_above {
            above = a;
        }
        self.measured = Some((surface, at_end, above));
        Some(Reading {
            surface,
            more_above: above,
            at_end,
        })
    }

    /// What the drawing last measured, for the state log.
    #[must_use]
    pub fn measured(&self) -> Option<(Surface, bool, bool)> {
        self.measured
    }
}

#[cfg(test)]
mod tests;
