//! Preview-first, structurally: the gate every mutation in 0.4.0 sits behind (design `FR-120`/`FR-121`,
//! `OPL-01…05`, `CT-05`, `TU-09`; RFC 013).
//!
//! Three functions, three types, one guarantee: **an execution that skipped the preview, or skipped
//! confirmation, is unrepresentable — not merely forbidden.** [`preview`] is the only producer of
//! [`PreviewToken`]; [`confirm`] is the only producer of [`ConfirmedToken`]; [`execute`] is the only
//! consumer of the latter, and it takes it **by value**, so it cannot be replayed. Neither token has a
//! public constructor and there is no `From`/`Into` between them.
//!
//! The [`stikk_model::ChangeToken`] (RFC 003) is checked **twice**: once in [`confirm`] (catches the
//! world moving while the user read the preview) and once at the top of [`execute`] (the real guard,
//! immediately before whatever the seam call will be). Either mismatch is
//! [`stikk_model::StikkError::Stale`] — never a retry (`NFR-S04`): the correct next step is a fresh
//! [`preview`], not trying the same execution again.
//!
//! **No mutation ships in this increment.** [`preview`]/[`execute`] are generic over what an operation
//! actually computes/runs, so the machinery is exercised end-to-end here by a scripted, non-mutating
//! test vehicle rather than a real seam call — the seam gains its first mutating method only when
//! commit does.

use std::path::Path;
use stikk_model::{Binding, RoleReadiness};

use stikk_model::{
    Capability, ChangeToken, Readiness, RequestCategory, Result, SigningRequirement, StikkError,
    Tier,
};
use stikk_prikk::Prikk;

/// What a preview intends to do (design `OPL-01`; RFC 013 §2). Carries the request category, which
/// determines the tier ([`RequestCategory::tier`]) — never declared separately, so a new intent cannot
/// forget to be gated. `operation` is stikk's own short name for it (`"commit"`, `"seal"`, …), used
/// verbatim in [`StikkError::Stale`]/[`StikkError::Declined`] messages; never prikk's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Intent {
    /// The request category this intent belongs to — determines the confirmation tier.
    pub category: RequestCategory,
    /// A short, stikk-authored name for this operation (e.g. `"commit"`).
    pub operation: &'static str,
}

/// One value a confirmation restates, with stikk's own word for what it is (RFC 047 review v1 C1).
///
/// **The label and the value are kept apart on purpose.** The role is stikk's chrome, written in
/// this crate and never sourced from a repository; the id is repository text and must reach the
/// terminal through `identifier`/`inert` like every other repository string (`C-T2a`). Formatting
/// the label into the id would hand a bare `String` to the renderer with no way to tell which half
/// is which, and the escape would then be applied to stikk's own words or skipped on prikk's.
///
/// **Most operations name one thing and need no label.** A merge names four, two of them branches
/// whose order is the whole meaning of the operation -- the measured cost of the wrong `into` is a
/// worktree left behind its branch (RFC 047 F3) -- so there the role is not decoration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetId {
    /// stikk's own word for this value's role on the card, or `None` for an unlabelled row.
    pub role: Option<&'static str>,
    /// The repository string itself. Never contains the role.
    pub id: String,
}

impl TargetId {
    /// An unlabelled row: the operation names one thing, and the card's own title says what.
    #[must_use]
    pub fn bare(id: impl Into<String>) -> Self {
        Self {
            role: None,
            id: id.into(),
        }
    }

    /// A row the card labels, for an operation that names several things of different kinds.
    #[must_use]
    pub fn named(role: &'static str, id: impl Into<String>) -> Self {
        Self {
            role: Some(role),
            id: id.into(),
        }
    }
}

/// The `TU-09` fact set a confirmation restates — fixed and uniform, not a per-operation blob (RFC 013
/// Q1). Composed once, at preview time, and carried unchanged through to [`confirm`]; never re-derived
/// there, which would reintroduce the drift this token exists to prevent.
///
/// Every string is stikk-authored or a prikk-authoritative id (`C-T4e`) — never a display string a
/// repository could influence. Render `target_ids` through `inert` at the frontend, as everywhere else
/// repository-sourced text reaches a cell (`C-T2a`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmationSummary {
    /// The operation's human name (e.g. `"Commit"`).
    pub operation: String,
    /// The target ids and names involved, from prikk-authoritative values.
    pub target_ids: Vec<TargetId>,
    /// Labelled counts (e.g. `("patches", 3)`) — a label stikk authored, a number prikk reported.
    pub counts: Vec<(&'static str, u64)>,
    /// The capability this operation consumes (`AUTHOR`/`MAINTAINER`) — informative only for
    /// tier-3-typed, which is gated on Operator readiness (`AC-04`), not the signing ladder.
    ///
    /// **`None` means the operation consumes none**, and the card says so (RFC 045 §3c). A branch
    /// switch signs nothing: prikk needs no key for it (F1, measured), so naming a capability
    /// here would state a requirement stikk invented. It is an absence of consumption, never an
    /// absence of information.
    pub capability: Option<Capability>,
    /// What becomes permanent, and what does not — in stikk's own words.
    pub consequence: String,
    /// The exact name the user must type back for a tier-3-typed confirmation (`FR-102`/`FR-103`).
    /// `None` for every other tier.
    pub target_name: Option<String>,
    /// The key id that will sign this operation, if one is configured (`FL-05` step 5; `FL-06` as
    /// amended by RFC 023 Handoff B) — read from the environment by
    /// [`stikk_prikk::key_id`], which is a separate module from the presence-only
    /// [`stikk_prikk::env`] for a reason worth reading there.
    ///
    /// **One field, not two.** Whichever role the operation consumes is the one whose id belongs here;
    /// `capability` already says which role that is, so a pair of fields would let them disagree.
    ///
    /// **`None` is an absence and must render as nothing** — never `"(unknown)"` or any other
    /// placeholder. An operation reaching a confirmation without the id it needs is already impossible
    /// (`capability_gate` refuses on `NotReady` first), so a placeholder here could only ever be stikk
    /// claiming to know something it does not.
    ///
    /// A key id is a **label**, not key material: prikk prints it itself, and `C-I1b` already has the
    /// UI showing key ids and nothing else. It is still repository-adjacent text and renders through
    /// `inert` (`C-T2a`) like every other string here.
    pub signing_key_id: Option<String>,
    /// What the card may honestly claim about [`Self::signing_key_id`] (RFC 026 §5, following prikk's
    /// own guidance in letter 007 §2).
    ///
    /// **The id alone is not a statement.** On a repository where nothing has signed under this id
    /// yet, naming it plainly claims a binding that does not exist — and that is the *default* state
    /// of every freshly created repository, so it is the first commit a new user makes.
    pub signing_key_claim: KeyClaim,
    /// Set when the key that would sign is one prikk **publishes as an example** (`C-S2`).
    ///
    /// **Persistent, not a one-time notice.** It is recomputed for every confirmation from the key
    /// actually in effect, so a user who dismissed it once and kept working meets it again at the next
    /// one — which is what `FR-104`'s "persistently" asks for, and what makes it a control rather than
    /// a toast.
    pub signing_key_is_published_example: bool,
    /// Safeguard 3 (RFC 029 Handoff B §5): stikk's words when the target differs from prikk's current
    /// branch, or prikk reports that branch unresolved. `None` when there is nothing to say — the target
    /// is prikk's current branch, or prikk reports none (below 0.42).
    ///
    /// **A notice, not a block.** Committing or sealing another branch is legitimate, and prikk allows
    /// it. Computed from the Orientation the preview itself reads, never from a frontend's older copy.
    /// It names refs and may carry prikk's own text, so it renders through `inert` (`C-T2a`).
    pub branch_notice: Option<String>,
    /// What a seal freezes, named (RFC 028 decision 4; Handoff B §3). `None` for commit.
    ///
    /// **The one part of the card allowed to give up height** (Handoff B §4): the consequence, its
    /// trust-refusal warning and the affordances are never clipped to make room for it, and when it gives
    /// up rows it says how many, in [`unshown_patches_line`]'s words.
    pub freezes: Option<FrozenPatches>,
    /// RFC 032 decision 5, A2: stikk's words when the target has no published history, chosen from
    /// prikk's `refs()` and queue facts. `None` for a published ref, and for seal. Names a ref: render
    /// it inert.
    pub history_notice: Option<String>,
    /// RFC 032 decision 4: [`crate::RENAMES_ALSO_COUNTED`] when the counts include paired renames, else
    /// `None`.
    pub rename_note: Option<String>,
    /// RFC 032 decision 2: one sentence per declaration prikk will not author as a rename, from
    /// [`crate::DeclaredRename::notice`]. Carries paths: render each inert.
    pub declaration_notices: Vec<String>,
    /// Cautions about what confirming will produce, in **stikk's own words**, drawn above the
    /// consequence and in warn (RFC 053 decision 5).
    ///
    /// **Not [`Self::declaration_notices`]**, which is RFC 032's and is drawn only on a card that
    /// has counts — a card with none would have carried these and shown nothing, which is how
    /// this field came to exist. And not the consequence either: the consequence says what the
    /// operation does, and these say what the *result* will and will not be.
    ///
    /// Empty for every operation that has none.
    pub notices: Vec<String>,
    /// RFC 044 decision 6: the message a new tag will carry, restated so the confirmation shows
    /// what is about to be recorded. `None` for every other operation, and for a tag with no
    /// message — prikk refuses an empty one, so a blank message is no message, and the
    /// consequence says so in stikk's own words.
    ///
    /// **The one string here the user wrote**, rather than stikk or prikk. It is held in its own
    /// field for exactly that reason: it must reach the card through `inert` like every
    /// repository-sourced string, and burying it inside [`Self::consequence`] -- which this
    /// type's own contract says a repository can never influence -- would smuggle it past that.
    pub message_note: Option<String>,
    /// RFC 045 decision 5: the files a switch will write and delete. `None` for every operation
    /// that writes no file.
    ///
    /// **The one part of a switch's card allowed to give up height**, as [`Self::freezes`] is for
    /// seal: the counts, the consequence and the affordances are never clipped to make room for
    /// paths, and when paths give up rows the card says how many, in
    /// [`unshown_paths_line`]'s words. Every path is repository text: render each inert
    /// (`C-T2a`).
    pub materializes: Option<FilePlan>,
    /// RFC 047 decision 6: prikk's own merge plan, verbatim, restated under the card so what is
    /// confirmed is the plan and not the idea. `None` for every other operation.
    ///
    /// **The one part of a merge's card allowed to give up height**, as [`Self::freezes`] is for
    /// seal and [`Self::materializes`] for a switch: the consequence and the affordances are
    /// never clipped to make room for it, and when it gives up rows the card says how many, in
    /// [`unshown_plan_lines_line`]'s words. Every line is prikk's text: render each inert
    /// (`C-T2a`).
    pub merge_plan: Option<Vec<String>>,
    /// RFC 050 review v1 C1: labelled asides — facts the card must state that are **not targets
    /// and not consequences of confirming**.
    ///
    /// A target row says *this is what the operation acts on*. A sentence sitting among the
    /// target rows is read the same way, and for an import that was wrong: prikk's preview
    /// describes how the bundle relates to a branch the import **does not touch**, and read as
    /// a target row it promised that branch would move. Each aside carries its own label, so
    /// what it is cannot be inferred from where it sits.
    ///
    /// Empty for every operation that has none.
    pub asides: Vec<Aside>,
}

/// One labelled aside on a confirmation card (RFC 050 review v1 C1).
///
/// **The label is stikk's chrome and the name is repository text**, kept in separate fields for
/// the reason [`TargetId`] keeps its two apart: a frontend escapes the one and draws the other
/// as itself, and it cannot do that to a string that has already been joined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aside {
    /// stikk's words before the name (e.g. `"Compared with your"`). Drawn as itself.
    pub before: &'static str,
    /// The repository string the label is about — **render through `identifier`** (`C-T2a`).
    pub name: String,
    /// stikk's words after the name (e.g. `"(importing leaves it as it is):"`). Drawn as itself.
    pub after: &'static str,
    /// The sentences under it, stikk's or prikk's — **render each through `inert`**.
    pub body: Vec<String>,
}

/// What a switch will do to the worktree, from stikk's own Compare (RFC 045 decision 5).
///
/// **stikk classifies prikk's entries; it computes no part of the comparison** (RFC 038 §7 trap
/// 1). The mapping is measured, not assumed: at 0.52.0 a `diff` listing one `modified`, one
/// `deleted` and one `added` was followed by a switch reporting **written 2, deleted 1**, with
/// the worktree then holding exactly the target's files (RFC 045 F4).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FilePlan {
    /// Paths the switch will write — prikk's `added` and `modified`, and a rename's destination.
    pub written: Vec<String>,
    /// Paths the switch will delete — prikk's `deleted`, and a rename's source.
    pub deleted: Vec<String>,
}

impl FilePlan {
    /// Classify a comparison into the files a switch will write and delete, or `None` when the
    /// plan would not be whole (RFC 045 §6 trap 2).
    ///
    /// **`None` for anything unmeasured.** A status word stikk has not measured is kept and shown
    /// by Compare, which is right there — it is prikk's word about a difference, and a reader
    /// loses nothing by seeing it. Here it would be a **hole in a plan the user is about to
    /// confirm**, and `FR-053` asks them to confirm what will happen. The same for any
    /// `unsupported_paths`: prikk could not look at that path, which is not the same fact as
    /// having looked and found nothing (`C-T2c′`).
    #[must_use]
    pub fn classify(diff: &stikk_prikk::Diff) -> Option<Self> {
        if !diff.unsupported_paths.is_empty() {
            return None;
        }
        let mut plan = Self::default();
        for entry in &diff.entries {
            match entry.status.as_str() {
                // A rename moves the bytes: the destination is written and the source goes.
                // prikk reports it as one entry carrying both paths.
                "renamed" => {
                    plan.written.push(entry.path.clone());
                    plan.deleted.push(
                        entry
                            .from_path
                            .clone()
                            .unwrap_or_else(|| entry.path.clone()),
                    );
                }
                "added" | "modified" | "mode" | "binary" => plan.written.push(entry.path.clone()),
                "deleted" => plan.deleted.push(entry.path.clone()),
                // Measured status words are the five above and `renamed` (RFC 038 decision 2).
                // Anything else is a change stikk cannot place in the plan.
                _ => return None,
            }
        }
        Some(plan)
    }

    /// True when the switch writes and deletes nothing — the two branches hold the same files.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.written.is_empty() && self.deleted.is_empty()
    }

    /// Every path, written first then deleted, each with stikk's own one-word mark.
    ///
    /// **The mark is stikk's and the path is prikk's**, which is why they are returned apart: the
    /// frontend styles the mark as chrome and renders the path inert.
    #[must_use]
    pub fn rows(&self) -> Vec<(&'static str, &str)> {
        self.written
            .iter()
            .map(|path| ("written", path.as_str()))
            .chain(self.deleted.iter().map(|path| ("deleted", path.as_str())))
            .collect()
    }
}

/// The line a switch confirmation shows when it cannot hold every path row (RFC 045 §3c):
/// `unshown` is exactly the rows not shown, and `total` every path in the plan. The frontend
/// decides only how many rows fit, never what this says.
///
/// **A fact, and no key to press now**, for the reason [`unshown_patches_line`] gives: `Esc`
/// cancels, so telling a reader to press it mid-confirmation would send them out of the flow.
///
/// **It names the whole route, not one key** (RFC 045 review v1 C2). Where the plan can be seen
/// whole is Compare, over the same two refs this plan was made from — and `d` alone does not get
/// there: in the ref picker it answers *"mark a ref with m first"*. RFC 041's route is **`m` on
/// one branch, then `d` on the other**, so that is what this says.
#[must_use]
pub fn unshown_paths_line(unshown: usize, total: usize) -> String {
    let route =
        format!("Compare lists all {total} (in the ref picker: m on one branch, d on the other)");
    if unshown >= total {
        format!("none shown here — {route}")
    } else {
        format!("and {unshown} more not shown — {route}")
    }
}

/// What a merge card says when prikk's plan did not fit (RFC 047 decision 6).
///
/// **A fact, and no key to press now**, for the reason [`unshown_patches_line`] gives: `Esc`
/// cancels, so telling a reader to press it mid-confirmation would send them out of the flow.
/// Where the plan is whole is the Merge plan view this confirmation was opened from, which is
/// still underneath it — so that is what this names.
#[must_use]
pub fn unshown_plan_lines_line(unshown: usize, total: usize) -> String {
    let route = format!("the Merge plan view behind this card shows all {total}");
    if unshown >= total {
        format!("prikk's plan is not shown here — {route}")
    } else {
        format!("and {unshown} more lines not shown — {route}")
    }
}

/// How many characters of an object id a short id shows — the form History's block rows use, shared so a
/// seal confirmation names a patch the same way History names a block.
pub const SHORT_ID_CHARS: usize = 12;

/// What a seal confirmation names as frozen (RFC 028 decision 4; Handoff B §3). Every string is repository
/// text or carries it — an id, a message, a version — so the frontend renders each through `inert`
/// (`C-T2a`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrozenPatches {
    /// prikk ≥ 0.39: one row per queued patch, in queue order — `{short id}  {message}`, or the short id
    /// alone where prikk does not report messages — and, at 0.39–0.41, a foot saying so.
    Listed {
        /// One row per patch.
        rows: Vec<String>,
        /// `prikk 0.{minor} does not report a queued patch's message.`, at 0.39–0.41 only.
        foot: Option<String>,
    },
    /// Below prikk 0.39: `prikk 0.{minor} does not list queued patches.` One line, which never yields.
    Unlisted(String),
}

/// The line a seal confirmation shows when it cannot hold every patch row (Handoff B §3): `unshown` is
/// exactly the rows not shown, and `total` the queue's patches. The frontend decides only how many rows
/// fit, never what this says.
///
/// **A fact, and no key** (review v1 §2.1). `Esc` cancels the seal, so telling a maintainer to press it
/// mid-ceremony would quietly send them out of a signing flow; the line says where the rest can be seen,
/// and leaves the choice to them. When no row fits, `and {total} more` beneath nothing would read wrong, so
/// that case has its own words.
#[must_use]
pub fn unshown_patches_line(unshown: usize, total: usize) -> String {
    if unshown >= total {
        format!("none shown here — the Queue view lists all {total}")
    } else {
        format!("and {unshown} more not shown — the Queue view lists all {total}")
    }
}

/// The line the commit-unavailable card shows when it cannot hold every refused declaration
/// (RFC 034 Handoff C): `unshown` is exactly the declarations not shown.
///
/// **A fact, and no key.** `Esc` closes that card, so inviting it would send a reader out of the
/// one screen that is telling them why their commit is unavailable — the same reason
/// [`unshown_patches_line`] and [`unshown_paths_line`] name no key either.
///
/// **And no view, because there is none.** stikk lists refused declarations nowhere else: this
/// card is where they are. What a reader can act on is prikk's own reason, which each unshown
/// declaration has — so that is what this says, rather than pointing somewhere that would not
/// help.
///
/// **No "none shown" branch.** The card always shows at least the first declaration whole: it is
/// the one prikk's commit would stop on first, and a card that showed none of them would be
/// saying nothing at all.
#[must_use]
pub fn unshown_declarations_line(unshown: usize) -> String {
    if unshown == 1 {
        return "and 1 more declaration prikk would refuse, with its own reason".to_string();
    }
    format!("and {unshown} more declarations prikk would refuse, each with its own reason")
}

/// How firmly a confirmation may state the signing key id (RFC 026 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyClaim {
    /// No id claim at all — there is nothing prikk could tell stikk, or no id to name.
    #[default]
    None,
    /// prikk reports the key binds to what this repository already records. **The only state that
    /// earns a plain statement.**
    Bound,
    /// prikk reports no signature under this id yet: it will bind on this first signature. Stated as
    /// such — an honest confirmation, not a hedge.
    Unbound,
    /// stikk could not ask (prikk ≤ 0.40). The id is the one stikk would pass, and stikk does not know
    /// whether prikk will use it — which is exactly what RFC 026 F4 found stikk claiming without
    /// warrant, so it is said rather than implied.
    Unchecked,
}

/// Safeguard 3 (RFC 029 Handoff B §5): what a commit's or seal's confirmation says about prikk's current
/// branch, in stikk's words, or `None` when there is nothing to say.
///
/// **One function, both ceremonies**, for the same reason as [`signing_key_claim`]. `target` is the
/// previewed ref; `current` is what the preview's own Orientation read reported. The words rest on a
/// measurement, not on prikk's help text (`C-T2b`): at prikk 0.42 a raw `prikk commit` with no `--ref`
/// queues for the ref `.prikk/current-branch` names (re-measured for this handoff).
///
/// Below 0.42 prikk reports no current branch and this says nothing; stikk infers none.
#[must_use]
pub fn branch_notice(target: &str, current: &stikk_model::CurrentBranch) -> Option<String> {
    match current {
        stikk_model::CurrentBranch::NotReported => None,
        stikk_model::CurrentBranch::Branch(branch) if branch.as_str() == target => None,
        stikk_model::CurrentBranch::Branch(branch) => Some(format!(
            "This targets {target}. prikk's current branch is {}, the ref prikk uses when no --ref \
             is given.",
            branch.as_str()
        )),
        // **Not *"prikk reports"*** (RFC 057 decision 3): at prikk ≥ 0.43 the report says only
        // `null`, and the sentence after it is stikk's own.
        stikk_model::CurrentBranch::Unresolved(text) => Some(format!(
            "This targets {target}. prikk's current branch is {text}."
        )),
    }
}

/// The id to show and how firmly to claim it, for one role (RFC 026 §5).
///
/// **One function, both ceremonies**, so commit's card and seal's card cannot drift into two answers
/// to one question — which is the drift RFC 015 found between `stikk-tui`'s copy and the ceiling it
/// was describing.
///
/// The id comes from prikk where prikk has it (≥ 0.41, via `key status`), and from the environment
/// below that. **That is what fixes the silent case**: `PRIKK_<ROLE>_KEY_ID` unset is not "no id" —
/// prikk defaults to the role's own name and signs as it — so reading only the variable rendered
/// nothing on the default setup while prikk had an answer all along (RFC 026 Handoff B §0).
#[must_use]
pub fn signing_key_claim(
    role: RoleReadiness,
    detail: &stikk_prikk::RoleDetail,
    env_fallback: Option<String>,
) -> (Option<String>, KeyClaim) {
    let id = detail.key_id.clone().or(env_fallback);
    let Some(id) = id else {
        return (None, KeyClaim::None);
    };
    match role {
        // prikk answered.
        RoleReadiness::Known(Binding::Matches) => (Some(id), KeyClaim::Bound),
        RoleReadiness::Known(Binding::Unrecorded) => (Some(id), KeyClaim::Unbound),
        // These withhold the capability (`Capability::derive`), so a confirmation is unreachable —
        // named rather than defaulted so a future state cannot acquire a silent claim here.
        RoleReadiness::Known(Binding::NotAdopted | Binding::Mismatch) => (Some(id), KeyClaim::None),
        RoleReadiness::Known(Binding::Absent) => (None, KeyClaim::None),
        // stikk could not ask. The id is real — it is what stikk would pass — but the binding is not
        // a claim stikk has warrant for.
        RoleReadiness::Unknown | RoleReadiness::Unverifiable => (Some(id), KeyClaim::Unchecked),
        RoleReadiness::NotReady => (None, KeyClaim::None),
    }
}

/// What the user supplied to satisfy a tier's confirmation requirement (design `TU-09`; RFC 013 §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evidence {
    /// Tier 2/3: an explicit yes.
    ExplicitYes,
    /// Tier 3-typed: the user's typed input, checked against the summary's `target_name`.
    TypedName(String),
}

/// Produced only by [`preview`]. Single-use and step-scoped (RFC 013 Q2): one preview→confirm→execute
/// step, never held open across a multi-step ceremony. No public constructor and no `From`/`Into`
/// toward [`ConfirmedToken`] — the only way to get one is to preview.
#[derive(Debug)]
pub struct PreviewToken {
    intent: Intent,
    summary: ConfirmationSummary,
    change_token: ChangeToken,
    tier: Tier,
}

impl PreviewToken {
    /// The tier this preview's confirmation must satisfy — read to decide which evidence shape (an
    /// explicit yes, or a typed name) the confirmation surface should collect.
    #[must_use]
    pub fn tier(&self) -> Tier {
        self.tier
    }

    /// The fact set the confirmation surface renders. Never re-derive from a fresh read at confirm
    /// time (RFC 013 §3) — this is the one true copy.
    #[must_use]
    pub fn summary(&self) -> &ConfirmationSummary {
        &self.summary
    }
}

/// Produced only by [`confirm`]. The only thing [`execute`] accepts, and it takes this **by value**, so
/// it cannot be replayed — an execution that skipped confirmation is as unrepresentable as one that
/// skipped the preview (RFC 013, "a decision that followed from Q1/Q2").
#[derive(Debug)]
pub struct ConfirmedToken {
    intent: Intent,
    change_token: ChangeToken,
}

/// The result of a completed [`execute`] — the operation's name and whatever its own execution
/// produced (a patch id, a block id, …). Generic because no real mutation ships this increment; a real
/// operation supplies its own `T`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome<T> {
    /// The operation that ran, stikk's own name for it.
    pub operation: String,
    /// Whatever the operation itself produced.
    pub result: T,
}

/// Compute a preview and stamp it with the repository's current change token (design `LC-4`/`OPL-02`).
///
/// Generic over what the operation itself computes: `compute` builds the view the user will actually
/// look at (a `ChangesView`, prikk's own plan text, …) plus the [`ConfirmationSummary`] restating it.
/// The change token is read **before** `compute` runs, so a token stamped here reflects the state the
/// preview was genuinely computed under.
///
/// # Errors
/// Propagates any [`StikkError`] the seam or `compute` raises.
pub fn preview<V>(
    prikk: &impl Prikk,
    repo: &Path,
    intent: Intent,
    compute: impl FnOnce() -> Result<(V, ConfirmationSummary)>,
) -> Result<(V, PreviewToken)> {
    let change_token = prikk.change_token(repo)?;
    let (view, summary) = compute()?;
    let tier = intent.category.tier();
    Ok((
        view,
        PreviewToken {
            intent,
            summary,
            change_token,
            tier,
        },
    ))
}

/// Check whether `readiness` satisfies what `tier` requires, independent of staleness or evidence
/// (design `FR-121`, `OPL-04`). Public and standalone — per `internal-design.md`'s `OPL-04`, this same
/// check serves **two** roles that must never drift apart: a frontend calls it directly for UI
/// affordance (disabling a command with the reason shown, before any preview exists — RFC 007's
/// `Command::unmet_reason` is today's Viewer/Author/Maintainer-only version of this idea; a tier-aware
/// operation should call this instead), and [`confirm`] calls it as its own first, structural check
/// (`OPL-04`'s "first check"). `confirm` uses it, but does not own it.
///
/// Tier 1 always passes (RFC 013 decision 6: free, no exception). Read-only refuses every category
/// above it outright (`NFR-S01`), via [`Readiness::may_operate`] — the method RFC 012 F-a moved to
/// `Readiness` precisely because [`Capability`] had already discarded the fact it needs. Above
/// that, the **category's own** [`stikk_model::SigningRequirement`] decides which key, if any,
/// must be present.
///
/// **Keyed on the category, not on the tier** (RFC 045 §2b). The two agreed for every category
/// that existed before `Materialization`, which is why the old tier-keyed form was correct for as
/// long as it was: it mapped Tier Three to MAINTAINER. A branch switch is Tier Three and needs no
/// key at all (RFC 045 F1, measured), so keeping the tier as the key would have demanded a
/// maintainer key for an operation that signs nothing — a requirement stikk invented.
///
/// # Errors
/// [`StikkError::NotReady`] when read-only mode or capability is insufficient.
pub fn capability_gate(
    operation: &str,
    category: RequestCategory,
    readiness: Readiness,
) -> Result<()> {
    if category.tier() == Tier::One {
        return Ok(());
    }
    if !readiness.may_operate() {
        return Err(StikkError::NotReady {
            detail: format!("{operation} is not available in read-only mode"),
        });
    }
    let capability = Capability::derive(readiness);
    match category.signing_requirement() {
        SigningRequirement::Author if !capability.may_author() => Err(StikkError::NotReady {
            detail: format!("{operation} needs AUTHOR signing readiness"),
        }),
        SigningRequirement::Maintainer if !capability.may_publish() => Err(StikkError::NotReady {
            detail: format!("{operation} needs MAINTAINER signing readiness"),
        }),
        _ => Ok(()),
    }
}

/// Consume a preview and produce a [`ConfirmedToken`] — the only way to get one (RFC 013, "a decision
/// that followed from Q1/Q2"). This is where every gate lives (`OPL-04`'s first check): read-only and
/// capability ([`capability_gate`]), then a fresh change-token comparison
/// ([`StikkError::Stale`] on any difference — the world moved while the
/// user was looking at the preview), then the evidence itself
/// ([`StikkError::Declined`] if it does not satisfy the tier).
///
/// # Errors
/// [`StikkError::NotReady`] if read-only mode or capability is insufficient; [`StikkError::Stale`] if
/// the repository changed since the preview; [`StikkError::Declined`] if `evidence` does not satisfy
/// `token`'s tier; any [`StikkError`] the seam's change-token read raises.
pub fn confirm(
    prikk: &impl Prikk,
    repo: &Path,
    token: PreviewToken,
    readiness: Readiness,
    evidence: Evidence,
) -> Result<ConfirmedToken> {
    capability_gate(token.intent.operation, token.intent.category, readiness)?;

    let current = prikk.change_token(repo)?;
    if current != token.change_token {
        return Err(StikkError::Stale {
            operation: token.intent.operation.to_string(),
            cause: stikk_model::StaleCause::Repository,
        });
    }

    let satisfied = match (token.tier, &evidence) {
        (Tier::One, _) => true, // never reached in practice: tier 1 never calls confirm
        (Tier::Two | Tier::Three, Evidence::ExplicitYes) => true,
        (Tier::ThreeTyped, Evidence::TypedName(typed)) => {
            !typed.is_empty() && token.summary.target_name.as_deref() == Some(typed.as_str())
        }
        _ => false,
    };
    if !satisfied {
        return Err(StikkError::Declined {
            detail: format!(
                "{} was not confirmed as this tier requires",
                token.intent.operation
            ),
        });
    }

    Ok(ConfirmedToken {
        intent: token.intent,
        change_token: token.change_token,
    })
}

/// Consume a [`ConfirmedToken`] and run the operation, re-checking the change token immediately first
/// (`OPL-02`'s real guard). Takes `confirmed` **by value**: once used, it is gone, so an execution
/// cannot be replayed by holding onto the token.
///
/// # Errors
/// [`StikkError::Stale`] if the repository changed since confirmation; otherwise whatever `run` or the
/// seam's change-token read raises.
pub fn execute<T>(
    prikk: &impl Prikk,
    repo: &Path,
    confirmed: ConfirmedToken,
    run: impl FnOnce() -> Result<T>,
) -> Result<Outcome<T>> {
    let current = prikk.change_token(repo)?;
    if current != confirmed.change_token {
        return Err(StikkError::Stale {
            operation: confirmed.intent.operation.to_string(),
            cause: stikk_model::StaleCause::Repository,
        });
    }
    let result = run()?;
    Ok(Outcome {
        operation: confirmed.intent.operation.to_string(),
        result,
    })
}

#[cfg(test)]
mod tests;
