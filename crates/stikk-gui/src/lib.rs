//! **stikk's desktop app** (RFC 070), built to RFC 069's rules.
//!
//! It draws `stikk_app::App` — the one program both front ends drive (RFC 067) — and **adds no
//! behaviour of its own**. Everything it shows, every key it takes and every reading it reports
//! goes through the program's own types, exactly as the terminal does.
//!
//! # The seam, in four pieces
//!
//! | Piece | Where | Rule |
//! |---|---|---|
//! | iced's key events to `stikk_app::Intent` | [`keys::dispatch`] | 6, 7 |
//! | what the drawing measured, to `stikk_app::Reading` | [`reading::Report`] | 8 |
//! | the program's state, in one line, for a run's record | [`log::state_line`] | — |
//! | the look | the `stikk-gui-design` crate | 1 |
//!
//! # What this crate never does
//!
//! - **It names no colour, size, font or Snora style.** Those are `stikk-gui-design`'s, and a
//!   source check holds it (rule 1's test).
//! - **It opens no card that writes.** The writing cards come in the release that publishes the
//!   app; until then no path in the desktop app changes a repository (RFC 070's *What this RFC does
//!   not do*).
//! - **It is not published.** `publish = false`, and the release builds `-p stikk`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(test)]
pub mod harness;
#[cfg(test)]
mod rules;

pub mod card;
pub mod keys;
pub mod log;
pub mod reading;
pub mod shell;
pub mod text;
pub mod theme;
pub mod view;
pub mod words;

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use iced::keyboard::{self, Modifiers};
use iced::widget::scrollable;
use iced::{Element, Subscription, Task};
use stikk_app::{App, Direction, Effect, Intent, Reading, Request, Response, Session, Surface};
use stikk_core::Key as CatalogKey;
use stikk_gui_design::{Overrides, Preset};

/// The widget id of the card's own frame, so a measurement can tell the card from the page the
/// scrim covers behind it.
pub const CARD_FRAME: &str = "card-frame";
/// The widget id of the card's way out — the control a card's focus settles on (rule 6).
pub const CARD_EXIT: &str = "card-exit";
/// The widget id of the scrollable a card's text is drawn in, which `Effect::Scroll` moves.
pub const CARD_SCROLL: &str = "card-scroll";
/// The widget id of the scrollable a screen's body is drawn in.
pub const BODY_SCROLL: &str = "body-scroll";

/// **How far one `Effect::Scroll` moves a card**, in logical pixels: one line of `body` text at its
/// own line height, which is the terminal's one row.
fn line_height(tokens: &stikk_gui_design::Tokens) -> f32 {
    tokens.typography.body.size * tokens.typography.body.line_height
}

/// **What a launch decided**, before any window exists.
///
/// Everything the app reads from its environment is read once, here, by whoever launches it — so
/// the program value itself reads nothing and every test can state the whole launch in one
/// literal (the lesson RFC 059's `theme::Environment` wrote down for the terminal).
#[derive(Debug, Clone)]
pub struct Settings {
    /// The repository to open: the working directory, or the path given on the command line.
    pub repo: PathBuf,
    /// Where this repository may be remembered, or `None` for a launch that remembers nothing.
    pub session: Option<Session>,
    /// The preset choice the user's configuration asked for.
    pub theme: stikk_state::config::Theme,
    /// iced's scale factor: stikk's zoom. **Handoff A has no key that changes it** (the zoom keys
    /// are Handoff B's); the tests and the probe set it to lay out at 150 % or 250 %.
    pub zoom: f32,
    /// stikk's overrides over Snora's defaults. A shipped build passes [`Overrides::stikk`]; a test
    /// turns one off to show the guard that rule names failing.
    pub overrides: Overrides,
    /// **An opening script: catalog keys to run, one per beat, through the ordinary message path.**
    ///
    /// A measured run has to reach a screen or a card, and the owner's GUI rules send keys only
    /// when the keyboard is what is being measured — for a layout or a reading sequence it is not.
    /// Each step leaves through the same `Message::Press` a pointer press produces, so the state
    /// log records it exactly as it records a key, and a run can assert the card it reached.
    ///
    /// **A shipped launch passes none**, and the binary has no way to set it.
    pub opening: Vec<CatalogKey>,
}

impl Settings {
    /// A launch that opens `repo` with everything else at its default.
    #[must_use]
    pub fn opening(repo: impl Into<PathBuf>) -> Self {
        Self {
            repo: repo.into(),
            session: None,
            theme: stikk_state::config::Theme::default(),
            zoom: 1.0,
            overrides: Overrides::stikk(),
            opening: Vec::new(),
        }
    }
}

/// Every message the desktop app can receive.
#[derive(Debug, Clone)]
pub enum Message {
    /// A key press, from iced's own event loop.
    Key(keyboard::Key, Modifiers, bool),
    /// The program's clock, for its silent change check (RFC 031 §5), and the beat on which the
    /// worker's answers are taken up.
    Tick,
    /// A card's or a screen's viewport moved, for the surface it was drawn for.
    Scrolled(Surface, scrollable::Viewport),
    /// A surface's first line was laid out: it is on screen.
    TopShown(Surface),
    /// A surface's last line came into view, or left it.
    EndVisible(Surface, bool),
    /// A control was pressed with the pointer: the same catalog key a key press would run.
    Press(CatalogKey),
    /// The window's own logical size, from the window (rule 8).
    WindowSize(iced::Size),
    /// The display's own scale factor, from the window (rule 8).
    ScaleFactor(f32),
    /// The size the shell's root was laid out at (rule 8).
    LaidOut(iced::Size),
    /// **What the desktop says about light or dark** (rule 3), from iced itself.
    SystemTheme(iced::theme::Mode),
    /// A message with no effect, so a subscription can map an event it does not use.
    Ignored,
}

/// **The desktop app's own state**: the program, the worker, and what the drawing has learned.
pub struct Stikk {
    app: App,
    responses: mpsc::Receiver<Response>,
    worker_alive: bool,
    choice: theme::Choice,
    overrides: Overrides,
    zoom: f32,
    viewport: reading::Viewport,
    report: reading::Report,
    log: log::Log,
    /// Where focus is, by widget id — settled after every update and by nothing else (rule 6).
    focus: Option<&'static str>,
    /// Where focus was before the card on top opened, to return it when that card closes (rule 6).
    focus_before_card: Vec<Option<&'static str>>,
    /// What is left of the opening script, taken one step per beat.
    opening: std::collections::VecDeque<CatalogKey>,
}

impl Stikk {
    /// **Open a repository and start the worker.**
    ///
    /// Two tasks go out at once, and both are the window answering for itself (rule 8): its own
    /// size, and the desktop's light-or-dark setting.
    pub fn new(settings: Settings) -> (Self, Task<Message>) {
        let (req_tx, req_rx) = mpsc::channel::<Request>();
        let (res_tx, res_rx) = mpsc::channel::<Response>();
        let repo = settings.repo.clone();
        std::thread::spawn(move || {
            let backend = stikk_prikk::CliBackend::new();
            // Caught here, not propagated: a worker panic must surface as a fault screen (ER-04),
            // never a crash of the window. Either way `res_tx` is dropped when this closure
            // returns, which the drain observes as a disconnected channel.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                stikk_app::run(&backend, &repo, req_rx, res_tx);
            }));
        });
        let app = App::open(settings.repo, settings.session, req_tx);
        let stikk = Self {
            app,
            responses: res_rx,
            worker_alive: true,
            choice: theme::Choice::from_config(settings.theme),
            overrides: settings.overrides,
            zoom: settings.zoom,
            viewport: reading::Viewport::new(settings.zoom),
            report: reading::Report::default(),
            log: log::Log::open(),
            focus: None,
            focus_before_card: Vec::new(),
            opening: settings.opening.into_iter().collect(),
        };
        (
            stikk,
            Task::batch([
                // **Ask the window how big it really is, and at what scale** — not what stikk
                // asked for at launch. Rule 8 reports nothing until both have answered.
                iced::window::latest()
                    .and_then(iced::window::size)
                    .map(Message::WindowSize),
                iced::window::latest()
                    .and_then(iced::window::scale_factor)
                    .map(Message::ScaleFactor),
                // **And what the desktop says about light or dark** (rule 3). `iced_test`'s
                // emulator drops `Action::System` with a `TODO` (`emulator.rs:262-265`), so this
                // never resolves headless — which is why rule 3's H test goes through the message
                // instead (RFC 070 F4).
                iced::system::theme().map(Message::SystemTheme),
            ]),
        )
    }

    /// The tokens the app is drawing with.
    #[must_use]
    pub fn tokens(&self) -> stikk_gui_design::Tokens {
        stikk_gui_design::tokens(self.preset(), self.overrides)
    }

    /// The preset the app is drawing with (rule 3).
    #[must_use]
    pub fn preset(&self) -> Preset {
        self.choice.preset()
    }

    /// The program, for a test that asserts its state after a key.
    #[must_use]
    pub fn app(&self) -> &App {
        &self.app
    }

    /// Where focus is, by widget id — what rule 6's focus-return test reads.
    #[must_use]
    pub fn focus(&self) -> Option<&'static str> {
        self.focus
    }

    /// What the front end knows about the window it is drawing in (rule 8).
    #[must_use]
    pub fn viewport(&self) -> &reading::Viewport {
        &self.viewport
    }

    /// Who decided the preset that is drawing (rule 3).
    #[must_use]
    pub fn theme_choice(&self) -> theme::Choice {
        self.choice
    }

    /// One state line, as the log writes it — so a test asserts exactly what a run records.
    #[must_use]
    pub fn state_line(&self) -> String {
        log::state_line(&self.app, &self.viewport, &self.report)
    }

    /// **One message.**
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let label = log::label(&message, self.preset());
        let layer_before = self.app.top_card().map(|(_, surface)| surface);
        let task = self.apply(message);
        // **Focus settles here, and nowhere else** (rule 6): one step, after the update, comparing
        // the top layer before and after. No open or close site touches focus.
        let focus_task = self.settle_focus(layer_before);
        if self.app.should_quit() {
            return iced::exit();
        }
        if self.log.is_open() {
            self.log
                .record(&label, &self.app, &self.viewport, &self.report);
        }
        Task::batch([task, focus_task])
    }

    /// The message itself.
    fn apply(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Key(key, modifiers, _repeat) => self.key(&key, modifiers),
            Message::Tick => {
                self.app.tick(Instant::now());
                self.drain();
                // One step per beat, so each is applied to a program that has reacted to the one
                // before it.
                match self.opening.pop_front() {
                    Some(key) => Task::done(Message::Press(key)),
                    None => Task::none(),
                }
            }
            Message::Scrolled(surface, viewport) => {
                let above = viewport.absolute_offset().y > 0.5;
                self.measure(surface, None, Some(above));
                Task::none()
            }
            Message::TopShown(surface) => {
                self.measure(surface, None, None);
                Task::none()
            }
            Message::EndVisible(surface, visible) => {
                self.measure(surface, Some(visible), None);
                Task::none()
            }
            Message::Press(key) => {
                if let Some(action) = stikk_core::actions::by_key(key) {
                    let _ = self.app.handle(Intent::Run(action));
                }
                Task::none()
            }
            Message::WindowSize(size) => {
                self.viewport.window = Some(size);
                Task::none()
            }
            Message::ScaleFactor(scale) => {
                self.viewport.scale = Some(scale);
                Task::none()
            }
            Message::LaidOut(size) => {
                self.viewport.laid_out = Some(size);
                Task::none()
            }
            Message::SystemTheme(mode) => {
                let _ = self.choice.desktop_says(mode);
                Task::none()
            }
            Message::Ignored => Task::none(),
        }
    }

    /// **One key press, through the program** (rules 6 and 7).
    ///
    /// Snora's own keyboard comes first and stikk adds no model beside it: Tab moves between
    /// controls, F6 between the frame's zones. What is left is a catalog key.
    fn key(&mut self, key: &keyboard::Key, modifiers: Modifiers) -> Task<Message> {
        if let Some(task) = snora::keyboard::traverse(key.clone(), modifiers) {
            return task;
        }
        if let Some(cycle) = snora::keyboard::cycle_zones(key.clone(), modifiers) {
            return snora::keyboard::focus_zone(cycle);
        }
        let intent = keys::dispatch(key, modifiers, self.app.wants_text_input());
        if matches!(intent, Intent::None) {
            return Task::none();
        }
        let scrolls = self.app.top_card().is_some();
        match self.app.handle(intent) {
            Effect::Scroll { direction, .. } => {
                let step = match direction {
                    Direction::Up => -line_height(&self.tokens()),
                    Direction::Down => line_height(&self.tokens()),
                };
                let id = if scrolls { CARD_SCROLL } else { BODY_SCROLL };
                iced::widget::operation::scroll_by(
                    id,
                    scrollable::AbsoluteOffset { x: 0.0, y: step },
                )
            }
            _ => Task::none(),
        }
    }

    /// **The one place a `Reading` reaches the program** (rule 8).
    fn measure(&mut self, surface: Surface, end_visible: Option<bool>, above: Option<bool>) {
        if let Some(reading) = self
            .report
            .measure(&self.viewport, surface, end_visible, above)
        {
            self.app.report_reading(reading);
        }
    }

    /// **Where focus goes after an update** (rule 6).
    ///
    /// A card that reaches the top takes focus on its own entry — which is **never a control that
    /// starts a change**: in this handoff every card is read-only, so the entry is the way out. A
    /// card that closes gives focus back to wherever it was before it opened.
    fn settle_focus(&mut self, before: Option<Surface>) -> Task<Message> {
        let after = self.app.top_card().map(|(_, surface)| surface);
        if before == after {
            return Task::none();
        }
        // **The entry the card on top actually has**, which for a card that draws no control is
        // none: focus then settles nowhere, and every catalog key keeps reaching the program.
        let entry = self
            .app
            .top_card()
            .and_then(|(overlay, _)| card::entry_of(overlay));
        match (before, after) {
            (None, Some(_)) => {
                self.focus_before_card.push(self.focus);
                self.focus = entry;
                Self::focus_on(entry)
            }
            (Some(_), None) => {
                self.focus = self.focus_before_card.pop().flatten();
                Self::focus_on(self.focus)
            }
            // One card replaced by another: the new card's entry, and the focus to return to is
            // still the one the first card remembered.
            (Some(_), Some(_)) => {
                self.focus = entry;
                Self::focus_on(entry)
            }
            (None, None) => Task::none(),
        }
    }

    /// Put focus on a widget, or on nothing.
    fn focus_on(id: Option<&'static str>) -> Task<Message> {
        match id {
            Some(id) => iced::widget::operation::focus(id),
            None => Task::none(),
        }
    }

    /// Take up whatever the worker has answered.
    fn drain(&mut self) {
        if !self.worker_alive {
            return;
        }
        loop {
            match self.responses.try_recv() {
                Ok(response) => self.app.apply(response),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.app.worker_stopped();
                    self.worker_alive = false;
                    break;
                }
            }
        }
    }

    /// The events the app listens to.
    ///
    /// **The clock is the prototype's** (RFC 065 M9), and so is its reason: `stikk_app::run` answers
    /// on a `std::sync::mpsc` channel, which is not a stream, so the answers are taken up on a beat
    /// rather than woken by the channel. 50 ms is what RFC 065 measured as a window that feels
    /// immediate; it is also the beat `App::tick` needs for its own 5-second change check.
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            keyboard::listen().map(|event| match event {
                keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    repeat,
                    ..
                } => Message::Key(key, modifiers, repeat),
                _ => Message::Ignored,
            }),
            iced::time::every(Duration::from_millis(50)).map(|_| Message::Tick),
            // Every resize, so no record can quote a size the window no longer had — and so rule
            // 8's confirmation is withdrawn the moment the window changes under it.
            iced::window::resize_events().map(|(_, size)| Message::WindowSize(size)),
            // **While the app is open, too** (rule 3, amended 2026-11-28).
            iced::system::theme_changes().map(Message::SystemTheme),
        ])
    }

    /// The whole window.
    #[must_use]
    pub fn view(&self) -> Element<'_, Message> {
        shell::view(self)
    }

    /// The preset's `iced::Theme`, so stock iced widgets and the window's background follow it.
    #[must_use]
    pub fn theme(&self) -> iced::Theme {
        stikk_gui_design::theme(self.preset(), self.overrides)
    }

    /// stikk's zoom, as iced's scale factor.
    #[must_use]
    pub fn scale_factor(&self) -> f32 {
        self.zoom
    }

    /// The window's title.
    #[must_use]
    pub fn title(&self) -> String {
        words::Say::Header.fill(&[(
            "repository",
            &text::identifier(&self.app.repo().display().to_string()),
        )])
    }

    /// **The app in an explicit state, with no worker and no prikk** — tests only.
    ///
    /// The headless level has to run on all three CI runners, where prikk is not installed, so
    /// every fixture is built with `App::from_state` and answered with `App::apply`. That is what
    /// `stikk-app`'s `test-fixtures` feature exists for, and what `stikk-tui`'s own tests do.
    #[cfg(test)]
    #[must_use]
    pub fn for_test(app: App, settings: &Settings) -> Self {
        Self {
            app,
            responses: mpsc::channel().1,
            worker_alive: false,
            choice: theme::Choice::from_config(settings.theme),
            overrides: settings.overrides,
            zoom: settings.zoom,
            viewport: reading::Viewport::new(settings.zoom),
            report: reading::Report::default(),
            log: log::Log::open(),
            focus: None,
            focus_before_card: Vec::new(),
            opening: settings.opening.iter().copied().collect(),
        }
    }

    /// **A reading offered to the program with no condition at all** — the shape a front end that
    /// reports at once has, which is what rule 8's condition refuses. Tests only.
    #[cfg(test)]
    pub fn report_reading_without_the_condition(&mut self, reading: Reading) {
        self.app.report_reading(reading);
    }

    /// Put a card on the stack directly, through the program's own test fixture.
    #[cfg(test)]
    pub fn push_card_for_test(&mut self, overlay: stikk_app::Overlay) {
        self.app.push_overlay_for_test(overlay);
    }

    /// Close the top card through the program alone, with no focus settling — the counterfactual
    /// rule 6's failing run needs, and the shape an open or close site touching focus would have.
    #[cfg(test)]
    pub fn close_card_without_settling_for_test(&mut self) {
        self.app.close_overlay();
    }

    /// Route an error through the program, so a test can draw the banner it sets — tests only.
    #[cfg(test)]
    pub fn surface_error_for_test(
        &mut self,
        error: &stikk_model::StikkError,
        op: stikk_core::present::OperationContext,
    ) {
        self.app.surface_error(error, op);
    }

    /// The next step of the opening script, for a harness that performs the app's tasks itself.
    #[cfg(test)]
    pub fn next_opening_step(&mut self) -> Option<CatalogKey> {
        self.opening.pop_front()
    }

    /// What the overrides are, for the tests that build a tree with one of them off.
    #[must_use]
    pub fn overrides(&self) -> Overrides {
        self.overrides
    }

    /// A reading the drawing would have reported, for a test that needs to offer one directly.
    ///
    /// **Not a second path into the program**: it goes through the same [`reading::Report`], so the
    /// viewport condition applies to it exactly as it applies to a sensor's own message.
    pub fn offer_reading(&mut self, reading: Reading) {
        self.measure(
            reading.surface,
            Some(reading.at_end),
            Some(reading.more_above),
        );
    }
}

/// **The one program value** — the binary runs it, and the headless tests drive this same value
/// through `iced_test`'s emulator, so the thing measured headless cannot drift from the thing a
/// window runs.
pub fn program(
    settings: Settings,
) -> iced::application::Application<
    impl iced::Program<State = Stikk, Message = Message, Theme = iced::Theme>,
> {
    let launch = settings.clone();
    iced::application(
        move || Stikk::new(launch.clone()),
        Stikk::update,
        Stikk::view,
    )
    .title(Stikk::title)
    .subscription(Stikk::subscription)
    .theme(Stikk::theme)
    .scale_factor(Stikk::scale_factor)
    // **The interface font, named** (rule 2, the owner's ruling B). Unset, iced draws Open Sans
    // and not the user's own choice; `stikk_gui_design::interface_font` says why.
    .default_font(stikk_gui_design::interface_font())
    .window(window_settings())
}

/// The window's settings, in one place.
///
/// - **1280 × 720 logical at launch**, the owner's GUI rule, asked for through the app's own
///   setting and never by resizing a running window (RFC 070 decision 2);
/// - **a minimum size** at which rule 4's guard passes at 100 %, which is the smaller of the two
///   sizes the H matrix runs;
/// - **the app id on Linux only.** It is a Wayland and X11 concept, and `PlatformSpecific` is a
///   different struct per platform: on Windows it has no `application_id` at all, so naming the
///   field unconditionally stops the Windows build (found by RFC 065's own cross build).
#[must_use]
pub fn window_settings() -> iced::window::Settings {
    #[allow(unused_mut)]
    let mut settings = iced::window::Settings {
        size: iced::Size::new(1280.0, 720.0),
        min_size: Some(iced::Size::new(800.0, 600.0)),
        ..Default::default()
    };
    #[cfg(target_os = "linux")]
    {
        settings.platform_specific.application_id = String::from(APP_ID);
    }
    settings
}

/// **The window's `app_id`.**
///
/// RFC 071 decision 5 wants one id everywhere — the `.desktop` file's name, the window's Wayland
/// `app_id`, and later the stores' — because a Wayland desktop finds a window's icon by matching
/// the two. The id itself is RFC 071's (`io.github.prikk_vcs.stikk`); it is written here because
/// the window is the thing that carries it.
pub const APP_ID: &str = "io.github.prikk_vcs.stikk";

/// Open `settings`' repository in a window.
pub fn run(settings: Settings) -> iced::Result {
    program(settings).run()
}
