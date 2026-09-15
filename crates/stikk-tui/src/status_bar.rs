//! The status bar (design TU-03; handoff §2 `status_bar.rs`; RFC 010).
//!
//! One line: repository, focused ref (never "HEAD" — it does not exist), with prikk's current branch
//! beside it as `prikk's default: <branch>` when the two differ, or `no ref focused` (RFC 029 Handoff B
//! §3), queue depth, worktree marker, the `⟳ n` background-operation indicator
//! (TU-03; RFC 010 — the count of requests the worker has not yet answered), and the
//! capability/readiness badges. Every badge has a text form so a monochrome terminal loses nothing
//! (design NFR-A03).
//!
//! **When the line is wider than the terminal, it sheds by priority, never by position** (RFC 029 Handoff
//! B review v1 §2.1) — see [`fit`].

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use stikk_model::{Binding, CurrentBranch, Readiness, RoleReadiness};

use crate::app::{App, OrientationState, RefFocus};
use crate::text::inert;
use crate::theme::Palette;

/// The key hint, the first thing the line sheds.
const HINT: &str = "   :palette  ?:help  q:back";

/// The mark a shortened value ends with.
const ELLIPSIS: &str = "…";

/// Render the status bar for `app` into the one-line `area`.
pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let palette = app.palette();
    let parts = Parts {
        repo: app
            .repo()
            .file_name()
            .map(|n| inert(&n.to_string_lossy()))
            .unwrap_or_else(|| "?".to_string()),
        focus: focus_spans(app, palette),
        default: prikks_default(app),
        tail: tail_spans(app, palette),
    };
    frame.render_widget(
        Paragraph::new(fit(&parts, palette, usize::from(area.width))),
        area,
    );
}

/// The line's pieces, before anything is shed. Every string is already inert (`C-T2a`).
struct Parts {
    /// The repository's short name.
    repo: String,
    /// The focus, or `no ref focused`, led by its separator. Never shortened.
    focus: Vec<Span<'static>>,
    /// prikk's current branch or its unresolved text, when it belongs beside the focus.
    default: Option<String>,
    /// `(loading)`/`(error)`, or `●n queued` and the badges; then `⟳ n`. Never shortened.
    tail: Vec<Span<'static>>,
}

/// Build the line from `parts`, with the given repository name and default value, and the hint or not.
fn line(
    parts: &Parts,
    palette: &Palette,
    repo: &str,
    default: Option<&str>,
    hint: bool,
) -> Line<'static> {
    let mut spans = vec![Span::styled(
        repo.to_string(),
        Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
    )];
    spans.extend(parts.focus.iter().cloned());
    if let Some(value) = default {
        spans.push(sep(palette));
        spans.push(Span::styled(
            "prikk's default: ",
            Style::default().fg(palette.dim),
        ));
        spans.push(Span::styled(
            value.to_string(),
            Style::default().fg(palette.fg),
        ));
    }
    spans.extend(parts.tail.iter().cloned());
    if hint {
        spans.push(Span::styled(HINT, Style::default().fg(palette.dim)));
    }
    Line::from(spans)
}

/// The line that fits `width` terminal cells, shedding in this order and each step only if the line still
/// does not fit:
///
/// 1. drop the key hint;
/// 2. shorten the repository name from its end, marked with `…`, down to one character and `…` — the
///    header already shows the name, so it is the first thing worth giving up (review v2 §2);
/// 3. shorten prikk's default from its end, marked with `…`, down to `prikk's default: …`;
/// 4. drop the `prikk's default` segment, separator included.
///
/// **Never shortened:** the focus (or `no ref focused`), `(loading)`/`(error)`, `●n queued`, every badge,
/// and `⟳ n` — `[MNT ?]` must never go missing (`C-T2c′`, `FR-104`). Beyond step 4 the line clips at the
/// right edge, as it always did. Widths are cells ([`Line::width`]), not bytes or chars, because `inert` can
/// emit wide characters; shortening cuts on a character boundary and then measures.
fn fit(parts: &Parts, palette: &Palette, width: usize) -> Line<'static> {
    let default = parts.default.as_deref();
    let full = line(parts, palette, &parts.repo, default, true);
    if full.width() <= width {
        return full;
    }
    // 1. The key hint.
    let without_hint = line(parts, palette, &parts.repo, default, false);
    if without_hint.width() <= width {
        return without_hint;
    }
    // 2. The repository name, shortened from its end, down to one character.
    let repo_chars: Vec<char> = parts.repo.chars().collect();
    let mut repo = parts.repo.clone();
    for keep in (1..repo_chars.len()).rev() {
        repo = shortened(&repo_chars, keep);
        let candidate = line(parts, palette, &repo, default, false);
        if candidate.width() <= width {
            return candidate;
        }
    }
    // 3. prikk's default, shortened from its end.
    if let Some(value) = default {
        let chars: Vec<char> = value.chars().collect();
        for keep in (0..chars.len()).rev() {
            let candidate = line(parts, palette, &repo, Some(&shortened(&chars, keep)), false);
            if candidate.width() <= width {
                return candidate;
            }
        }
    }
    // 4. The whole segment. Past this the line clips at the right edge, as it always did.
    line(parts, palette, &repo, None, false)
}

/// The first `keep` characters of `chars`, marked as shortened.
fn shortened(chars: &[char], keep: usize) -> String {
    let mut out: String = chars.iter().take(keep).collect();
    out.push_str(ELLIPSIS);
    out
}

/// The focus piece, led by its separator (RFC 029 Handoff B §3's table): nothing while pending (the
/// `(loading)` or `(error)` marker says why), `no ref focused`, or the focused ref.
fn focus_spans(app: &App, palette: &Palette) -> Vec<Span<'static>> {
    match app.ref_focus() {
        RefFocus::Pending => Vec::new(),
        RefFocus::Unfocused => vec![
            sep(palette),
            Span::styled("no ref focused", Style::default().fg(palette.warn)),
        ],
        // The focused ref — a client-side pointer, not a HEAD (design FR-055).
        RefFocus::Ref(name) => vec![
            sep(palette),
            Span::styled(inert(name), Style::default().fg(palette.accent)),
        ],
    }
}

/// What goes beside a focused ref as `prikk's default: `, inert: prikk's current branch when it differs
/// from the focus, or prikk's unresolved text verbatim.
///
/// **Computed from the loaded Orientation on every render**, so it follows every Orientation read — and
/// only those: stikk reads on open, on `r`, and after a commit or seal. Between reads a terminal
/// `prikk branch switch` is not seen here, so nothing in this segment claims to be live. The label is
/// exactly `prikk's default: `, never `HEAD` or anything that reads as an authority (RFC 029 decision 4).
fn prikks_default(app: &App) -> Option<String> {
    let RefFocus::Ref(focused) = app.ref_focus() else {
        return None;
    };
    let OrientationState::Loaded(view) = app.state() else {
        return None;
    };
    match &view.current_branch {
        CurrentBranch::Branch(branch) if branch.as_str() != focused => Some(inert(branch.as_str())),
        CurrentBranch::Unresolved(text) => Some(inert(text)),
        CurrentBranch::Branch(_) | CurrentBranch::NotReported => None,
    }
}

/// Everything after the focus that is never shortened: the load marker, or the queue and the badges; then
/// the in-flight indicator.
fn tail_spans(app: &App, palette: &Palette) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    match app.state() {
        OrientationState::Loaded(view) => {
            if view.queued_patches > 0 {
                spans.push(sep(palette));
                spans.push(Span::styled(
                    format!("●{} queued", view.queued_patches),
                    Style::default().fg(palette.warn),
                ));
            }
            spans.push(sep(palette));
            spans.append(&mut badges(palette, view.readiness));
        }
        OrientationState::Loading | OrientationState::Failed(_) => {
            spans.push(sep(palette));
            spans.push(Span::styled(
                match app.state() {
                    OrientationState::Failed(_) => "(error)",
                    _ => "(loading)",
                },
                Style::default().fg(palette.dim),
            ));
        }
    }

    let in_flight = app.in_flight_count();
    if in_flight > 0 {
        spans.push(sep(palette));
        spans.push(Span::styled(
            format!("⟳ {in_flight}"),
            Style::default().fg(palette.accent),
        ));
    }
    spans
}

fn sep(palette: &Palette) -> Span<'static> {
    Span::styled("  ·  ", Style::default().fg(palette.dim))
}

/// The capability/readiness badges (design TU-03): `[RO]` when read-only, then author and maintainer
/// readiness. Text-forward so colour is never load-bearing.
fn badges(palette: &Palette, readiness: Readiness) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    if readiness.read_only {
        out.push(Span::styled(
            "[RO] ",
            Style::default()
                .fg(palette.warn)
                .add_modifier(Modifier::BOLD),
        ));
    }
    out.push(role_badge(palette, "AUT", readiness.author));
    out.push(Span::raw(" "));
    out.push(role_badge(palette, "MNT", readiness.maintainer));
    out
}

/// One role's badge (RFC 016 §4, extended to both roles by RFC 026 §3): every state rendered
/// distinctly, never collapsed to two.
///
/// **`?` and `✓` must never be shared** — `C-T2c′`: a claim stikk cannot verify must not render as a
/// pass. Three glyphs carry four ideas:
///
/// | state | glyph | why |
/// |---|---|---|
/// | `Known(Matches)` | `✓` | prikk answered and the key binds |
/// | `Known(Unrecorded)` | `✓` | prikk answered and will bind on first signature — it *can* sign |
/// | `Unknown` | `?` | key material present, binding unanswerable on this prikk |
/// | `Unverifiable` | `?` | stikk cannot see whether there is key material at all (prikk 0.40) |
/// | `Known(NotAdopted \| Mismatch)` | `✗` | prikk answered, and the answer is that it will refuse |
/// | `NotReady` | `–` | nothing to sign with |
///
/// `?` covers both unknowns deliberately at *this* size — a one-cell badge cannot carry the
/// distinction, and both mean "stikk is not claiming". The distinction that matters is in what the
/// session can *do*, which `Capability::derive` already separates, and in the Orientation view's
/// full-sentence rendering, which has room to say which unknown it is.
fn role_badge(palette: &Palette, label: &str, readiness: RoleReadiness) -> Span<'static> {
    let (mark, style) = match readiness {
        RoleReadiness::Known(Binding::Matches | Binding::Unrecorded) => {
            ("✓", Style::default().fg(palette.ok))
        }
        RoleReadiness::Known(Binding::NotAdopted | Binding::Mismatch) => {
            ("✗", Style::default().fg(palette.warn))
        }
        RoleReadiness::Known(Binding::Absent) | RoleReadiness::NotReady => {
            ("–", Style::default().fg(palette.dim))
        }
        RoleReadiness::Unknown | RoleReadiness::Unverifiable => {
            ("?", Style::default().fg(palette.warn))
        }
    };
    Span::styled(format!("[{label} {mark}]"), style)
}

#[cfg(test)]
mod tests;
