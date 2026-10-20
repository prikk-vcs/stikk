//! Opening a bundle file: what prikk says it **is**, and what prikk says it would **change**
//! (design `FR-091`; RFC 050 Handoff A).
//!
//! **This half writes nothing.** Import and export are RFC 050's Handoff B; nothing here spawns
//! `bundle import` or `bundle export`, and stikk creates no file of its own.
//!
//! Two shapes of prikk's answer meet here, and they are kept apart on purpose.
//!
//! - **`bundle verify` is prose, shown and not read.** It ends with prikk's own caveat that the
//!   check is structural only and *"a verified bundle is not yet a trusted one"*. That sentence
//!   is the reason the view exists at all, so it travels verbatim rather than being summarised
//!   into a word like *valid*.
//! - **`bundle preview` is JSON, read strictly.** Its `connectivity` and `conflict` are prikk's
//!   closed vocabularies, and stikk turns the pair into **one sentence of its own** — because
//!   *"diverged / applies-cleanly"* is prikk's shorthand for a reader who already knows what
//!   those words mean here, and the reader this view is for does not.
//!
//! **A bundle prikk will not verify is never previewed.** A malformed file has nothing to
//! preview, and asking anyway would put a second refusal under the first for the same cause.

use std::path::Path;

use stikk_model::Result;
use stikk_prikk::{
    BundleConflict, BundlePreview, BundlePreviewRead, BundleVerify, Connectivity, Prikk,
};

/// The first prikk minor that offers this, re-exported so a view says the number the seam gates
/// on -- the one-constant rule every other band follows.
pub use stikk_prikk::EXCHANGE_FROM_MINOR as BUNDLE_FROM_MINOR;

/// **stikk's own reason, below the band** (decision 6; `C-T4d`), given the minor prikk reported.
#[must_use]
pub fn bundle_unavailable(minor: u32) -> String {
    format!(
        "Opening a bundle needs prikk ≥ 0.{BUNDLE_FROM_MINOR} — this prikk is 0.{minor}. The \
         bundle commands are older than that, but 0.{BUNDLE_FROM_MINOR} is the version stikk has \
         measured, and stikk does not read a report it has not checked. The rest of stikk works."
    )
}

/// How many effect rows the Bundle view draws before it says how many are left.
///
/// **A bound, not a guess at the terminal's height**: the view scrolls, so the limit is not
/// about what fits on a screen — it is about a bundle carrying thousands of paths and burying
/// prikk's verification under them. The number is the same one Patch detail and the confirmation
/// cards use for a long list, so a reader meets one length of list in stikk and not three.
pub const BUNDLE_EFFECTS_SHOWN: usize = 50;

/// What the Bundle view says when the effects did not all fit (§4c).
///
/// **It names prikk's own command**, because stikk has no other surface for a bundle's effects
/// today: there is no second view to send the reader to, so the honest route is the one prikk
/// already offers.
#[must_use]
pub fn unshown_effects_line(unshown: usize, total: usize) -> String {
    format!(
        "and {unshown} more not shown of {total} — `prikk bundle preview --input <file>` lists \
         every one"
    )
}

/// **The bundle brings a ref this repository has never had.** Nothing of yours is compared,
/// because there is nothing of that name to compare with.
pub const NEW_TO_THIS_REPOSITORY: &str =
    "This is new here: you have no branch of that name, so nothing of yours is being compared.";

/// **The bundle and the local branch of the same name share no history** — two lineages that
/// happen to carry one name.
pub const SHARES_NOTHING: &str = "This has nothing in common with your branch of that name: the two were started separately \
     and share no block. Importing stores it beside yours; it joins nothing.";

/// **The local branch already holds everything the bundle carries.**
pub const ALREADY_HELD: &str =
    "Your branch of that name already has all of it. Importing changes nothing you can see.";

/// **The bundle continues the local branch**, with nothing local it lacks.
pub const WOULD_BRING_FORWARD: &str = "This continues your branch of that name: it would bring it forward, with nothing of yours \
     left behind.";

/// **Both sides moved on, and prikk's replay succeeds.**
pub const DIVERGED_CLEAN: &str = "Both sides have moved on since they parted, and prikk's preview applies this on top of \
     yours cleanly.";

/// **Both sides moved on, and prikk's replay fails.** prikk's own reason is shown below this,
/// verbatim (`ER-02`), because why it conflicts is prikk's to say and not stikk's to summarise.
pub const DIVERGED_CONFLICT: &str = "Both sides have moved on since they parted, and this does not apply on top of yours. prikk's \
     reason is below, in its own words.";

/// **Both sides moved on, and prikk could not tell.** Not *it conflicts* and not *it is fine*:
/// `UD-02`'s rule applied to a word prikk chose for exactly this case.
pub const DIVERGED_UNDETERMINED: &str = "Both sides have moved on since they parted, and prikk could not determine whether this \
     applies on top of yours.";

/// What the Bundle view has to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleOpened {
    /// prikk verified the file, and previewed it against the local branch.
    Read(Box<BundleView>),
    /// prikk would not read the file. **Its words, verbatim**, and nothing else — no preview
    /// was asked for.
    ///
    /// **The path travels with the refusal** (review v1 C1). prikk names the file in some of
    /// its refusals and not in others — *"malformed persisted data: …"* names none — so a
    /// reader who mistyped a path and a reader whose download is damaged would otherwise read
    /// the same screen, with no way to check which file was opened.
    Refused {
        /// The file, **resolved**, exactly as [`BundleView::path`] carries it.
        path: String,
        /// prikk's refusal, verbatim (`ER-02`).
        message: String,
    },
    /// stikk did not ask prikk at all, and says why in its own voice.
    Unavailable(String),
}

/// prikk's two answers about one bundle file, and stikk's sentence for the second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleView {
    /// The file, **resolved** against the directory stikk was started in, as the reader sees it.
    ///
    /// Repository-adjacent text from outside stikk, rendered through `inert` at every cell
    /// (`C-T2a`).
    pub path: String,
    /// `bundle verify`'s lines, **verbatim and in order** -- prikk's caveat among them.
    pub verified: Vec<String>,
    /// `bundle preview`'s report.
    pub preview: BundlePreviewRead,
    /// stikk's own sentence for this connectivity and conflict. **No name is in it**: the branch
    /// names are drawn beside it, through `identifier`, so repository text and stikk's chrome
    /// stay apart (`C-T2a`, RFC 044 C1).
    pub sentence: &'static str,
}

impl BundleView {
    /// prikk's replay error, when there is one — shown verbatim beneath [`Self::sentence`].
    #[must_use]
    pub fn conflict_detail(&self) -> Option<&str> {
        self.preview.conflict_detail.as_deref()
    }
}

/// stikk's sentence for prikk's connectivity and conflict.
///
/// **The pair is read together, not the connectivity alone**: `diverged` is three different
/// situations for a reader, and the word on its own tells them nothing they can act on.
///
/// # Errors
/// [`stikk_model::StikkError::Environment`] for a combination prikk's own construction cannot
/// produce — a `fast-forward` that does not apply cleanly. prikk replays a fast-forward onto a
/// tip the bundle already contains, so the replay cannot fail; a report that says it did is one
/// stikk misread, and `UD-02` refuses rather than inventing a sentence for it.
pub fn sentence_for(
    connectivity: Connectivity,
    conflict: Option<BundleConflict>,
) -> Result<&'static str> {
    Ok(match (connectivity, conflict) {
        (Connectivity::NoLocalHistory, _) => NEW_TO_THIS_REPOSITORY,
        (Connectivity::DoesNotConnect, _) => SHARES_NOTHING,
        (Connectivity::AlreadyIncluded, _) => ALREADY_HELD,
        (Connectivity::FastForward, Some(BundleConflict::AppliesCleanly) | None) => {
            WOULD_BRING_FORWARD
        }
        (Connectivity::Diverged, Some(BundleConflict::AppliesCleanly)) => DIVERGED_CLEAN,
        (Connectivity::Diverged, Some(BundleConflict::Conflict)) => DIVERGED_CONFLICT,
        (Connectivity::Diverged, Some(BundleConflict::Undetermined) | None) => {
            DIVERGED_UNDETERMINED
        }
        (Connectivity::FastForward, Some(other)) => {
            return Err(stikk_model::StikkError::environment_msg(format!(
                "prikk reports a fast-forward that is `{}`. A fast-forward replays onto a tip the \
                 bundle already contains, so it cannot fail to apply — stikk does not invent a \
                 sentence for a combination prikk's own construction cannot produce.",
                other.name()
            )));
        }
    })
}

/// Open the bundle at `path`: verify it, and — only if that succeeded — preview it.
///
/// `path` is passed as it should be run and **shown as it is passed**: the caller resolves it
/// against the directory stikk was started in, so what the view prints is the file prikk read.
///
/// **The preview compares against the local branch of the bundle's own name** (prikk's default,
/// decision 3): `local_ref` is left to prikk, which derives it from the file rather than stikk
/// deriving a name of its own. Choosing another branch is decision 7's *not in this RFC*.
///
/// # Errors
/// Propagates every [`stikk_model::StikkError`] the preview raises — a report of another shape is
/// an environment error (`UD-02`). **A refusal of the file itself is not an error here**: prikk's
/// words become [`BundleOpened::Refused`], because a malformed bundle is the ordinary answer to
/// pointing stikk at the wrong file, not a fault of the session.
pub fn open_bundle(prikk: &impl Prikk, repo: &Path, path: &Path) -> Result<BundleOpened> {
    let shown = path.display().to_string();
    let verified = match prikk.bundle_verify(path) {
        Ok(BundleVerify::Verified(lines)) => lines,
        Ok(BundleVerify::Unavailable) => {
            let minor = prikk.handshake()?.version.minor;
            return Ok(BundleOpened::Unavailable(bundle_unavailable(minor)));
        }
        // **prikk's words, and no preview** (§4a): a file prikk will not read has nothing to
        // preview, and asking anyway would stack a second refusal under the first for one cause.
        Err(error) => {
            return Ok(BundleOpened::Refused {
                path: shown,
                message: error.to_string(),
            });
        }
    };
    let preview = match prikk.bundle_preview(repo, path, None)? {
        BundlePreview::Read(read) => *read,
        // Unreachable in practice -- the same band gates both calls, and the verify above
        // answered. Carried rather than asserted: a band that ever differs must not panic.
        BundlePreview::Unavailable => {
            let minor = prikk.handshake()?.version.minor;
            return Ok(BundleOpened::Unavailable(bundle_unavailable(minor)));
        }
    };
    let sentence = sentence_for(preview.connectivity, preview.conflict)?;
    Ok(BundleOpened::Read(Box::new(BundleView {
        path: shown,
        verified,
        preview,
        sentence,
    })))
}

#[cfg(test)]
mod tests;
