//! The Orientation view (design VW-01, FR-002; use case UC-01).
//!
//! Renders the `stikk_core::OrientationView` the operation layer produces: prikk version and support,
//! the session's derived capability, signing readiness, queue depth, a torn-tail warning, and the
//! `heads/main` state. Every repository-sourced string is routed through [`crate::text::inert`] before
//! it reaches a cell (threat model C-T2a).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use stikk_core::OrientationView;
use stikk_model::{Binding, RoleReadiness};

use crate::text::inert;
use crate::theme::Palette;

/// Render the Orientation view into `area`.
pub fn render(view: &OrientationView, palette: &Palette, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    // RFC 009 decisions 6–7: the range has two ends that behave differently. Below the floor stikk
    // degrades to read-only; above the validated ceiling it still runs, but says so rather than
    // silently asserting a validation it has not done (text-first, NFR-A03 — never colour alone).
    let (support_text, support_style) = if !view.prikk_supported {
        (
            "outside stikk's validated range — read-only".to_string(),
            Style::default()
                .fg(palette.warn)
                .add_modifier(Modifier::BOLD),
        )
    } else if !view.prikk_validated {
        (
            // `validated_through` comes from `stikk_prikk::validated_ceiling_display` (RFC 015) —
            // never hardcode this number here again: it drifted silently once already (still said
            // "0.30" after RFC 012 F-e had raised the real ceiling to 31, caught only while re-basing
            // to 0.32) precisely because nothing forced this copy to move with the constant.
            format!(
                "validated through {} — this prikk is newer; its output shapes have not been checked \
                 against stikk",
                view.validated_through
            ),
            Style::default().fg(palette.warn),
        )
    } else {
        ("supported".to_string(), Style::default().fg(palette.dim))
    };
    lines.push(field(
        palette,
        "prikk",
        vec![
            Span::styled(inert(&view.prikk_version), Style::default().fg(palette.fg)),
            Span::raw("  "),
            Span::styled(support_text, support_style),
        ],
    ));

    lines.push(field(
        palette,
        "capability",
        vec![Span::styled(
            view.capability.name(),
            Style::default().fg(palette.accent),
        )],
    ));

    lines.push(field(
        palette,
        "signing",
        vec![Span::raw(signing_line(view))],
    ));

    let mut queued_spans = vec![Span::styled(
        view.queued_patches.to_string(),
        Style::default().fg(palette.fg),
    )];
    // RFC 009 F1/F4: showing the queue's target ref (now that the parser carries it) is strictly more
    // honest than a bare count — it is the same fact behind the Changes view's queued-elsewhere
    // warning.
    if let Some(target) = &view.queued_target {
        queued_spans.push(Span::styled(
            " · targeting ",
            Style::default().fg(palette.dim),
        ));
        queued_spans.push(Span::styled(inert(target), Style::default().fg(palette.fg)));
    }
    lines.push(field(palette, "queued", queued_spans));

    if view.trailing_partial_wal_bytes != 0 {
        lines.push(field(
            palette,
            "warning",
            vec![Span::styled(
                format!(
                    "{} trailing partial WAL byte(s) — an interrupted commit left a torn tail",
                    view.trailing_partial_wal_bytes
                ),
                Style::default().fg(palette.warn),
            )],
        ));
    }

    let main_ref = match &view.main_ref_state {
        Some(id) => inert(id),
        None => "<unpublished>".to_string(),
    };
    lines.push(field(
        palette,
        "heads/main",
        vec![Span::styled(main_ref, Style::default().fg(palette.fg))],
    ));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Orientation ")
        .style(Style::default().fg(palette.fg));
    // The unvalidated-ceiling notice (RFC 009 decision 7) is long enough to overflow a narrow terminal;
    // wrap rather than silently truncate it out of view (design TU-11).
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

/// A `label: value` line with the label dimmed and left-padded to a column.
fn field<'a>(palette: &Palette, label: &'a str, mut value: Vec<Span<'a>>) -> Line<'a> {
    let mut spans = vec![Span::styled(
        format!("  {label:<12}"),
        Style::default().fg(palette.dim),
    )];
    spans.append(&mut value);
    Line::from(spans)
}

/// The signing-readiness summary (never key material — presence only, design C-I1).
///
/// **This is where the two unknowns are told apart.** The status bar's one-cell badge cannot carry the
/// distinction and renders both as `?`; here there is room for a sentence, and the sentence is the
/// difference between *"stikk cannot check this"* and *"stikk cannot see whether there is anything to
/// check"*. Neither renders as a pass (`C-T2c′`), and the second names its fix, because a user on
/// prikk 0.40 has one (RFC 026 Q1(b)).
fn signing_line(view: &OrientationView) -> String {
    let mut s = format!(
        "author {} · maintainer {}",
        role_words(view.readiness.author, Role::Author),
        role_words(view.readiness.maintainer, Role::Maintainer),
    );
    if view.readiness.read_only {
        s.push_str(" · read-only");
    }
    if view.readiness.author == RoleReadiness::Unverifiable
        || view.readiness.maintainer == RoleReadiness::Unverifiable
    {
        // Named cause, named fix. The band is one prikk version wide and has a remedy, so saying only
        // "unknown" would be honest and useless.
        s.push_str(
            " — prikk 0.40 moved signing keys to a key directory and does not report them; \
             prikk 0.41 answers this directly",
        );
    }
    if view.stale_seed_variables.any() {
        // RFC 026 §4: the variable is set, this prikk ignores it, and it is not what will sign. A user
        // who exported it has every reason to believe otherwise.
        s.push_str(" — a PRIKK_*_SEED variable is set; this prikk ignores it");
    }
    s
}

/// Which role a word is being written for — the two differ in exactly one state.
#[derive(Clone, Copy)]
enum Role {
    Author,
    Maintainer,
}

/// One role's state, in words.
///
/// **`Unknown` says different things for the two roles**, because the unanswerable question differs:
/// for MAINTAINER it is trust-policy *adoption* (RFC 016 F3's wording, kept), and for AUTHOR there is
/// no adoption to speak of — what is unknown is whether the key binds. Saying "adoption unknown" of an
/// author key would name a mechanism that does not apply to it.
fn role_words(readiness: RoleReadiness, role: Role) -> &'static str {
    match readiness {
        RoleReadiness::NotReady => "not ready",
        RoleReadiness::Unknown => match role {
            Role::Author => "present, unverified",
            Role::Maintainer => "present, adoption unknown",
        },
        RoleReadiness::Unverifiable => "unknown",
        RoleReadiness::Known(Binding::Matches) => "ready",
        // Can sign, and the card that matters says what will happen (RFC 026 §5). "ready" alone would
        // overstate a key that is about to be bound for the first time.
        RoleReadiness::Known(Binding::Unrecorded) => "ready, not yet bound",
        RoleReadiness::Known(Binding::NotAdopted) => "present, not adopted by this repository",
        RoleReadiness::Known(Binding::Mismatch) => {
            "present, does not match this repository's record"
        }
        RoleReadiness::Known(Binding::Absent) => "not ready",
    }
}

#[cfg(test)]
mod tests;
