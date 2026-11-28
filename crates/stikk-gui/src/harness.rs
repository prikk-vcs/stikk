//! **The headless harness** (RFC 070 decision 7, level H): the app's own `view` and `update`,
//! measured without a screen.
//!
//! **Two instruments, because `iced_test` 0.14 needs both** (RFC 065 F4, measured):
//! - **`Simulator`** keeps the messages a draw produces, so the measuring sensors' reports reach
//!   the program — but it owns the widget state tree for one step only, so a scroll offset cannot
//!   survive from step to step.
//! - **`Emulator`** keeps the state tree and runs the program's tasks, and its `screenshot` is the
//!   one way to the rendered pixels (`window::Screenshot::rgba`) — but that same call passes
//!   `&mut Vec::new()` as its message sink (`emulator.rs:465`, its own *"TODO: Nested redraws!"*),
//!   so every sensor report it produces is thrown away.
//!
//! So the state and the reading seam are driven through [`Screen`], which holds the real [`Stikk`]
//! and draws it through a `Simulator` each step, and the pixels come from [`picture`], which draws
//! the same view at the same size through an `Emulator`. Nothing is written by hand: the program is
//! told what a real layout of its own view measured.
//!
//! **No prikk, and no worker.** Every fixture is built with `App::from_state` and answered with
//! `App::apply`, which is what `stikk-app`'s `test-fixtures` feature is for and what `stikk-tui`'s
//! own tests do. That is also what lets this level run on all three CI runners.
//!
//! **Zoom is modelled by the layout size, not by a scale factor** (`logical`): iced's scale factor
//! divides the window's logical size, and both harnesses take a logical size directly. 1280 × 720
//! at 150 % is 853 × 480, which is the same layout the window produces — and the real
//! `scale_factor` is exercised by the window probe and the R runs.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use iced::keyboard::{Key as IcedKey, Modifiers, key::Named};
use iced::{Rectangle, Size};
use iced_test::futures::futures::channel::mpsc as test_mpsc;
use iced_test::{Simulator, emulator};
use stikk_app::{App, OrientationState, Request, Response, ResponseKind};
use stikk_core::Key as CatalogKey;
use stikk_gui_design::{Overrides, Preset};

use crate::{Message, Settings, Stikk};

/// The owner's window size, in logical pixels.
pub const WINDOW: Size = Size::new(1280.0, 720.0);
/// The smaller of the two sizes the H matrix runs — the app's own minimum.
pub const SMALL: Size = Size::new(800.0, 600.0);

/// **One rendering at a time, in this process.**
///
/// RFC 065's F4 recorded that `iced_test` 0.14 manages **one `Emulator` capture per process**, from
/// two upstream defects. Measured here: with `cargo test`'s default thread pool the test binary
/// dies with `SIGSEGV` part-way through the matrix, and the same tests pass one after another. The
/// renderer behind both harnesses — and the one global font system they share — is not safe to
/// drive from two threads at once.
///
/// So every draw and every screenshot takes this lock. Tests still run in parallel; only the
/// drawing is serialised, which is what the harness can honestly promise. **A poisoned lock is
/// taken anyway** (`into_inner`): one test panicking inside a draw must fail that test, not turn
/// every other one into a second failure with an unrelated message.
fn rendering() -> std::sync::MutexGuard<'static, ()> {
    static RENDERING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    RENDERING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The same lock, for a measurement that draws a specimen of its own rather than a screen.
pub fn rendering_for_a_specimen() -> std::sync::MutexGuard<'static, ()> {
    rendering()
}

/// A window size at a zoom, as iced lays it out: the logical size the app actually gets.
#[must_use]
pub fn logical(window: Size, zoom: f32) -> Size {
    Size::new(window.width / zoom, window.height / zoom)
}

/// Every cell of the H matrix: four presets × two zooms × two window sizes.
#[must_use]
pub fn matrix() -> Vec<(Preset, f32, Size)> {
    let mut cells = Vec::new();
    for preset in Preset::every() {
        for zoom in [1.0_f32, 1.5] {
            for window in [WINDOW, SMALL] {
                cells.push((preset, zoom, window));
            }
        }
    }
    cells
}

/// A launch that draws `preset`, with every override on.
#[must_use]
pub fn drawing(preset: Preset) -> Settings {
    Settings {
        theme: match preset {
            Preset::LightAaa => stikk_state::config::Theme::Light,
            Preset::DarkAaa => stikk_state::config::Theme::Dark,
            // The high-contrast pair is reached by the level plus the desktop's mode, so a test
            // that wants one names the level here and tells the app what the desktop says.
            Preset::HcLight | Preset::HcDark => stikk_state::config::Theme::HighContrast,
        },
        ..Settings::opening("/repo")
    }
}

/// An `OrientationView` with every field a screen draws filled in.
#[must_use]
pub fn orientation_view() -> stikk_core::OrientationView {
    let readiness = stikk_model::Readiness {
        author: stikk_model::RoleReadiness::Known(stikk_model::Binding::Matches),
        maintainer: stikk_model::RoleReadiness::Known(stikk_model::Binding::Matches),
        read_only: false,
    };
    stikk_core::OrientationView {
        interrupted_materialization: None,
        prikk_version: "prikk 0.59.0".to_string(),
        prikk_supported: true,
        prikk_validated: true,
        validated_through: "0.59".to_string(),
        prikk_persists_messages: true,
        prikk_minor: 59,
        queued_patches: 2,
        queued_target: Some("heads/main".to_string()),
        trailing_partial_wal_bytes: 0,
        main_ref_state: Some("237d0681aa11".to_string()),
        capability: stikk_model::Capability::derive(readiness),
        readiness,
        stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
        current_branch: stikk_model::CurrentBranch::NotReported,
    }
}

/// **Orientation with every sentence it can say**, which is the worst case the matrix measures:
/// an unvalidated prikk, both roles unverifiable, a stale seed variable, an interrupted
/// materialization with two routes, a torn tail, and an unresolved current branch.
///
/// The owner's GUI rules ask for the worst case plus one baseline; [`orientation_view`] is the
/// baseline and this is the worst case.
#[must_use]
pub fn orientation_view_at_its_longest() -> stikk_core::OrientationView {
    let readiness = stikk_model::Readiness {
        author: stikk_model::RoleReadiness::Unverifiable,
        maintainer: stikk_model::RoleReadiness::Unverifiable,
        read_only: true,
    };
    stikk_core::OrientationView {
        interrupted_materialization: Some(vec![
            "prikk checkout --ref heads/main --finish".to_string(),
            "prikk checkout --ref heads/main --abandon".to_string(),
        ]),
        prikk_validated: false,
        prikk_minor: 60,
        prikk_version: "prikk 0.60.0".to_string(),
        trailing_partial_wal_bytes: 4096,
        queued_target: Some(LONG_REF.to_string()),
        main_ref_state: Some(LONG_ID.to_string()),
        current_branch: stikk_model::CurrentBranch::Unresolved(
            "<unresolved; run `prikk doctor`>".to_string(),
        ),
        capability: stikk_model::Capability::derive(readiness),
        readiness,
        ..orientation_view()
    }
}

/// A program focused on Orientation at its longest.
#[must_use]
pub fn on_orientation_at_its_longest() -> App {
    let (tx, _rx) = mpsc::channel();
    App::from_state(
        "/repo",
        OrientationState::Loaded(orientation_view_at_its_longest()),
        tx,
    )
}

/// A ref name with no word boundary in it, long enough to fill the body's width at 150 %.
const LONG_REF: &str = "heads/feature/the-whole-team-agreed-this-was-a-good-name-for-the-branch-that-rewrites-the-delivery-pipeline-and-nobody-shortened-it";
/// A full-length object id, as prikk reports one.
const LONG_ID: &str = "237d0681aa1143c0de99f1c2b3a4958677e5d4c3b2a1908f7e6d5c4b3a291807";

/// A two-block history on `heads/main`.
#[must_use]
pub fn history_view() -> stikk_core::HistoryView {
    stikk_core::HistoryView {
        reff: "heads/main".to_string(),
        queued: 2,
        queued_target: Some("heads/main".to_string()),
        blocks: vec![
            block("bbbb111122223333", 2, "Normal", 3),
            block("aaaa444455556666", 1, "Root", 1),
        ],
    }
}

fn block(id: &str, update_seq: u64, kind: &str, patches: u64) -> stikk_prikk::BlockRow {
    stikk_prikk::BlockRow {
        block_id: id.to_string(),
        ref_state_id: format!("{id}-ref"),
        update_seq,
        kind: kind.to_string(),
        rollback_block: false,
        parents: 1,
        patches,
        rollback_patches: 0,
        required_attestations: 0,
        messages: Vec::new(),
        previous_ref_state: None,
    }
}

/// A program in its loaded state, with the request channel the fixture answers on.
#[must_use]
pub fn loaded() -> (App, mpsc::Receiver<Request>) {
    let (tx, rx) = mpsc::channel();
    let app = App::from_state("/repo", OrientationState::Loaded(orientation_view()), tx);
    (app, rx)
}

/// A program focused on Orientation, loaded.
#[must_use]
pub fn on_orientation() -> App {
    loaded().0
}

/// A program focused on History, with two blocks — reached the way a user reaches it, and answered
/// the way the worker answers.
#[must_use]
pub fn on_history() -> App {
    let (mut app, rx) = loaded();
    app.open_history();
    let seq = rx.try_recv().map(|request| request.seq).unwrap_or_default();
    app.apply(Response {
        seq,
        kind: ResponseKind::History(Ok(history_view())),
    });
    app
}

/// **The real app, drawn through a `Simulator` at one size.**
pub struct Screen {
    stikk: Stikk,
    size: Size,
}

impl Screen {
    /// Open `app` at `size`, as a window of that size would.
    ///
    /// The window's own two facts are **not** given here: a test that wants a confirmed viewport
    /// calls [`Screen::window_says`], and a test about rule 8's condition does not.
    #[must_use]
    pub fn open(app: App, settings: &Settings, size: Size) -> Self {
        let mut screen = Self {
            stikk: Stikk::for_test(app, settings),
            size,
        };
        screen.draw();
        screen
    }

    /// **What the window answers for itself** (rule 8): its own logical size, and its scale factor.
    pub fn window_says(&mut self, size: Size, scale: f32) {
        let _ = self.stikk.update(Message::WindowSize(size));
        let _ = self.stikk.update(Message::ScaleFactor(scale));
        self.draw();
    }

    /// What the desktop says about light or dark (rule 3).
    pub fn desktop_says(&mut self, mode: iced::theme::Mode) {
        let _ = self.stikk.update(Message::SystemTheme(mode));
        self.draw();
    }

    /// **Draw the app's own view at this size, and give it back what the drawing measured.**
    pub fn draw(&mut self) {
        let reports: Vec<Message> = {
            let _one_at_a_time = rendering();
            let mut simulator =
                Simulator::with_size(iced::Settings::default(), self.size, self.stikk.view());
            let _ = simulator.simulate([iced::Event::Window(
                iced::window::Event::RedrawRequested(Instant::now()),
            )]);
            simulator.into_messages().collect()
        };
        for report in reports {
            let _ = self.stikk.update(report);
        }
    }

    /// A key, as the message `keyboard::listen()` produces, then a draw.
    pub fn key(&mut self, key: IcedKey) {
        let _ = self
            .stikk
            .update(Message::Key(key, Modifiers::empty(), false));
        self.draw();
    }

    /// A catalog key, as a pointer press on a control produces it, then a draw.
    pub fn press(&mut self, key: CatalogKey) {
        let _ = self.stikk.update(Message::Press(key));
        self.draw();
    }

    /// Let the program's own clock run, which is how an opening script advances.
    pub fn beat(&mut self, beats: u32) {
        for _ in 0..beats {
            let task = self.stikk.update(Message::Tick);
            drop(task);
            // The opening script's step is a `Task::done`, which no harness performs — so the beat
            // sends the same message the task would have carried.
            if let Some(key) = self.stikk.next_opening_step() {
                self.press(key);
            } else {
                self.draw();
            }
        }
    }

    /// The app.
    #[must_use]
    pub fn stikk(&self) -> &Stikk {
        &self.stikk
    }

    /// The app, to drive directly where a test is about the program and not the drawing.
    pub fn stikk_mut(&mut self) -> &mut Stikk {
        &mut self.stikk
    }

    /// **Every text in the rendered tree**, with what it says and where it was laid out.
    #[must_use]
    pub fn texts(&self) -> Vec<TextBox> {
        self.walk(|candidate| match candidate {
            iced_test::selector::Candidate::Text {
                bounds,
                visible_bounds,
                content,
                ..
            } => Some(TextBox {
                content: content.to_string(),
                bounds,
                visible_bounds,
            }),
            _ => None,
        })
    }

    /// **Every text in the tree, with one of the shell's parts turned off** — the shape a guard's
    /// failing run measures.
    #[must_use]
    pub fn texts_without_the_banner_row(&self) -> Vec<TextBox> {
        self.texts_drawn_with(crate::shell::Draws {
            banner: false,
            ..crate::shell::Draws::shipped()
        })
    }

    /// Every text in the tree, drawn with `draws`.
    #[must_use]
    pub fn texts_drawn_with(&self, draws: crate::shell::Draws) -> Vec<TextBox> {
        let mut found: Vec<TextBox> = Vec::new();
        {
            let _one_at_a_time = rendering();
            let collect = |candidate: iced_test::selector::Candidate<'_>| -> Option<()> {
                if let iced_test::selector::Candidate::Text {
                    bounds,
                    visible_bounds,
                    content,
                    ..
                } = candidate
                {
                    found.push(TextBox {
                        content: content.to_string(),
                        bounds,
                        visible_bounds,
                    });
                }
                None
            };
            let mut simulator = Simulator::with_size(
                iced::Settings::default(),
                self.size,
                crate::shell::view_with(&self.stikk, draws),
            );
            let _ = simulator.find(collect);
        }
        found
    }

    /// **The card's own frame**, when one is open: the area a measurement must stay inside, so
    /// the page the scrim covers behind it is not measured against the content floor.
    #[must_use]
    pub fn card_frame(&self) -> Option<Rectangle> {
        let wanted = iced::widget::Id::from(crate::CARD_FRAME);
        self.walk(|candidate| match candidate {
            iced_test::selector::Candidate::Container {
                id: Some(id),
                bounds,
                ..
            } if *id == wanted => Some(bounds),
            _ => None,
        })
        .into_iter()
        .next()
    }

    /// **Every control the keyboard can reach**, with the area a pointer must hit.
    #[must_use]
    pub fn controls(&self) -> Vec<Rectangle> {
        self.walk(|candidate| match candidate {
            iced_test::selector::Candidate::Focusable { bounds, .. } => Some(bounds),
            _ => None,
        })
    }

    /// **Every scrollable**, with its own area and the height of what it holds.
    #[must_use]
    pub fn scrollables(&self) -> Vec<ScrollBox> {
        self.walk(|candidate| match candidate {
            iced_test::selector::Candidate::Scrollable {
                bounds,
                content_bounds,
                ..
            } => Some(ScrollBox {
                bounds,
                content: content_bounds,
            }),
            _ => None,
        })
    }

    /// **Walk the whole rendered tree**, collecting everything `pick` matches.
    ///
    /// `Simulator` has `find`, which stops at the first match, and no `find_all` — the operation
    /// behind it is not reachable from outside the crate. So the selector collects into a vector
    /// and answers `None` for every candidate, which makes `find` visit all of them and return
    /// its not-found error, and the vector is the result.
    fn walk<T: Send>(
        &self,
        mut pick: impl FnMut(iced_test::selector::Candidate<'_>) -> Option<T> + Send,
    ) -> Vec<T> {
        let mut found: Vec<T> = Vec::new();
        {
            let _one_at_a_time = rendering();
            let collect = |candidate: iced_test::selector::Candidate<'_>| -> Option<()> {
                if let Some(item) = pick(candidate) {
                    found.push(item);
                }
                None
            };
            let mut simulator =
                Simulator::with_size(iced::Settings::default(), self.size, self.stikk.view());
            let _ = simulator.find(collect);
        }
        found
    }
}

/// One text in the rendered tree.
#[derive(Debug, Clone)]
pub struct TextBox {
    /// What it says.
    pub content: String,
    /// Where the layout put it.
    pub bounds: Rectangle,
    /// How much of it is on screen, if any.
    pub visible_bounds: Option<Rectangle>,
}

/// One scrollable in the rendered tree.
#[derive(Debug, Clone, Copy)]
pub struct ScrollBox {
    /// The scrollable's own area.
    pub bounds: Rectangle,
    /// The area of everything it holds.
    pub content: Rectangle,
}

impl ScrollBox {
    /// Whether a bar is drawn at all: the content is taller than the room for it.
    #[must_use]
    pub fn scrolls(&self) -> bool {
        self.content.height > self.bounds.height + 0.5
    }
}

/// **The rendered pixels** of the same view at the same size.
///
/// Through `Emulator::screenshot`, which is the only path to them: `Simulator::snapshot` keeps its
/// `window::Screenshot` private.
#[must_use]
pub fn picture(
    fixture: fn() -> App,
    settings: &Settings,
    size: Size,
    mode: iced::theme::Mode,
    open: Option<CatalogKey>,
) -> Picture {
    let _one_at_a_time = rendering();
    let launch = settings.clone();
    let program = iced::application(
        move || {
            let mut stikk = Stikk::for_test(fixture(), &launch);
            let _ = stikk.update(Message::SystemTheme(mode));
            // The same scene the tree was read from: a card is opened through the ordinary
            // message path, or the picture would be of the screen without it.
            if let Some(key) = open {
                let _ = stikk.update(Message::Press(key));
            }
            (stikk, iced::Task::none())
        },
        Stikk::update,
        Stikk::view,
    )
    .theme(Stikk::theme);
    let (sender, mut receiver) = test_mpsc::channel(1);
    let mut emulator = emulator::Emulator::new(sender, &program, emulator::Mode::Immediate, size);
    pump(
        &mut emulator,
        &program,
        &mut receiver,
        Duration::from_millis(400),
    );
    // The theme the app itself would draw with, computed the same way: the configuration's choice,
    // then what the desktop said. Not `Emulator::theme`, which answers `Option` and would need a
    // fallback that could quietly measure the wrong preset.
    let mut choice = crate::theme::Choice::from_config(settings.theme);
    let _ = choice.desktop_says(mode);
    let theme = stikk_gui_design::theme(choice.preset(), settings.overrides);
    let shot = emulator.screenshot(&program, &theme, 1.0);
    Picture {
        rgba: shot.rgba.to_vec(),
        width: shot.size.width,
        height: shot.size.height,
    }
}

/// **The specimen**: one of each control, available and unavailable, in the preset under test.
///
/// RFC 065's M4 measured the same four and found Snora's unavailable pair at 1.48–2.90 : 1. No
/// screen this handoff draws has an unavailable control, so the specimen is what carries rule 3's
/// unavailable floors — and it is drawn by the design crate's own functions, which is what the
/// rule is about.
#[must_use]
pub fn specimen(preset: Preset, overrides: Overrides) -> iced::Element<'static, ()> {
    let tokens = stikk_gui_design::tokens(preset, overrides);
    iced::widget::column![
        stikk_gui_design::button_primary(&tokens, overrides, "spec-1", "Confirm", Some(())),
        stikk_gui_design::button_primary(&tokens, overrides, "spec-2", "Confirm", None),
        stikk_gui_design::button_secondary(&tokens, overrides, "spec-3", "Go back", Some(())),
        stikk_gui_design::button_secondary(&tokens, overrides, "spec-4", "Go back", None),
    ]
    .spacing(16)
    .padding(24)
    .into()
}

/// What the specimen is drawn with.
#[derive(Debug, Clone, Copy)]
pub struct Spec {
    /// The preset under test.
    pub preset: Preset,
    /// Which of stikk's overrides are on.
    pub overrides: Overrides,
}

/// The specimen's own view — **a named function**, because `iced`'s `ViewFn` has to hold for every
/// lifetime and a closure annotated with one concrete `&State` does not.
fn spec_view(spec: &Spec) -> iced::Element<'_, ()> {
    specimen(spec.preset, spec.overrides)
}

/// **The rendered pixels of the specimen**, for a style that no screen draws.
///
/// The same path as [`picture`] — `Emulator::screenshot`.
#[must_use]
pub fn picture_of_specimen(spec: Spec, size: Size) -> Picture {
    let _one_at_a_time = rendering();
    let theme = stikk_gui_design::theme(spec.preset, spec.overrides);
    let for_program = theme.clone();
    let program = iced::application(
        move || (spec, iced::Task::none()),
        |_: &mut Spec, ()| {},
        spec_view,
    )
    .theme(move |_: &Spec| for_program.clone());
    let (sender, mut receiver) = test_mpsc::channel(1);
    let mut emulator = emulator::Emulator::new(sender, &program, emulator::Mode::Immediate, size);
    pump(
        &mut emulator,
        &program,
        &mut receiver,
        Duration::from_millis(200),
    );
    let shot = emulator.screenshot(&program, &theme, 1.0);
    Picture {
        rgba: shot.rgba.to_vec(),
        width: shot.size.width,
        height: shot.size.height,
    }
}

/// Let the emulator work for `duration`, performing whatever it asks for.
fn pump<P: iced_test::program::Program + 'static>(
    emulator: &mut emulator::Emulator<P>,
    program: &P,
    receiver: &mut test_mpsc::Receiver<emulator::Event<P>>,
    duration: Duration,
) {
    let until = Instant::now() + duration;
    while Instant::now() < until {
        match receiver.try_recv() {
            Ok(emulator::Event::Action(action)) => emulator.perform(program, action),
            Ok(emulator::Event::Failed(instruction)) => {
                panic!("the emulator refused an instruction: {instruction}")
            }
            Ok(_) => {}
            Err(_) => std::thread::sleep(Duration::from_millis(5)),
        }
    }
}

/// The rendered pixels, with the geometry to read them by.
#[derive(Debug, Clone)]
pub struct Picture {
    /// Four bytes per pixel, row by row.
    pub rgba: Vec<u8>,
    /// The picture's width in pixels.
    pub width: u32,
    /// The picture's height in pixels.
    pub height: u32,
}

impl Picture {
    /// One pixel's colour, or `None` outside the picture.
    #[must_use]
    pub fn at(&self, x: u32, y: u32) -> Option<snora::design::Color> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let at = ((y * self.width + x) * 4) as usize;
        let channel = |offset: usize| self.rgba.get(at + offset).map(|&b| f32::from(b) / 255.0);
        Some(snora::design::Color {
            r: channel(0)?,
            g: channel(1)?,
            b: channel(2)?,
            a: channel(3)?,
        })
    }

    /// **Every colour in a rectangle, with how many pixels carry it**, most first.
    ///
    /// Exact `u8` triples, so a glyph's or a thumb's own colour is one entry and its antialiasing
    /// is a crowd of rare ones. That is what makes "the colour this mark was drawn in" answerable
    /// from a picture rather than guessed.
    #[must_use]
    pub fn colours_in(&self, area: Rectangle) -> Vec<(snora::design::Color, usize)> {
        let mut counts: std::collections::HashMap<[u8; 3], usize> =
            std::collections::HashMap::new();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (x0, y0) = (area.x.max(0.0) as u32, area.y.max(0.0) as u32);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (x1, y1) = (
            (area.x + area.width).max(0.0).ceil() as u32,
            (area.y + area.height).max(0.0).ceil() as u32,
        );
        for y in y0..y1.min(self.height) {
            for x in x0..x1.min(self.width) {
                let at = ((y * self.width + x) * 4) as usize;
                let Some(bytes) = self.rgba.get(at..at + 3) else {
                    continue;
                };
                let Ok(key) = <[u8; 3]>::try_from(bytes) else {
                    continue;
                };
                *counts.entry(key).or_default() += 1;
            }
        }
        let mut out: Vec<([u8; 3], usize)> = counts.into_iter().collect();
        out.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
        out.into_iter()
            .map(|([r, g, b], n)| {
                (
                    snora::design::Color {
                        r: f32::from(r) / 255.0,
                        g: f32::from(g) / 255.0,
                        b: f32::from(b) / 255.0,
                        a: 1.0,
                    },
                    n,
                )
            })
            .collect()
    }

    /// **The contrast a text was drawn at**, measured from its own rectangle.
    ///
    /// The darkest and the lightest pixel in the rectangle: a glyph's core is drawn in the text's
    /// own colour at full coverage, and the rest of the rectangle is the surface behind it, so the
    /// two extremes of luminance in a text's own bounds are its foreground and its background. The
    /// pixels between them are the antialiasing, and they are neither.
    ///
    /// `None` for a rectangle with nothing in it — an empty line, or a text off screen.
    #[must_use]
    pub fn contrast_in(&self, area: Rectangle) -> Option<f32> {
        let luminance = snora::design::contrast::relative_luminance;
        let mut darkest: Option<snora::design::Color> = None;
        let mut lightest: Option<snora::design::Color> = None;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (x0, y0) = (area.x.max(0.0) as u32, area.y.max(0.0) as u32);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (x1, y1) = (
            (area.x + area.width).max(0.0).ceil() as u32,
            (area.y + area.height).max(0.0).ceil() as u32,
        );
        for y in y0..y1.min(self.height) {
            for x in x0..x1.min(self.width) {
                let Some(colour) = self.at(x, y) else {
                    continue;
                };
                if darkest.is_none_or(|d| luminance(colour) < luminance(d)) {
                    darkest = Some(colour);
                }
                if lightest.is_none_or(|l| luminance(colour) > luminance(l)) {
                    lightest = Some(colour);
                }
            }
        }
        let (dark, light) = (darkest?, lightest?);
        Some(snora::design::contrast::contrast_ratio(dark, light))
    }
}

/// A named key, for the steps.
#[must_use]
pub fn named(name: Named) -> IcedKey {
    IcedKey::Named(name)
}

/// A typed character, for the steps.
#[must_use]
pub fn character(c: &str) -> IcedKey {
    IcedKey::Character(c.into())
}

/// Every override on — what a shipped build draws with.
#[must_use]
pub fn stikk_overrides() -> Overrides {
    Overrides::stikk()
}
