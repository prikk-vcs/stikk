//! **The app's own state log** (RFC 070 Handoff A §6), for the real-window checks.
//!
//! `STIKK_GUI_LOG=<file>` writes one line after every message: what arrived, and the program's
//! whole state after it. That line is what a nested-compositor run asserts — **recorded state, not
//! pixels**, which is what the owner's GUI rules ask for first — and it is the same line the
//! headless tests read, so a step there checks what a step here checks.
//!
//! **Unset, it writes nothing**, opens no file and formats no line.

use std::io::Write as _;

use stikk_app::{App, Focus, Overlay};

use crate::Message;
use crate::reading::{Report, Viewport};

/// The state log, open or not.
#[derive(Debug)]
pub struct Log {
    file: Option<std::fs::File>,
    last: String,
}

impl Log {
    /// Open the log named by `STIKK_GUI_LOG`, or hold nothing.
    ///
    /// A path that cannot be created is not an error the user needs: the app is running, and the
    /// log is a measurement instrument. The run's own script checks that the file appeared.
    #[must_use]
    pub fn open() -> Self {
        Self {
            file: std::env::var_os("STIKK_GUI_LOG")
                .and_then(|path| std::fs::File::create(path).ok()),
            last: String::new(),
        }
    }

    /// Whether anything is being written — so a caller can skip formatting a line nobody reads.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.file.is_some()
    }

    /// **Record one message and the state it left.**
    ///
    /// `label` names what arrived. When it is empty the line is written only if the state changed,
    /// which is how an answer the worker brought — a screen that finished loading — is recorded
    /// too, rather than only the keys.
    pub fn record(&mut self, label: &str, app: &App, viewport: &Viewport, report: &Report) {
        let line = state_line(app, viewport, report);
        if label.is_empty() && line == self.last {
            self.last = line;
            return;
        }
        if let Some(file) = self.file.as_mut() {
            let shown = if label.is_empty() { "state" } else { label };
            let _ = writeln!(file, "{shown} | {line}");
        }
        self.last = line;
    }
}

/// **The program's whole state, in one line.**
///
/// The terminal's own golden frames are pixels; this is the GUI's equivalent, and it is deliberately
/// the same shape as RFC 065's prototype wrote, so the prototype's records and this app's can be
/// read side by side.
#[must_use]
pub fn state_line(app: &App, viewport: &Viewport, report: &Report) -> String {
    let top = match app.top_overlay() {
        Some(overlay) => format!("overlay {:?}", overlay.title().trim()),
        None => format!("focus {}", screen_name(&app.focus())),
    };
    let cursor = match app.top_overlay() {
        Some(Overlay::ActionList {
            cursor, entries, ..
        }) => format!(
            " cursor {cursor} entry {:?}",
            entries.get(*cursor).map(|a| a.name)
        ),
        _ => match app.focus() {
            Focus::History(_, cursor) => format!(" cursor {cursor}"),
            _ => String::new(),
        },
    };
    let measured = match report.measured() {
        Some((surface, at_end, above)) => {
            format!("measured {surface:?} at_end {at_end} more_above {above}")
        }
        None => "measured none".to_string(),
    };
    format!(
        "{top}{cursor} | at_end {} more_above {} seen {} | {measured} | {}",
        app.at_end(),
        app.more_above(),
        app.seen_whole_card(),
        viewport.line(),
    )
}

/// A screen's own name, as the header line and the records write it.
#[must_use]
pub fn screen_name(focus: &Focus<'_>) -> &'static str {
    match focus {
        Focus::Orientation(_) => "Orientation",
        Focus::Loading(_) => "Loading",
        Focus::History(..) => "History",
        Focus::BlockDetail(..) => "Block detail",
        Focus::Trust(..) => "Trust",
        Focus::Changes(..) => "Changes",
        Focus::Queue(..) => "Queue",
        Focus::PatchDetail(..) => "Patch detail",
        Focus::Compare(..) => "Compare",
        Focus::MergePlan(..) => "Merge plan",
        Focus::Bundle(..) => "Bundle",
        Focus::Verify(..) => "Verify",
    }
}

/// **What a message is, for the state log.**
///
/// In this module and not beside `update`, because it is a record and not a sentence anyone reads
/// on the screen: rule 13's check holds the views to the template table, and this is the one place
/// that writes the words a measured run is asserted on.
#[must_use]
pub fn label(message: &Message, preset: stikk_gui_design::Preset) -> String {
    match message {
        Message::Key(key, _, repeat) => {
            format!("key {key:?}{}", if *repeat { " (repeat)" } else { "" })
        }
        Message::Scrolled(..) => "scrolled".to_string(),
        Message::TopShown(_) => "top shown".to_string(),
        Message::EndVisible(_, visible) => format!("end visible {visible}"),
        Message::Press(key) => format!("button {key:?}"),
        Message::WindowSize(size) => format!("window {}x{}", size.width, size.height),
        Message::ScaleFactor(scale) => format!("scale {scale}"),
        Message::LaidOut(size) => format!("laid out {}x{}", size.width, size.height),
        // Rule 3: Light opens light_aaa, Dark opens dark_aaa, and **no preference opens
        // light_aaa**. The preset is named so the record shows what the rule chose, not only
        // what iced reported.
        Message::SystemTheme(mode) => {
            format!("system theme: {mode:?} -> drawing {}", preset.name())
        }
        Message::Tick | Message::Ignored => String::new(),
    }
}
