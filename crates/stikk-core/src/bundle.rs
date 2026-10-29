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

use stikk_model::{RequestCategory, Result};
use stikk_prikk::{
    BundleConflict, BundleExport, BundleImport, BundlePreview, BundlePreviewRead, BundleVerify,
    Connectivity, Prikk, bundle_tip,
};

use crate::confirm::{self, Aside, ConfirmationSummary, Evidence, Intent, Outcome, PreviewToken};

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
    /// Each recorded sealer with **whether this repository trusts it** (RFC 054 decision 3),
    /// from one `trusted_keys` read made with this one.
    ///
    /// **prikk's ids, in prikk's order**, and prikk's own continuity note still sits beneath
    /// them: what stikk adds is an answer prikk does not give, not a replacement for what it
    /// does. Every entry is [`crate::trust::SealerTrust::Unknown`] where the list could not be
    /// read, so a card says what it does not know rather than implying *not trusted*.
    pub sealers: crate::trust::SealerTrustRead,
}

impl BundleView {
    /// prikk's replay error, when there is one — shown verbatim beneath [`Self::sentence`].
    #[must_use]
    pub fn conflict_detail(&self) -> Option<&str> {
        self.preview.conflict_detail.as_deref()
    }

    /// Where importing this bundle would store it: `remotes/<bundle_ref>` (measured at 0.53).
    ///
    /// **`None` when the tip line could not be read** (RFC 050 Handoff B §3b). Without the tip
    /// there is no older-bundle guard, and the view must not offer a key whose preview would
    /// only block: the one answer, in one place, for both the key and what the view advertises.
    #[must_use]
    pub fn received_branch(&self) -> Option<String> {
        bundle_tip(&self.verified).ok()?;
        Some(format!("{RECEIVED_PREFIX}{}", self.preview.bundle_ref))
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
    // **One read for every sealer** (RFC 054 decision 3), made with this one so the view holds
    // a single answer rather than asking again per row.
    let sealers = crate::trust::sealers(prikk, repo, &preview.sealed_by);
    Ok(BundleOpened::Read(Box::new(BundleView {
        path: shown,
        verified,
        preview,
        sentence,
        sealers,
    })))
}

// -------------------------------------------------------------------------------------------
// Import (RFC 050 decision 4; Handoff B)
// -------------------------------------------------------------------------------------------

/// stikk's own short name for the operation, used in stale and declined messages.
pub const IMPORT_OPERATION: &str = "import";

/// How many blocks of the received branch's lineage the older-bundle guard reads.
///
/// **The same number the merge baseline finder reads**, and the same number History lists: a
/// bundle stikk declines for a truncated read is one whose tip the reader could not have found
/// by scrolling either.
pub const BUNDLE_LINEAGE_LIMIT: usize = 200;

/// The prefix prikk stores a received ref under, measured at 0.53: `bundle import` of
/// `heads/main` writes `remotes/heads/main`.
pub const RECEIVED_PREFIX: &str = "remotes/";

/// **Why stikk will not import a bundle the received branch already holds at its tip** (F2,
/// guard 1).
pub const ALREADY_RECEIVED: &str = "Already received: this repository's received branch is already at this bundle's tip, so \
     importing it would change nothing.";

/// **Why stikk will not import a bundle older than what was received** (F2, guard 2) — the
/// measurement this whole guard exists for.
///
/// Measured at 0.53: importing `v1` after `v2` moved `remotes/heads/main` **back** to `v1`'s
/// tip, exit 0, with the same message as a forward import. Nothing is lost — the objects stay —
/// but the received branch stops showing the newer work and prikk says nothing about it. A
/// stale file in a download folder is the ordinary way to do this.
pub const WOULD_MOVE_THE_RECEIVED_BRANCH_BACK: &str = "This bundle is older than what this repository already received: its tip is a block the \
     received branch has already moved past. Importing it would move that branch back to the \
     older block and hide the newer work, and prikk would report it as an ordinary import.";

/// **Why stikk will not import when it cannot see far enough to tell** (F2, guard 3).
///
/// The guard compares the bundle's tip with the received branch's lineage. Where that lineage
/// came back full, the tip may be older than everything read and stikk cannot distinguish that
/// from a tip that is not in the branch at all — so it declines and names the way out rather
/// than guessing in the direction that writes.
pub const LINEAGE_TOO_LONG_TO_TELL: &str = "This repository's received branch is longer than stikk reads, and this bundle's tip is not \
     among the blocks it read — so stikk cannot tell whether the bundle is older than what you \
     already have. `prikk bundle import --input <file>` imports it without this check.";

/// The sentence under the import card, in stikk's own words — **with no name in it** (RFC 044
/// C1): `consequence` is drawn unescaped, and every name is already restated in `target_ids`.
///
/// **It used to end by explaining the author-key material** (review v1 C1). The `Sealed by`
/// aside now says that, in prikk's own words and beside the ids it is about, so the clause here
/// was a second copy — and the one that cost the consequence its last two rows at 80×24, which
/// is the one part of a card that must never give them up.
const IMPORT_CONSEQUENCE: &str = "The bundle is stored as a received branch, which is read-only history beside your own. No \
     branch of yours is created or moved, nothing is written to your worktree, and nobody's \
     signing key is trusted by this.";

/// **stikk's one line after an import lands**: what arrived is read-only, and the reader is
/// now looking at it.
pub const RECEIVED_IS_READ_ONLY: &str = "This is received history: it is read beside your own branches and nothing of yours moved. \
     Taking its work into a branch of yours is a merge, which stikk does not offer from a \
     received ref yet -- `prikk merge --from` does.";

/// The label above prikk's comparison on the import card and the Bundle view (review v1 C1).
///
/// **It says what the sentence is and what it is not.** prikk's preview describes how the
/// bundle relates to your branch of the same name; an import does not touch that branch, and
/// without this the sentence — *"it would bring it forward"* — reads as the effect of pressing
/// the key.
pub const COMPARED_WITH_BEFORE: &str = "Compared with your";

/// The second half of [`COMPARED_WITH_BEFORE`], after the branch name.
pub const COMPARED_WITH_AFTER: &str = "(importing leaves it as it is):";

/// The label above the recorded signer ids, which prikk's own note then qualifies.
pub const SEALED_BY_BEFORE: &str = "Sealed by";

/// The second half of [`SEALED_BY_BEFORE`], after the ids.
pub const SEALED_BY_AFTER: &str = "\u{2014}";

/// What an import can say before it runs.
#[derive(Debug)]
pub enum ImportPreview {
    /// stikk will not run it, and says why in its own voice — or prikk's, where prikk refused
    /// to read the file at all.
    Blocked(String),
    /// Ready to confirm.
    Ready {
        /// The confirmation token.
        token: Box<PreviewToken>,
    },
}

/// The one path an import is run with, and what the card named.
///
/// **A value, not a bare `&Path`**: the received branch's name is derived from the bundle and
/// restated on the card, and carrying the two together is what keeps the card and the call
/// describing one operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportTarget {
    /// The file, resolved — what the card showed and what prikk is given.
    pub path: std::path::PathBuf,
    /// Where prikk will store it: `remotes/<bundle_ref>`.
    pub received: String,
}

/// Preview importing the bundle at `path`.
///
/// **Everything is read again here**, as a merge's preview re-reads its guards: the verification,
/// the tip, prikk's preview and the received branch's lineage. A bundle imported in another
/// terminal between the view opening and `I` is exactly the case guard 1 exists for.
///
/// **The two guards decline without spawning `import`** (§7 trap 3). They are prikk's gap, not
/// prikk's rule — prikk imports an older bundle happily and reports it as an ordinary import —
/// so stikk asks before, not after.
///
/// # Errors
/// Propagates every [`stikk_model::StikkError`] the reads raise. prikk's **refusal of the file**
/// is not an error but a block carrying prikk's own words (`ER-02`), as in [`open_bundle`].
pub fn import_preview(
    prikk: &impl Prikk,
    repo: &Path,
    path: &Path,
    limit: usize,
) -> Result<ImportPreview> {
    let shown = path.display().to_string();
    let path = path.to_path_buf();
    let intent = Intent {
        category: RequestCategory::Exchange,
        operation: IMPORT_OPERATION,
    };
    let (view, token): (ReadView, _) = confirm::preview(prikk, repo, intent, || {
        // 1. Verify. A file prikk will not read is prikk's refusal, and nothing follows it.
        let verified = match prikk.bundle_verify(&path) {
            Ok(BundleVerify::Verified(lines)) => lines,
            Ok(BundleVerify::Unavailable) => {
                let minor = prikk.handshake()?.version.minor;
                return Ok(blocked(&bundle_unavailable(minor)));
            }
            Err(error) => return Ok(blocked(&error.to_string())),
        };
        // 2. The tip, from the one pinned line. A shape error blocks: without the tip there is
        //    no guard, and an import with no guard is the one F2 measured.
        let tip = match bundle_tip(&verified) {
            Ok(tip) => tip,
            Err(error) => return Ok(blocked(&error.to_string())),
        };
        // 3. prikk's preview, for the bundle's own ref name and the sentence the card restates.
        let preview = match prikk.bundle_preview(repo, &path, None)? {
            BundlePreview::Read(read) => *read,
            BundlePreview::Unavailable => {
                let minor = prikk.handshake()?.version.minor;
                return Ok(blocked(&bundle_unavailable(minor)));
            }
        };
        let sentence = sentence_for(preview.connectivity, preview.conflict)?;
        let received = format!("{RECEIVED_PREFIX}{}", preview.bundle_ref);
        // 4. The two guards, against the received branch's own lineage -- read only where that
        //    branch exists, because there is nothing to move back otherwise.
        if prikk
            .refs(repo)?
            .refs
            .iter()
            .any(|entry| entry.name == received)
            && let Some(reason) = older_than_received(prikk, repo, &received, &tip, limit)?
        {
            return Ok(blocked(&reason));
        }
        Ok((
            ReadView::Ready,
            ConfirmationSummary {
                operation: "Import".to_string(),
                // **Labelled, because neither value speaks for itself** (RFC 047 C1's rule): a
                // bare `remotes/heads/main` beside a bare short id leaves a reader unable to
                // say which is the branch being written and which the block being brought in.
                target_ids: vec![
                    confirm::TargetId::named("received as", received.clone()),
                    confirm::TargetId::named("tip", short(&tip)),
                    confirm::TargetId::named("file", shown.clone()),
                ],
                counts: Vec::new(),
                // **No key** (decision 2): `Exchange` asks for none, and the card must not
                // claim one is consumed.
                capability: None,
                consequence: IMPORT_CONSEQUENCE.to_string(),
                target_name: None,
                signing_key_id: None,
                signing_key_claim: confirm::KeyClaim::None,
                signing_key_is_published_example: false,
                branch_notice: None,
                freezes: None,
                history_notice: None,
                rename_note: None,
                declaration_notices: Vec::new(),
                notices: Vec::new(),
                message_note: None,
                materializes: None,
                merge_plan: None,
                // **Not targets** (review v1 C1). prikk's comparison is about a branch this
                // operation does not touch, and prikk's continuity note is about the ids it
                // sits with -- each gets a label saying which, so neither is read as a row in
                // the list of things the import acts on.
                asides: vec![
                    Aside {
                        before: COMPARED_WITH_BEFORE,
                        name: preview.local_ref.clone(),
                        after: COMPARED_WITH_AFTER,
                        body: vec![sentence.to_string()],
                    },
                    Aside {
                        before: SEALED_BY_BEFORE,
                        name: preview.sealed_by.join(", "),
                        after: SEALED_BY_AFTER,
                        // **One line per sealer, then prikk's note** (RFC 054 decision 3).
                        // The read is inside `confirm::preview`, so it is re-taken at
                        // confirmation like every other fact on this card.
                        body: sealer_lines(prikk, repo, &preview.sealed_by)
                            .into_iter()
                            .chain(std::iter::once(preview.sealed_by_note.clone()))
                            .collect(),
                    },
                ],
            },
        ))
    })?;
    Ok(match view {
        ReadView::Blocked(reason) => ImportPreview::Blocked(reason),
        ReadView::Ready => ImportPreview::Ready {
            token: Box::new(token),
        },
    })
}

/// The two guards F2 and F3 wrote, against the received branch's lineage.
///
/// `Ok(None)` means the bundle is newer than everything received, which is the only shape that
/// imports. **Nothing here spawns `import`.**
fn older_than_received(
    prikk: &impl Prikk,
    repo: &Path,
    received: &str,
    tip: &str,
    limit: usize,
) -> Result<Option<String>> {
    let lineage = prikk.history(repo, received, limit)?;
    let Some(head) = lineage.blocks.first() else {
        // A received ref with no blocks is not a ref this guard can say anything about, and an
        // import that creates history where there was none is the ordinary forward case.
        return Ok(None);
    };
    if head.block_id == tip {
        return Ok(Some(ALREADY_RECEIVED.to_string()));
    }
    if lineage.blocks.iter().any(|block| block.block_id == tip) {
        return Ok(Some(WOULD_MOVE_THE_RECEIVED_BRANCH_BACK.to_string()));
    }
    // **A full read may be a truncated one** -- the same reading `merge_baseline` gives a
    // lineage that came back at the limit. The tip is not among what was read; whether it is
    // older than what was not read is exactly what stikk cannot see.
    if lineage.blocks.len() >= limit {
        return Ok(Some(LINEAGE_TOO_LONG_TO_TELL.to_string()));
    }
    Ok(None)
}

/// Import the bundle this token was previewed for.
///
/// # Errors
/// As [`import_preview`], plus every refusal prikk raises.
pub fn import_confirm_and_execute(
    prikk: &impl Prikk,
    repo: &Path,
    token: PreviewToken,
    readiness: stikk_model::Readiness,
    evidence: Evidence,
    target: &ImportTarget,
) -> Result<Outcome<BundleImport>> {
    let confirmed = confirm::confirm(prikk, repo, token, readiness, evidence)?;
    confirm::execute(prikk, repo, confirmed, || {
        prikk.bundle_import(repo, &target.path)
    })
}

/// One line per sealer: prikk's id, and what this repository says about it (RFC 054).
///
/// **The id is repository text and the sentence is stikk's**, joined here only because a card's
/// aside body is lines of prose. A frontend that wants them apart has
/// [`BundleView::sealers`].
fn sealer_lines(prikk: &impl Prikk, repo: &Path, sealed_by: &[String]) -> Vec<String> {
    crate::trust::sealers(prikk, repo, sealed_by)
        .judged()
        .iter()
        .map(|sealer| format!("{} — {}", sealer.key_id, sealer.trust.sentence()))
        .collect()
}

/// An object id, shortened the way History, seal and every preview already shorten one.
fn short(id: &str) -> String {
    short_id(id)
}

/// [`short`], for the other modules that shorten the same ids the same way — RFC 053's saves
/// among them. **One function, so two previews cannot disagree about what a short id is.**
pub(crate) fn short_id(id: &str) -> String {
    id.chars().take(crate::confirm::SHORT_ID_CHARS).collect()
}

/// What a preview decided, before it becomes a token or a sentence.
enum ReadView {
    Blocked(String),
    Ready,
}

/// A blocked preview still needs a summary for the token; nothing shows it.
fn blocked(reason: &str) -> (ReadView, ConfirmationSummary) {
    (
        ReadView::Blocked(reason.to_string()),
        ConfirmationSummary {
            operation: "Import".to_string(),
            target_ids: Vec::new(),
            counts: Vec::new(),
            capability: None,
            consequence: String::new(),
            target_name: None,
            signing_key_id: None,
            signing_key_claim: confirm::KeyClaim::None,
            signing_key_is_published_example: false,
            branch_notice: None,
            freezes: None,
            history_notice: None,
            rename_note: None,
            declaration_notices: Vec::new(),
            notices: Vec::new(),
            message_note: None,
            materializes: None,
            merge_plan: None,
            asides: Vec::new(),
        },
    )
}

// -------------------------------------------------------------------------------------------
// Export (RFC 050 decision 5; Handoff B)
// -------------------------------------------------------------------------------------------

/// stikk's own short name for the operation, used in stale and declined messages.
pub const EXPORT_OPERATION: &str = "export";

/// **Why stikk will not export a tag** (decision 5). Not measured, and a bundle of a tag is not
/// a thing this RFC claims to know the shape of.
pub const TAGS_ARE_NOT_EXPORTED: &str = "stikk exports a branch. A tag has not been measured here, so stikk does not offer it — \
     `prikk bundle export --ref <tag>` does it without stikk.";

/// **Why stikk will not export a received ref** (decision 5). Received history is somebody
/// else's bundle already; re-exporting it is not what the key is for.
pub const RECEIVED_IS_NOT_EXPORTED: &str = "This is received history, not a branch of yours. stikk exports your own branches; \
     `prikk bundle export --ref <ref>` exports anything prikk will.";

/// **Why stikk will not export a closed branch** (decision 5).
pub const CLOSED_IS_NOT_EXPORTED: &str =
    "This branch is closed. stikk exports an open branch; reopening is prikk's to do.";

/// **Why stikk will not export a branch with no published history** (decision 5). There is no
/// ref for prikk to walk a closure from.
pub const UNPUBLISHED_IS_NOT_EXPORTED: &str = "This branch has nothing published yet, so there is no history to put in a bundle. Seal \
     first.";

/// **stikk's own reason, below the band**, for the export key.
#[must_use]
pub fn export_unavailable(minor: u32) -> String {
    format!(
        "Exporting a bundle needs prikk ≥ 0.{BUNDLE_FROM_MINOR} — this prikk is 0.{minor}. The \
         rest of stikk works."
    )
}

/// **Why stikk will not write a bundle inside the repository** (decision 5, `C-E2`).
///
/// prikk writes one there happily, and it then sits in the worktree as an untracked file: the
/// next `commit --from-worktree` captures it, and a bundle of a branch's history inside that
/// branch's history is not something anyone asked for.
pub const INSIDE_THE_REPOSITORY: &str = "That path is inside this repository. The bundle would sit in your worktree as an untracked \
     file, and the next commit would capture it. Choose a path outside the repository.";

/// **Why stikk will not overwrite** (decision 5). prikk refuses this too, without `--force`;
/// stikk says it first, and names what to do, because the reader is about to be told `no` by a
/// program they did not run.
pub const DESTINATION_EXISTS: &str =
    "A file is already there. stikk does not overwrite it — choose another name.";

/// The sentence under the export card, in stikk's own words — **with no name in it**.
const EXPORT_CONSEQUENCE: &str = "One ref's history is written to that file: the blocks, the patches and the recorded \
     author-key material. Nothing in the repository changes, and no key is used. Whoever you \
     send it to can read it with `prikk bundle verify` before importing anything.";

/// What an export can say before it runs.
#[derive(Debug)]
pub enum ExportPreview {
    /// stikk will not run it, and says why in its own voice.
    Blocked(String),
    /// Ready to confirm.
    Ready {
        /// The confirmation token.
        token: Box<PreviewToken>,
    },
}

/// The ref and the destination an export is run with, kept together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportTarget {
    /// The branch whose closure is written.
    pub reff: String,
    /// The file to write — resolved, and what the card showed.
    pub output: std::path::PathBuf,
}

/// The destination stikk suggests for `reff`: the repository's parent, the branch's last
/// segment, and the tip's short id (decision 5).
///
/// **Outside the repository by construction**, because that is where the one rule stikk adds
/// says it must go — a reader who accepts the suggestion never meets [`INSIDE_THE_REPOSITORY`].
#[must_use]
pub fn suggested_destination(repo: &Path, reff: &str, tip: &str) -> std::path::PathBuf {
    let leaf = reff.rsplit('/').next().unwrap_or(reff);
    let parent = repo.parent().unwrap_or(repo);
    parent.join(format!("{leaf}-{}.bundle", short(tip)))
}

/// Preview exporting `reff` to `output`.
///
/// **Two rules, and both are stikk's** (decision 5). prikk would write inside the repository and
/// would refuse an existing file in its own words; stikk refuses the first outright and the
/// second first, naming what to do. **A missing directory is prikk's refusal**, shown verbatim:
/// stikk does not pre-check it, because prikk's message already says which directory and stikk
/// adding a check would be a second opinion about the same filesystem.
///
/// # Errors
/// Propagates every [`stikk_model::StikkError`] the ref and history reads raise.
pub fn export_preview(
    prikk: &impl Prikk,
    repo: &Path,
    reff: &str,
    output: &Path,
) -> Result<ExportPreview> {
    let reff = reff.to_string();
    let output = output.to_path_buf();
    let intent = Intent {
        category: RequestCategory::Exchange,
        operation: EXPORT_OPERATION,
    };
    let (view, token): (ReadView, _) = confirm::preview(prikk, repo, intent, || {
        // **The rows the picker declined are re-asked here**, as a merge re-asks its guards: a
        // branch can be closed between the key and the confirmation.
        if let Some(reason) = not_exportable(prikk, repo, &reff)? {
            return Ok(export_blocked(&reason));
        }
        // **`C-E2`'s own check, resolved** -- the same control that keeps stikk's state files
        // out of every repository, applied to the one file stikk asks prikk to write.
        if stikk_state::paths::ensure_outside_repository(&output, Some(repo)).is_err() {
            return Ok(export_blocked(INSIDE_THE_REPOSITORY));
        }
        // **Existence, and nothing more.** Whether the *directory* is there is prikk's to say.
        if output.exists() {
            return Ok(export_blocked(DESTINATION_EXISTS));
        }
        let tip = prikk
            .history(repo, &reff, 1)?
            .blocks
            .first()
            .map(|block| short(&block.block_id));
        let mut target_ids = vec![confirm::TargetId::named("branch", reff.clone())];
        target_ids.extend(tip.map(|tip| confirm::TargetId::named("tip", tip)));
        target_ids.push(confirm::TargetId::named(
            "file",
            output.display().to_string(),
        ));
        Ok((
            ReadView::Ready,
            ConfirmationSummary {
                operation: "Export".to_string(),
                target_ids,
                counts: Vec::new(),
                capability: None,
                consequence: EXPORT_CONSEQUENCE.to_string(),
                target_name: None,
                signing_key_id: None,
                signing_key_claim: confirm::KeyClaim::None,
                signing_key_is_published_example: false,
                branch_notice: None,
                freezes: None,
                history_notice: None,
                rename_note: None,
                declaration_notices: Vec::new(),
                notices: Vec::new(),
                message_note: None,
                materializes: None,
                merge_plan: None,
                asides: Vec::new(),
            },
        ))
    })?;
    Ok(match view {
        ReadView::Blocked(reason) => ExportPreview::Blocked(reason),
        ReadView::Ready => ExportPreview::Ready {
            token: Box::new(token),
        },
    })
}

/// The four rows stikk will not export, from freshly read refs.
fn not_exportable(prikk: &impl Prikk, repo: &Path, reff: &str) -> Result<Option<String>> {
    if reff.starts_with("tags/") {
        return Ok(Some(TAGS_ARE_NOT_EXPORTED.to_string()));
    }
    if reff.starts_with(RECEIVED_PREFIX) {
        return Ok(Some(RECEIVED_IS_NOT_EXPORTED.to_string()));
    }
    let refs = prikk.refs(repo)?;
    let Some(entry) = refs.refs.iter().find(|entry| entry.name == reff) else {
        // Not in the list at all is the unpublished row: the picker offers it, and prikk has
        // nothing to walk a closure from.
        return Ok(Some(UNPUBLISHED_IS_NOT_EXPORTED.to_string()));
    };
    if entry.closed {
        return Ok(Some(CLOSED_IS_NOT_EXPORTED.to_string()));
    }
    if entry.received {
        return Ok(Some(RECEIVED_IS_NOT_EXPORTED.to_string()));
    }
    Ok(None)
}

/// Export the ref this token was previewed for.
///
/// # Errors
/// As [`export_preview`], plus every refusal prikk raises -- a missing directory among them.
pub fn export_confirm_and_execute(
    prikk: &impl Prikk,
    repo: &Path,
    token: PreviewToken,
    readiness: stikk_model::Readiness,
    evidence: Evidence,
    target: &ExportTarget,
) -> Result<Outcome<BundleExport>> {
    let confirmed = confirm::confirm(prikk, repo, token, readiness, evidence)?;
    confirm::execute(prikk, repo, confirmed, || {
        prikk.bundle_export(repo, &target.reff, &target.output)
    })
}

/// A blocked export still needs a summary for the token; nothing shows it.
fn export_blocked(reason: &str) -> (ReadView, ConfirmationSummary) {
    let (_, mut summary) = blocked(reason);
    summary.operation = "Export".to_string();
    (ReadView::Blocked(reason.to_string()), summary)
}

#[cfg(test)]
mod tests;
