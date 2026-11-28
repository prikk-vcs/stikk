//! **Can a runner open the app's window, and report its viewport?** (RFC 070 Handoff A §1.)
//!
//! RFC 069 rule 3's Windows test and rule 8's Windows test both depend on the answer, and RFC 065
//! left it open: its W2 measured a card reporting its end visible before the window's scaled size
//! had arrived, and nothing in that evidence could say whether the viewport it read was the window
//! on screen. A machine has to answer it, not an owner session.
//!
//! **What this does.**
//! 1. Opens the real app — the same `stikk_gui::Stikk`, the same `view`, the same keys — on a
//!    scratch directory, so nothing about any repository is involved.
//! 2. Opens the **Glossary** through the program's own intent, with no keys sent, using the app's
//!    own opening script.
//! 3. Logs every window fact it is given — `window::size`, `window::scale_factor`, the viewport the
//!    shell was laid out at, and the derived physical size — and the program's whole state line
//!    beside each.
//! 4. **Exits 0 as soon as the viewport is confirmed** (RFC 070 decision 5), and **1** if the
//!    deadline passes without that.
//!
//! **It is an example**, so no probe code is in the app.
//!
//! ```text
//! cargo run -p stikk-gui --example window_probe -- [--scale 2.5] [--seconds 30]
//! ```
//!
//! **Why the exit code and not only the log:** a CI job must fail on its own, without a human
//! reading an artefact. The log is the record; the code is the gate.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use stikk_gui::{Message, Settings, Stikk};

/// Whether the viewport was confirmed before the deadline. Read by `main` after the window closes,
/// which is the only moment both facts — the run finished, and what it found — are in one place.
static CONFIRMED: AtomicBool = AtomicBool::new(false);
/// **Whether anything was ever read before the viewport was confirmed** (RFC 069 rule 8's C test).
///
/// This is the question RFC 065 left open: on Windows a card reported its end visible on arrival,
/// before the window's scaled size had arrived. If it happens here, the probe fails whatever else
/// it found.
static READ_TOO_EARLY: AtomicBool = AtomicBool::new(false);

/// The app, plus the probe's own deadline and its record.
struct Probe {
    stikk: Stikk,
    started: Instant,
    deadline: Duration,
    /// The last line written, so the log carries every change and not one line per beat.
    last: String,
}

impl Probe {
    fn new(settings: Settings, deadline: Duration) -> (Self, iced::Task<Message>) {
        let (stikk, task) = Stikk::new(settings);
        println!("probe: opening, deadline {} s", deadline.as_secs_f32());
        (
            Self {
                stikk,
                started: Instant::now(),
                deadline,
                last: String::new(),
            },
            task,
        )
    }

    /// The app's own view, unchanged: what is measured is what ships.
    ///
    /// A named function and not a closure, because `iced`'s `ViewFn` has to hold for every
    /// lifetime and a closure annotated with one concrete `&Probe` does not.
    fn view(&self) -> iced::Element<'_, Message> {
        self.stikk.view()
    }

    fn update(&mut self, message: Message) -> iced::Task<Message> {
        // **Named before the update**, so a line that reports a window fact names the fact that
        // produced it rather than the one after it.
        let arrived = match &message {
            Message::WindowSize(size) => {
                Some(format!("window::size {}x{}", size.width, size.height))
            }
            Message::ScaleFactor(scale) => Some(format!("window::scale_factor {scale}")),
            Message::LaidOut(size) => Some(format!("laid out {}x{}", size.width, size.height)),
            Message::SystemTheme(mode) => Some(format!("system::theme {mode:?}")),
            _ => None,
        };
        let task = self.stikk.update(message);
        if let Some(arrived) = arrived {
            println!("probe: {arrived}");
        }
        let line = format!(
            "{} | preset {} | {}",
            self.stikk.viewport().line(),
            self.stikk.preset().name(),
            self.stikk.state_line(),
        );
        if line != self.last {
            println!("probe: {line}");
            self.last = line;
        }
        // **Rule 8's own assertion, taken on every message**: nothing may be read before the
        // window has answered for itself.
        if !self.stikk.viewport().confirmed()
            && (self.stikk.app().seen_whole_card() || self.stikk.app().more_above())
        {
            READ_TOO_EARLY.store(true, Ordering::SeqCst);
            println!(
                "probe: something was read before the viewport was confirmed: {}",
                self.stikk.state_line()
            );
        }
        if self.stikk.viewport().confirmed() {
            CONFIRMED.store(true, Ordering::SeqCst);
            println!("probe: the viewport is confirmed; the app may report a reading");
            return iced::exit();
        }
        if self.started.elapsed() >= self.deadline {
            println!(
                "probe: the deadline passed with the viewport unconfirmed: {}",
                self.stikk.viewport().line()
            );
            return iced::exit();
        }
        task
    }
}

fn main() -> std::process::ExitCode {
    let mut scale = 1.0_f32;
    let mut seconds = 30_u64;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--scale" => {
                scale = arguments
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(scale);
            }
            "--seconds" => {
                seconds = arguments
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(seconds);
            }
            other => {
                eprintln!("probe: unknown argument {other:?}");
                return std::process::ExitCode::from(2);
            }
        }
    }

    // **A scratch directory, and no repository.** The question is about the window, so nothing
    // here depends on prikk being installed: the program reports its own fault on the screen and
    // the window opens all the same, which is exactly what is being measured.
    let scratch = std::env::temp_dir().join(format!("stikk-window-probe-{}", std::process::id()));
    if let Err(error) = std::fs::create_dir_all(&scratch) {
        eprintln!("probe: could not make a scratch directory: {error}");
        return std::process::ExitCode::from(1);
    }

    let settings = Settings {
        zoom: scale,
        // **The Glossary, through the program's own intent** — the card rule 8's sequence needs,
        // opened with no keys sent.
        opening: vec![stikk_core::Key::Help],
        ..Settings::opening(&scratch)
    };
    println!(
        "probe: scale {scale}, repository {}, interface font {:?}",
        scratch.display(),
        stikk_gui_design::interface_font_name(),
    );

    let deadline = Duration::from_secs(seconds);
    let outcome = iced::application(
        move || Probe::new(settings.clone(), deadline),
        Probe::update,
        Probe::view,
    )
    .title(|probe: &Probe| probe.stikk.title())
    .subscription(|probe: &Probe| probe.stikk.subscription())
    .theme(|probe: &Probe| probe.stikk.theme())
    .scale_factor(|probe: &Probe| probe.stikk.scale_factor())
    .default_font(stikk_gui_design::interface_font())
    .window(stikk_gui::window_settings())
    .run();

    let _ = std::fs::remove_dir_all(&scratch);
    match outcome {
        Err(error) => {
            // **A runner that cannot open a window at all says so here**, with iced's own words,
            // and the job reports it rather than retrying.
            eprintln!("probe: the window could not be opened: {error}");
            std::process::ExitCode::from(1)
        }
        Ok(()) if READ_TOO_EARLY.load(Ordering::SeqCst) => {
            eprintln!(
                "probe: a reading was taken before the viewport was confirmed — rule 8's \
                 condition did not hold on this runner"
            );
            std::process::ExitCode::from(1)
        }
        Ok(()) if CONFIRMED.load(Ordering::SeqCst) => {
            println!("probe: confirmed, and nothing was read before it was");
            std::process::ExitCode::SUCCESS
        }
        Ok(()) => {
            eprintln!("probe: unconfirmed");
            std::process::ExitCode::from(1)
        }
    }
}
