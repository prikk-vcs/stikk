//! The README's key visuals, rendered by stikk's own views from invented data (RFC 042).
//!
//! **The images are rendered, never photographed**, because the project owner's condition is that
//! nothing of their machine may appear in them: no path, user name, host name, repository
//! content, key id or key material. A photograph of a terminal carries all of that and would have
//! to be scrubbed by hand every time. A scene built here carries none of it **by construction** --
//! there is nothing real in it to leak.
//!
//! # How a scene is made
//!
//! Each one drives the **real `App` and the real views**, the way the view tests do: a
//! [`NullBackend`] scripted for the purpose, the same methods the keys call (`open_history`,
//! `select`, `mark_block`, `compare`, `open_verify`), and the same `shell::render` the run loop
//! calls -- into a `TestBackend` buffer at 80 x 24. So an image cannot drift from the product: if
//! a view changes, the image changes, and `readme_images_are_current` fails until it is
//! regenerated.
//!
//! # Why this is a test module and not an example
//!
//! RFC 042's handoff asks for `examples/readme_visuals.rs`. **An example is a separate crate**,
//! and driving the real `App` from outside this one would mean making `App::from_state`,
//! `App::apply`, `shell::render` and the whole private `worker` protocol (`Request`, `Response`,
//! `ResponseKind`) public -- publishing stikk's internal worker protocol in order to draw a
//! picture. The RFC's own constraint is that the SVG writer is **not public API**; keeping the
//! generator here keeps *nothing* public. The cost is that regenerating is a `cargo test` line
//! rather than a `cargo run` one, which `CONTRIBUTING.md` carries.
//!
//! # Regenerating
//!
//! ```sh
//! cargo test -p stikk-tui regenerate_readme_images -- --ignored
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::PathBuf;
use std::sync::mpsc;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

use crate::app::{App, OrientationState};
use crate::theme::Palette;
use crate::worker::{Response, ResponseKind};

/// Where the committed images live, relative to the workspace root.
const IMAGE_DIR: &str = "docs/assets/readme";

/// The terminal the images are drawn at -- the width every capture in this project uses.
const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;

// The invented world. **Nothing here is copied from any real repository.**
const REPO: &str = "example-project";
const MAIN: &str = "heads/main";

/// An invented object id: **visibly a placeholder**, so no reader mistakes it for a real one and
/// no real one can hide among them. One repeated hex digit and a fixed tail.
fn invented_id(tag: char) -> String {
    format!("{}0badc0de", tag.to_string().repeat(56))
}

/// **Every 64-hex id a scene may contain**, and the only ones the privacy guard allows. It is the
/// same set the scenes draw from, so the declaration cannot drift from the use -- and a real id
/// pasted into a fixture by mistake is still not in it.
fn invented_ids() -> Vec<String> {
    "123456789abcdef".chars().map(invented_id).collect()
}

/// One scene: the file it is written to, and its SVG.
struct Scene {
    name: &'static str,
    svg: String,
}

// -------------------------------------------------------------------------------------------
// The scenes.

fn orientation() -> stikk_core::OrientationView {
    let readiness = stikk_model::Readiness::none();
    stikk_core::OrientationView {
        interrupted_materialization: None,
        prikk_version: "prikk 0.51.0".to_string(),
        prikk_supported: true,
        prikk_validated: true,
        validated_through: "0.51.0".to_string(),
        prikk_persists_messages: true,
        prikk_minor: 51,
        queued_patches: 2,
        queued_target: Some(MAIN.to_string()),
        trailing_partial_wal_bytes: 0,
        main_ref_state: Some(invented_id('4')),
        capability: stikk_model::Capability::derive(readiness),
        readiness,
        stale_seed_variables: stikk_prikk::env::StaleSeedVariables::default(),
        current_branch: stikk_model::CurrentBranch::NotReported,
    }
}

/// An `App` focused on the invented repository, with nothing loaded yet.
fn app() -> (App, mpsc::Receiver<crate::worker::Request>) {
    let (tx, rx) = mpsc::channel();
    let app = App::from_state(
        PathBuf::from(REPO),
        OrientationState::Loaded(orientation()),
        Palette::default(),
        tx,
    );
    (app, rx)
}

fn block(
    id: &str,
    state: &str,
    seq: u64,
    kind: &str,
    patches: u64,
    prev: Option<&str>,
) -> stikk_prikk::BlockRow {
    stikk_prikk::BlockRow {
        block_id: id.to_string(),
        ref_state_id: state.to_string(),
        update_seq: seq,
        kind: kind.to_string(),
        rollback_block: false,
        parents: u64::from(prev.is_some()),
        patches,
        rollback_patches: 0,
        required_attestations: 0,
        messages: Vec::new(),
        previous_ref_state: prev.map(str::to_string),
    }
}

fn history() -> stikk_core::HistoryView {
    // A lineage with something to look at: a merge, a rollback block, and one that needs an
    // attestation -- the rows whose markers are the reason History looks the way it does.
    // Long enough to fill a 24-row terminal, because an image of a mostly empty panel says less
    // about stikk than a lineage does.
    let rows = [
        // (id, ref-state, seq, kind, patches, rollback, attestations)
        ('1', 'd', 14, "Merge", 4, false, 0),
        ('2', 'e', 13, "Normal", 2, false, 0),
        ('3', 'f', 12, "Normal", 7, true, 0),
        ('4', '1', 11, "Normal", 1, false, 2),
        ('5', '2', 10, "Normal", 3, false, 0),
        ('6', '3', 9, "Normal", 1, false, 0),
        ('7', '4', 8, "Merge", 6, false, 0),
        ('8', '5', 7, "Normal", 2, false, 0),
        ('9', '6', 6, "Normal", 5, false, 0),
        ('a', '7', 5, "Normal", 1, false, 0),
        ('b', '8', 4, "Normal", 3, false, 0),
        ('c', '9', 3, "Normal", 2, false, 0),
        ('d', 'a', 2, "Normal", 1, false, 0),
        ('e', 'b', 1, "Root", 1, false, 0),
    ];
    let last = rows.len() - 1;
    stikk_core::HistoryView {
        reff: MAIN.to_string(),
        queued: 2,
        queued_target: Some(MAIN.to_string()),
        blocks: rows
            .iter()
            .enumerate()
            .map(
                |(at, (id, state, seq, kind, patches, rollback, attestations))| {
                    let mut row = block(
                        &invented_id(*id),
                        &invented_id(*state),
                        *seq,
                        kind,
                        *patches,
                        (at != last).then(|| invented_id(rows[at + 1].1)).as_deref(),
                    );
                    row.rollback_block = *rollback;
                    row.required_attestations = *attestations;
                    row
                },
            )
            .collect(),
    }
}

/// **Scene 1 -- History.** The lineage: the queued tier, three sealed blocks, the tip marked.
fn scene_history() -> Buffer {
    let (mut app, rx) = app();
    app.open_history();
    let seq = rx.recv().expect("a History request").seq;
    app.apply(Response {
        seq,
        kind: ResponseKind::History(Ok(history())),
    });
    draw(&app)
}

/// **Scene 2 -- Compare**, with one whitespace-only change among its entries: the rendering that
/// most distinguishes stikk, and the one a line-collapsing viewer shows as no change at all.
fn scene_compare() -> Buffer {
    let (mut app, rx) = app();
    app.open_history();
    let seq = rx.recv().expect("a History request").seq;
    app.apply(Response {
        seq,
        kind: ResponseKind::History(Ok(history())),
    });
    // **`d` on the selected block**, which compares it with the previous block in this ref's
    // history -- the comparison a reader gets without setting anything up first. Marking (`m`)
    // would leave its banner across the foot of the image, which is clutter in a still picture.
    app.nav_down();
    app.compare();
    let request = rx.recv().expect("a Compare request");
    app.apply(Response {
        seq: request.seq,
        kind: ResponseKind::Compare {
            opened: crate::app::CompareSource::PreviousInHistory,
            result: Ok(stikk_core::CompareView::Listed(stikk_core::compared(
                compare_diff(),
            ))),
        },
    });
    draw(&app)
}

fn text_file(size: u64) -> stikk_prikk::DiffFile {
    stikk_prikk::DiffFile {
        kind: "file".to_string(),
        encoding: "text".to_string(),
        mode: 0o100_644,
        size,
        content_id: None,
    }
}

fn compare_diff() -> stikk_prikk::Diff {
    stikk_prikk::Diff {
        from: stikk_prikk::DiffSide {
            point: invented_id('2'),
            target: stikk_prikk::DiffTarget::Block(invented_id('2')),
            queued: stikk_prikk::QueuedOnSide::NotApplicable,
        },
        to: stikk_prikk::DiffSide {
            point: invented_id('1'),
            target: stikk_prikk::DiffTarget::Block(invented_id('1')),
            queued: stikk_prikk::QueuedOnSide::NotApplicable,
        },
        entries: vec![
            stikk_prikk::DiffEntry {
                path: "src/main.rs".to_string(),
                status: "modified".to_string(),
                from_path: None,
                from: Some(text_file(96)),
                to: Some(text_file(98)),
                minimal: true,
                hunks: vec![
                    // **A whitespace-only change**: a trailing space added. A viewer that
                    // collapsed runs of spaces would show these two lines as identical.
                    "@@ -3 +3 @@\n-    println!(\"hello\");\n+    println!(\"hello\");  \n"
                        .to_string(),
                ],
                raw: String::new(),
            },
            stikk_prikk::DiffEntry {
                path: "README.md".to_string(),
                status: "modified".to_string(),
                from_path: None,
                from: Some(text_file(42)),
                to: Some(text_file(61)),
                minimal: true,
                hunks: vec![
                    "@@ -1 +1,2 @@\n # example-project\n+A small project, for the picture.\n"
                        .to_string(),
                ],
                raw: String::new(),
            },
        ],
        unsupported_paths: Vec::new(),
    }
}

/// **Scene 3 -- Verify**: prikk's verdict and its stages.
fn scene_verify() -> Buffer {
    let (mut app, rx) = app();
    app.open_verify();
    let seq = rx.recv().expect("a Verify request").seq;
    app.apply(Response {
        seq,
        kind: ResponseKind::Verify(Ok(stikk_core::VerifyView::Checked(stikk_core::checked(
            verification(),
        )))),
    });
    draw(&app)
}

fn verification() -> stikk_prikk::Verification {
    let stages = [
        "objects",
        "refs",
        "ref-update-schema-trust",
        "wal-replay",
        "wal-persistence",
        "rollback-drafts",
        "wal-record-schema",
        "active-wal-metadata",
        "commit-witness",
        "publication-reclassification",
        "commit-index",
        "lifecycle-cache",
        "wal-ordering",
        "received-refs",
        "local-tag-trust",
        "object-connectivity",
        "appended-file-tails",
    ];
    stikk_prikk::Verification {
        verdict: stikk_prikk::Verdict {
            ok: true,
            failed_conditions: Vec::new(),
        },
        active_sessions: Some(stikk_prikk::ActiveSessions {
            count: 1,
            verified_count: 1,
        }),
        stages: stages
            .into_iter()
            .map(|s| stikk_prikk::VerifyStage {
                stage: s.to_string(),
                status: stikk_prikk::StageStatus::Evaluated,
            })
            .collect(),
        raw: String::new(),
    }
}

/// Draw one `App` through the same `shell::render` the run loop calls.
fn draw(app: &App) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("a test terminal");
    terminal
        .draw(|frame| crate::shell::render(app, frame))
        .expect("a draw");
    terminal.backend().buffer().clone()
}

/// Every scene, in README order.
fn scenes() -> Vec<Scene> {
    vec![
        Scene {
            name: "history.svg",
            svg: svg(&scene_history()),
        },
        Scene {
            name: "compare.svg",
            svg: svg(&scene_compare()),
        },
        Scene {
            name: "verify.svg",
            svg: svg(&scene_verify()),
        },
    ]
}

// -------------------------------------------------------------------------------------------
// The cell-to-SVG writer.
//
// **SVG, not a bitmap**: it is text, so it is reviewable and greppable (which is what makes the
// privacy guard possible at all), it stays sharp on every display, and it has no metadata block
// for a camera, an OS or a path to hide in.

/// Cell geometry. Fixed numbers rather than measured ones: the output must be byte-identical on
/// every machine, and a measured advance width is not.
const CELL_W: u32 = 8;
const CELL_H: u32 = 17;
const BASELINE: u32 = 13;

/// The font is **named, never embedded**: embedding one would make the file huge and would carry
/// a licence question into this repository for no gain.
const FONT: &str = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";

/// The page behind the cells. The palette's own colours are foregrounds; a terminal supplies the
/// background, so the image supplies one too, dark to match [`Palette::dark`].
const BACKGROUND: &str = "#12161b";

/// Line thickness for the box-drawing characters, in pixels.
const STROKE: u32 = 1;

/// One buffer as an SVG document.
fn svg(buffer: &Buffer) -> String {
    let area = buffer.area;
    let width = u32::from(area.width) * CELL_W;
    let height = u32::from(area.height) * CELL_H;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" \
         viewBox=\"0 0 {width} {height}\" role=\"img\">\n"
    ));
    out.push_str(&format!(
        "<rect width=\"{width}\" height=\"{height}\" fill=\"{BACKGROUND}\"/>\n"
    ));
    out.push_str(&format!(
        "<g font-family=\"{FONT}\" font-size=\"13\" xml:space=\"preserve\">\n"
    ));

    for y in area.top()..area.bottom() {
        // One `<text>` run per stretch of equal style, not one element per cell: a cell-per-
        // element file is twenty times the size and no clearer.
        let mut run: Option<Run> = None;
        // A stretch of `\u{2500}` becomes one rectangle rather than eighty.
        let mut rule: Option<(u16, u32, String)> = None;

        for x in area.left()..area.right() {
            let Some(cell) = buffer.cell((x, y)) else {
                continue;
            };
            let style = CellStyle::of(cell);
            let symbol = cell.symbol();
            let is_box =
                symbol.chars().count() == 1 && symbol.chars().next().is_some_and(is_box_drawing);

            if !is_box {
                if let Some((start, count, fill)) = rule.take() {
                    out.push_str(&horizontal_rule(start, y, count, &fill));
                }
                match &mut run {
                    Some(open) if open.style == style => open.text.push_str(symbol),
                    Some(open) => {
                        out.push_str(&open.render(y));
                        run = Some(Run::start(x, style, symbol));
                    }
                    None => run = Some(Run::start(x, style, symbol)),
                }
                continue;
            }

            // A box-drawing character: not text here. See `box_glyph`.
            if let Some(open) = run.take() {
                out.push_str(&open.render(y));
            }
            let fill = style.fg.clone();
            if symbol == "\u{2500}" {
                match &mut rule {
                    Some((_, count, running)) if *running == fill => *count += 1,
                    Some(_) => {
                        if let Some((start, count, old)) = rule.take() {
                            out.push_str(&horizontal_rule(start, y, count, &old));
                        }
                        rule = Some((x, 1, fill));
                    }
                    None => rule = Some((x, 1, fill)),
                }
                continue;
            }
            if let Some((start, count, old)) = rule.take() {
                out.push_str(&horizontal_rule(start, y, count, &old));
            }
            if let Some(ch) = symbol.chars().next() {
                out.push_str(&box_glyph(ch, x, y, &fill));
            }
        }

        if let Some((start, count, fill)) = rule {
            out.push_str(&horizontal_rule(start, y, count, &fill));
        }
        if let Some(open) = run {
            out.push_str(&open.render(y));
        }
    }

    out.push_str("</g>\n</svg>\n");
    out
}

/// Whether this character is one of the six box-drawing characters stikk's panels use.
fn is_box_drawing(ch: char) -> bool {
    matches!(
        ch,
        '\u{2500}' | '\u{2502}' | '\u{250c}' | '\u{2510}' | '\u{2514}' | '\u{2518}'
    )
}

/// **Box-drawing characters are drawn, not typeset** -- which is what a terminal emulator does
/// with them, and for the same reason.
///
/// Set as text they are glyphs in whatever font the viewer happens to have, and a `\u{2502}`
/// glyph is as tall as the font's line box, not as tall as this file's cell. At a 17-pixel cell
/// and a 13-pixel font that leaves a four-pixel gap between one row's bar and the next, so a
/// panel border renders as a column of dashes. **Measured, in two different fonts**, on the first
/// render. Drawing them as rectangles makes them tile seamlessly in any viewer, and is the only
/// part of the image that does not come from the font.
fn box_glyph(ch: char, x: u16, y: u16, fill: &str) -> String {
    let px = u32::from(x) * CELL_W;
    let py = u32::from(y) * CELL_H;
    let mid_x = px + CELL_W / 2;
    let mid_y = py + CELL_H / 2;
    let right = CELL_W - CELL_W / 2;
    let left = CELL_W / 2 + STROKE;
    let below = CELL_H - CELL_H / 2;
    let above = CELL_H / 2 + STROKE;

    let rect = |x: u32, y: u32, w: u32, h: u32| {
        format!("<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{fill}\"/>\n")
    };
    match ch {
        '\u{2502}' => rect(mid_x, py, STROKE, CELL_H),
        '\u{250c}' => rect(mid_x, mid_y, right, STROKE) + &rect(mid_x, mid_y, STROKE, below),
        '\u{2510}' => rect(px, mid_y, left, STROKE) + &rect(mid_x, mid_y, STROKE, below),
        '\u{2514}' => rect(mid_x, mid_y, right, STROKE) + &rect(mid_x, py, STROKE, above),
        '\u{2518}' => rect(px, mid_y, left, STROKE) + &rect(mid_x, py, STROKE, above),
        // `\u{2500}` is coalesced by the caller; anything else is not one of the six.
        _ => String::new(),
    }
}

/// A coalesced run of `count` horizontal rules, as one rectangle.
fn horizontal_rule(x: u16, y: u16, count: u32, fill: &str) -> String {
    let px = u32::from(x) * CELL_W;
    let mid_y = u32::from(y) * CELL_H + CELL_H / 2;
    let w = count * CELL_W;
    format!("<rect x=\"{px}\" y=\"{mid_y}\" width=\"{w}\" height=\"{STROKE}\" fill=\"{fill}\"/>\n")
}

/// The styling of one cell, as the SVG cares about it.
#[derive(PartialEq, Eq)]
struct CellStyle {
    fg: String,
    bg: Option<String>,
    bold: bool,
    italic: bool,
}

impl CellStyle {
    fn of(cell: &ratatui::buffer::Cell) -> Self {
        Self {
            fg: colour(cell.fg).unwrap_or_else(|| "#d8dee4".to_string()),
            bg: colour(cell.bg),
            bold: cell.modifier.contains(Modifier::BOLD),
            italic: cell.modifier.contains(Modifier::ITALIC),
        }
    }
}

/// A run of cells sharing one style.
struct Run {
    x: u16,
    style: CellStyle,
    text: String,
}

impl Run {
    fn start(x: u16, style: CellStyle, symbol: &str) -> Self {
        Self {
            x,
            style,
            text: symbol.to_string(),
        }
    }

    fn render(&self, y: u16) -> String {
        if self.text.trim().is_empty() && self.style.bg.is_none() {
            // Nothing to draw: blank cells on the page background.
            return String::new();
        }
        let px = u32::from(self.x) * CELL_W;
        let py = u32::from(y) * CELL_H;
        let mut out = String::new();
        if let Some(bg) = &self.style.bg {
            let w = u32::try_from(self.text.chars().count()).unwrap_or(0) * CELL_W;
            out.push_str(&format!(
                "<rect x=\"{px}\" y=\"{py}\" width=\"{w}\" height=\"{CELL_H}\" fill=\"{bg}\"/>\n"
            ));
        }
        if self.text.trim().is_empty() {
            return out;
        }
        let weight = if self.style.bold {
            " font-weight=\"bold\""
        } else {
            ""
        };
        let slant = if self.style.italic {
            " font-style=\"italic\""
        } else {
            ""
        };
        let fill = &self.style.fg;
        let baseline = py + BASELINE;
        // **`textLength` pins the run to the grid.** Without it each glyph advances by the
        // *font's* own width, which is not this file's `CELL_W` -- and over 80 columns the drift
        // put the panel's right border inside the frame (seen in the first render). `spacing`
        // adjusts the gaps between glyphs and leaves their shapes alone, so the characters stay
        // the font's and only the grid is enforced.
        let span = u32::try_from(self.text.chars().count()).unwrap_or(0) * CELL_W;
        out.push_str(&format!(
            "<text x=\"{px}\" y=\"{baseline}\" textLength=\"{span}\" lengthAdjust=\"spacing\" \
             fill=\"{fill}\"{weight}{slant}>{}</text>\n",
            escape_xml(&self.text)
        ));
        out
    }
}

/// `None` for the terminal's own default, which the page background already provides.
fn colour(c: Color) -> Option<String> {
    let (r, g, b) = match c {
        Color::Reset => return None,
        Color::Rgb(r, g, b) => (r, g, b),
        // The eight ANSI colours stikk uses, as the values a reader would see in a common dark
        // terminal. Named rather than guessed per-terminal: the image has to pick one.
        Color::Black => (0x1b, 0x20, 0x24),
        Color::Red => (0xe0, 0x6c, 0x75),
        Color::Green => (0x8c, 0xc2, 0x65),
        Color::Yellow => (0xe5, 0xc0, 0x7b),
        Color::Blue => (0x61, 0xaf, 0xef),
        Color::Magenta => (0xc6, 0x78, 0xdd),
        Color::Cyan => (0x56, 0xb6, 0xc2),
        Color::Gray | Color::White => (0xd8, 0xde, 0xe4),
        Color::DarkGray => (0x5c, 0x63, 0x70),
        Color::LightRed => (0xff, 0x8a, 0x93),
        Color::LightGreen => (0xa8, 0xd9, 0x84),
        Color::LightYellow => (0xff, 0xd7, 0x8c),
        Color::LightBlue => (0x8a, 0xc6, 0xff),
        Color::LightMagenta => (0xdb, 0x9b, 0xf0),
        Color::LightCyan => (0x7a, 0xd4, 0xdd),
        Color::Indexed(_) => (0xd8, 0xde, 0xe4),
    };
    Some(format!("#{r:02x}{g:02x}{b:02x}"))
}

/// **Every character XML-escaped.** Box-drawing characters and the views' own `\uXXXX` escapes
/// are already plain text by the time they reach a cell (RFC 036), so they pass through as they
/// are -- but `&`, `<`, `>` and `"` must not.
fn escape_xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}

// -------------------------------------------------------------------------------------------
// The privacy guard (RFC 042 decision 4).
//
// **It reads the images, and the environment only to know what must NOT be in them.** It never
// reads key material, never looks for any, and never prints a value it matched -- it names the
// rule and the file, so a failure cannot itself leak the thing it caught.

/// One thing an image must not contain.
struct Rule {
    /// What the rule is called, which is all a failure prints.
    name: &'static str,
    /// Whether `svg` breaks it.
    broken_by: fn(&str, &[String]) -> bool,
}

/// Every rule, in the order the handoff lists them.
fn rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "a home-directory path (/home/, /Users/ or C:\\Users\\)",
            broken_by: |svg, _| {
                svg.contains("/home/") || svg.contains("/Users/") || svg.contains("C:\\Users\\")
            },
        },
        Rule {
            name: "this machine's own HOME, USER/USERNAME or host name",
            broken_by: |svg, _| machine_words().iter().any(|w| svg.contains(w.as_str())),
        },
        Rule {
            name: "an @ followed by a domain",
            broken_by: |svg, _| looks_like_an_email(svg),
        },
        Rule {
            name: "a 64-hex id that is not one of the invented ones",
            broken_by: |svg, invented| undeclared_hex(svg, invented).is_some(),
        },
        Rule {
            name: "the word `seed` or `secret`",
            broken_by: |svg, _| {
                let lower = svg.to_lowercase();
                lower.contains("seed") || lower.contains("secret")
            },
        },
        Rule {
            name: "a character the escape rules would have flagged",
            // **Except the document's own line breaks.** `needs_unicode_escape` answers `true`
            // for `\n`, and an SVG is written one element per line -- so the rule would fire on
            // the file's own structure. No *cell* can hold a newline: by the time repository text
            // reaches a cell it has been through the escape rules, which turn one into the two
            // characters `\` and `n` (RFC 036). Nothing real is lost by skipping it, and the
            // planted-fault test proves the rule still fires on an override.
            broken_by: |svg, _| {
                svg.chars()
                    .any(|c| c != '\n' && stikk_prikk::escape::needs_unicode_escape(c))
            },
        },
    ]
}

/// The values this machine would leak, read **only** so they can be forbidden.
///
/// Short or trivial values are skipped: a `USER` of `ci` would match half the English in an
/// image and say nothing about privacy.
fn machine_words() -> Vec<String> {
    let mut out = Vec::new();
    for key in [
        "HOME",
        "USER",
        "USERNAME",
        "LOGNAME",
        "HOSTNAME",
        "COMPUTERNAME",
    ] {
        if let Ok(value) = std::env::var(key) {
            let value = value.trim().to_string();
            if value.chars().count() >= 3 && value != "/" {
                out.push(value);
            }
        }
    }
    // The last path component of HOME too: an image could carry the user name without the path.
    if let Ok(home) = std::env::var("HOME")
        && let Some(name) = std::path::Path::new(&home).file_name()
        && name.to_string_lossy().chars().count() >= 3
    {
        out.push(name.to_string_lossy().into_owned());
    }
    out
}

/// An `@` with something domain-shaped after it. Deliberately loose: a false positive costs one
/// look at an image, and a false negative is the thing this exists to prevent.
fn looks_like_an_email(svg: &str) -> bool {
    svg.split('@').skip(1).any(|after| {
        let host: String = after
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
            .collect();
        match host.rsplit_once('.') {
            Some((name, tld)) => !name.is_empty() && tld.chars().count() >= 2,
            None => false,
        }
    })
}

/// The first 64-hex run that is not a declared invented id, if there is one.
fn undeclared_hex(svg: &str, invented: &[String]) -> Option<String> {
    let chars: Vec<char> = svg.chars().collect();
    let mut run = 0usize;
    for at in 0..chars.len() {
        if chars[at].is_ascii_hexdigit() {
            run += 1;
        } else {
            run = 0;
            continue;
        }
        if run >= 64 {
            let found: String = chars[at + 1 - 64..=at].iter().collect();
            if !invented.iter().any(|id| id.eq_ignore_ascii_case(&found)) {
                return Some(found);
            }
        }
    }
    None
}

/// Check one image. Returns the names of the rules it breaks.
fn broken_rules(svg: &str) -> Vec<&'static str> {
    let invented = invented_ids();
    rules()
        .into_iter()
        .filter(|rule| (rule.broken_by)(svg, &invented))
        .map(|rule| rule.name)
        .collect()
}

// -------------------------------------------------------------------------------------------
// The tests.

fn image_dir() -> PathBuf {
    // `CARGO_MANIFEST_DIR` is this crate; the images live at the workspace root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(IMAGE_DIR)
}

/// **Write the images.** Ignored by default: it is the one command that regenerates them.
///
/// ```sh
/// cargo test -p stikk-tui regenerate_readme_images -- --ignored
/// ```
#[test]
#[ignore = "writes docs/assets/readme/; run it on purpose when a view changes"]
fn regenerate_readme_images() {
    let dir = image_dir();
    std::fs::create_dir_all(&dir).expect("create the image directory");
    for scene in scenes() {
        std::fs::write(dir.join(scene.name), scene.svg.as_bytes())
            .unwrap_or_else(|e| panic!("write {}: {e}", scene.name));
    }
}

/// **The images on disk are what the current views draw**, byte for byte. A view change that
/// would alter a picture fails here rather than leaving the README showing last month's UI.
#[test]
fn readme_images_are_current() {
    for scene in scenes() {
        let path = image_dir().join(scene.name);
        let committed = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "{}: {e}\n\nRegenerate with:\n    cargo test -p stikk-tui \
                 regenerate_readme_images -- --ignored",
                scene.name
            )
        });
        assert!(
            committed == scene.svg,
            "{} is out of date with the views that draw it.\n\nRegenerate with:\n    \
             cargo test -p stikk-tui regenerate_readme_images -- --ignored",
            scene.name
        );
    }
}

/// **Nothing of this machine, or of any real repository, is in an image.**
///
/// Run over the committed files, not the freshly rendered ones, so it also covers an image edited
/// by hand after generation.
#[test]
fn the_committed_images_carry_nothing_private() {
    for scene in scenes() {
        let path = image_dir().join(scene.name);
        let committed =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", scene.name));
        let broken = broken_rules(&committed);
        assert!(
            broken.is_empty(),
            // **The rule and the file, never the value.** A guard that printed what it matched
            // would publish the thing it exists to keep out.
            "{} breaks the privacy rules: {broken:?}",
            scene.name
        );
    }
}

/// **Every rule fires.** A guard nobody has seen fail is a guard nobody knows works, so each rule
/// gets a planted fault -- in a scratch copy of an image, never in a committed one.
#[test]
fn every_privacy_rule_fires_on_a_planted_fault() {
    let clean = scenes().swap_remove(0).svg;
    assert!(
        broken_rules(&clean).is_empty(),
        "the real image must be clean: {:?}",
        broken_rules(&clean)
    );

    let home_leak = std::env::var("HOME")
        .ok()
        .filter(|h| h.chars().count() >= 3);
    let planted: Vec<(&str, String)> = vec![
        (
            "a home-directory path (/home/, /Users/ or C:\\Users\\)",
            clean.replace("example-project", "/home/someone/example-project"),
        ),
        (
            "an @ followed by a domain",
            clean.replace("example-project", "someone@example.com"),
        ),
        (
            "a 64-hex id that is not one of the invented ones",
            clean.replace("example-project", &"0123456789abcdef".repeat(4)),
        ),
        (
            "the word `seed` or `secret`",
            clean.replace("example-project", "author.seed"),
        ),
        (
            "a character the escape rules would have flagged",
            clean.replace("example-project", "example\u{202e}project"),
        ),
    ];
    for (rule, svg) in planted {
        let broken = broken_rules(&svg);
        assert!(
            broken.contains(&rule),
            "the planted fault for {rule:?} did not fire; fired: {broken:?}"
        );
    }

    // The machine rule needs this machine to have a usable HOME to plant.
    if let Some(home) = home_leak {
        let svg = clean.replace("example-project", &home);
        assert!(
            broken_rules(&svg).contains(&"this machine's own HOME, USER/USERNAME or host name"),
            "the planted HOME did not fire"
        );
    }
}

/// The invented ids really are 64 hex characters, so the guard's own rule can match them.
#[test]
fn every_invented_id_is_a_plausible_object_id() {
    for id in invented_ids() {
        assert_eq!(id.chars().count(), 64, "{id}");
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()), "{id}");
    }
}

/// XML's five dangerous characters never reach the output raw.
#[test]
fn the_writer_escapes_xml() {
    assert_eq!(
        escape_xml("a & b < c > d \" e"),
        "a &amp; b &lt; c &gt; d &quot; e"
    );
    for scene in scenes() {
        // Every `<` in the file opens a tag: no cell content can have introduced one.
        let tags = scene.svg.matches('<').count();
        let closes = scene.svg.matches('>').count();
        assert_eq!(tags, closes, "{} has unbalanced angle brackets", scene.name);
    }
}
