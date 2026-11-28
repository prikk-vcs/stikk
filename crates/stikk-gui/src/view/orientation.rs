//! **The Orientation screen** (design VW-01, FR-002), drawn to RFC 069's rules.
//!
//! The same facts, in the same order, in the same words as the terminal's own Orientation (rule
//! 11): prikk's version and support, the session's capability, signing readiness, the queue, a
//! torn tail, prikk's current branch, and `heads/main`. **Every repository-sourced string goes
//! through [`crate::text`]** (threat model C-T2a), and every id, ref and path is isolated by the
//! design crate (rule 12).
//!
//! **One row is the desktop's own:** which preset is drawing, and who decided. It is the
//! counterpart of the terminal's `colours` row (RFC 059 decision 10) — the one row that says
//! nothing about the repository, and the first thing to ask about when a user says the window
//! looks wrong.

use iced::Element;
use iced::widget::Column;
use stikk_app::OrientationState;
use stikk_core::OrientationView;
use stikk_gui_design as design;
use stikk_gui_design::Tokens;
use stikk_model::{Binding, CurrentBranch, RoleReadiness};

use crate::theme::Source;
use crate::words::{Label, Say};
use crate::{Message, Stikk, text, view};

/// The screen.
pub fn view<'a>(
    state: &'a OrientationState,
    stikk: &'a Stikk,
    tokens: &Tokens,
) -> Element<'a, Message> {
    match state {
        OrientationState::Loading => design::body(
            tokens,
            Say::LoadingScreen.fill(&[("screen", "orientation")]),
        )
        .into(),
        // prikk's own message, verbatim (NFR-I03), under the prose rule.
        OrientationState::Failed(said) => design::body(tokens, text::inert(said)).into(),
        OrientationState::Loaded(loaded) => loaded_view(loaded, stikk, tokens),
    }
}

fn loaded_view<'a>(
    loaded: &'a OrientationView,
    stikk: &'a Stikk,
    tokens: &Tokens,
) -> Element<'a, Message> {
    let mut rows = Column::new().spacing(tokens.spacing.sm);

    // prikk's version line is prose — prikk's own sentence, not an identifier — beside stikk's own
    // word about whether this version is one it validated.
    rows = rows.push(view::row(
        tokens,
        Label::Prikk.word(),
        iced::widget::column![
            design::body(tokens, text::inert(&loaded.prikk_version)),
            design::meta(tokens, support(loaded)),
        ]
        .spacing(tokens.spacing.xs)
        .into(),
    ));
    rows = rows.push(view::row(
        tokens,
        Label::Capability.word(),
        design::body(tokens, loaded.capability.name()).into(),
    ));
    rows = rows.push(view::row(
        tokens,
        Label::Signing.word(),
        design::body(tokens, signing(loaded)).into(),
    ));
    rows = rows.push(view::row(
        tokens,
        Label::Queued.word(),
        queued(loaded, tokens),
    ));
    // RFC 034 §7: a checkout or branch switch that stopped part-way. **A state of the repository,
    // not an error** — the lead-in is stikk's, and each route is prikk's own command, shown whole
    // and wrapped rather than clipped, because a clipped command is worse than none.
    if let Some(routes) = &loaded.interrupted_materialization {
        let mut lines = Column::new().spacing(tokens.spacing.xs).push(design::body(
            tokens,
            Say::Interrupted.fill(&[("sentence", stikk_core::INTERRUPTED_MATERIALIZATION)]),
        ));
        for route in routes {
            lines = lines.push(design::identifier(tokens, &text::identifier(route)));
        }
        rows = rows.push(view::row(tokens, Label::Interrupted.word(), lines.into()));
    }
    if loaded.trailing_partial_wal_bytes != 0 {
        rows = rows.push(view::row(
            tokens,
            Label::Warning.word(),
            design::body(
                tokens,
                Say::TornTail.fill(&[("bytes", &loaded.trailing_partial_wal_bytes.to_string())]),
            )
            .into(),
        ));
    }
    rows = rows.push(branch(loaded, tokens));
    let main = match &loaded.main_ref_state {
        Some(id) => design::identifier(tokens, &text::identifier(id)),
        None => design::body(tokens, Say::Unpublished.template()),
    };
    rows = rows.push(view::row(tokens, Label::HeadsMain.word(), main.into()));
    rows = rows.push(view::row(
        tokens,
        Label::Drawing.word(),
        design::meta(tokens, drawing(stikk)).into(),
    ));
    rows.into()
}

/// **Whether stikk validated this prikk**, in stikk's own words.
///
/// The range has two ends that behave differently (RFC 009 decisions 6–7): below the floor stikk
/// degrades to read-only; above the validated ceiling it still runs, and says so rather than
/// asserting a validation it has not done. **The ceiling's number is never written here** — it
/// comes from the view, which reads `stikk_prikk`'s own constant.
fn support(loaded: &OrientationView) -> String {
    if !loaded.prikk_supported {
        Say::PrikkUnsupported.template().to_string()
    } else if !loaded.prikk_validated {
        Say::PrikkUnvalidated.fill(&[("through", &loaded.validated_through)])
    } else {
        Say::PrikkSupported.template().to_string()
    }
}

/// **Signing readiness, presence only** — never key material (design C-I1).
fn signing(loaded: &OrientationView) -> String {
    let mut text = Say::Signing.fill(&[
        ("author", role_words(loaded.readiness.author, Role::Author)),
        (
            "maintainer",
            role_words(loaded.readiness.maintainer, Role::Maintainer),
        ),
    ]);
    if loaded.readiness.read_only {
        text.push_str(" · ");
        text.push_str(Say::SigningReadOnly.template());
    }
    if loaded.readiness.author == RoleReadiness::Unverifiable
        || loaded.readiness.maintainer == RoleReadiness::Unverifiable
    {
        // Named cause, named fix. The band is one prikk version wide and has a remedy, so saying
        // only "unknown" would be honest and useless.
        text.push_str(" — ");
        text.push_str(Say::SigningUnverifiable.template());
    }
    if loaded.stale_seed_variables.any() {
        // RFC 026 §4: the variable is set, this prikk ignores it, and it is not what will sign.
        text.push_str(" — ");
        text.push_str(Say::SigningStaleSeed.template());
    }
    text
}

/// Which role a word is being written for — the two differ in exactly one state.
#[derive(Clone, Copy)]
enum Role {
    Author,
    Maintainer,
}

/// One role's state, in words — the terminal's own, character for character (rule 11).
///
/// **`Unknown` says different things for the two roles**, because the unanswerable question
/// differs: for MAINTAINER it is trust-policy *adoption*, and for AUTHOR there is no adoption to
/// speak of — what is unknown is whether the key binds.
fn role_words(readiness: RoleReadiness, role: Role) -> &'static str {
    match readiness {
        RoleReadiness::NotReady => "not ready",
        RoleReadiness::Unknown => match role {
            Role::Author => "present, unverified",
            Role::Maintainer => "present, adoption unknown",
        },
        RoleReadiness::Unverifiable => "unknown",
        RoleReadiness::Known(Binding::Matches) => "ready",
        RoleReadiness::Known(Binding::Unrecorded) => "ready, not yet bound",
        RoleReadiness::Known(Binding::NotAdopted) => "present, not adopted by this repository",
        RoleReadiness::Known(Binding::Mismatch) => {
            "present, does not match this repository's record"
        }
        RoleReadiness::Known(Binding::Absent) => "not ready",
    }
}

/// The queue's depth, and the ref it targets when prikk reports one.
fn queued<'a>(loaded: &'a OrientationView, tokens: &Tokens) -> Element<'a, Message> {
    let mut lines = Column::new()
        .spacing(tokens.spacing.xs)
        .push(design::body(tokens, loaded.queued_patches.to_string()));
    if let Some(target) = &loaded.queued_target {
        lines = lines.push(design::meta(
            tokens,
            Say::QueuedTargeting.fill(&[("ref", &design::isolate(&text::identifier(target)))]),
        ));
    }
    lines.into()
}

/// prikk's current branch: an unresolved answer is a sentence, not a name (RFC 057 decision 3).
fn branch<'a>(loaded: &'a OrientationView, tokens: &Tokens) -> Element<'a, Message> {
    match &loaded.current_branch {
        CurrentBranch::Unresolved(said) => view::row(
            tokens,
            Label::Branch.word(),
            design::body(tokens, text::inert(said)).into(),
        ),
        CurrentBranch::Branch(branch) => view::row(
            tokens,
            Label::Branch.word(),
            iced::widget::column![
                design::identifier(tokens, &text::identifier(branch.as_str())),
                design::meta(tokens, Say::CurrentBranch.template()),
            ]
            .spacing(tokens.spacing.xs)
            .into(),
        ),
        CurrentBranch::NotReported => view::row(
            tokens,
            Label::Branch.word(),
            design::body(tokens, Say::Unpublished.template()).into(),
        ),
    }
}

/// **Which preset is drawing, and who decided** (rule 3).
fn drawing(stikk: &Stikk) -> String {
    let choice = stikk.theme_choice();
    let source = match choice.source() {
        Source::Desktop => Say::DrawingFollows,
        Source::Configured => Say::DrawingChosen,
        Source::ConfiguredLevelOnly => Say::DrawingChosenLevel,
    };
    Say::Drawing.fill(&[
        ("preset", stikk.preset().name()),
        ("source", source.template()),
    ])
}
