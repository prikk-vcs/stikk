//! Which maintainer keys this repository trusts, and whether a bundle's sealer is one of them
//! (RFC 054).
//!
//! # What trust is here, and what it is not
//!
//! prikk accepts an adopted key's signatures on the objects that key signed. **Adopting a key
//! never lets it move one of your branches.** prikk says so itself — but only in the prose form
//! of `trust maintainer list`, not in the JSON stikk reads (RFC 054 F1), so the sentence a view
//! shows beside the list is stikk's own rather than a second spawn to fetch prikk's.
//!
//! # Read-only
//!
//! Nothing here adds, removes or confirms anything. Changing trust is Tier Three Typed with a
//! changed key under a known id as a security event, and it is its own increment. **An untrusted
//! sealer does not block an import either**: received refs exist precisely so that history you
//! have not vouched for can sit beside your own.
//!
//! # No key material
//!
//! Ids and public keys, and nothing else. Not a seed, not a seed path, and not `key status`'s
//! own `path` field (`NFR-S03`, `C-I1`).

use std::path::Path;

use stikk_model::{RequestCategory, Result};
use stikk_prikk::{Prikk, TrustedKey, TrustedKeys};

use crate::confirm::{self, Evidence, Intent, Outcome, PreviewToken};

/// The first prikk minor that lists them, re-exported so a view says the number the seam gates
/// on — the one-constant rule every other band follows (RFC 054 decision 4).
pub use stikk_prikk::FILES_FROM_MINOR as TRUST_FROM_MINOR;

/// **What adopting a key means**, in stikk's words because prikk's JSON does not carry it
/// (RFC 054 decision 2, F1).
pub const WHAT_TRUST_MEANS: &str = "prikk accepts an adopted key's signatures on what that key signed. Adopting a key never \
     lets it move one of your branches.";

/// Shown where the list is empty.
pub const NONE_ADOPTED: &str = "This repository trusts no maintainer key.";

/// prikk's own command for adopting one, as **text**. stikk does not run it.
pub const HOW_TO_ADOPT: &str = "prikk trust maintainer add --key-id ID --public-key HEX";

/// The caution beside [`HOW_TO_ADOPT`] — the one thing a newcomer gets wrong.
pub const WHERE_TO_GET_THE_KEY: &str =
    "Get the public key from its owner, not from a bundle they sent.";

/// Said once, above the list, when `key status` could not be read.
pub const OWN_KEY_UNKNOWN: &str = "stikk could not read which key is yours, so no row below is marked — the list itself is \
     prikk's and is unaffected.";

/// **stikk's own reason, below the band.**
#[must_use]
pub fn trust_unavailable(minor: u32) -> String {
    format!(
        "Listing the keys this repository trusts needs prikk ≥ 0.{TRUST_FROM_MINOR} — this prikk \
         is 0.{minor}. The rest of stikk works."
    )
}

/// Whether a preview armed, or stikk's own reason not to -- the shape `files.rs` and
/// `bundle.rs` both use.
#[derive(Debug)]
enum ReadView {
    Blocked(String),
    Ready,
}

/// A blocked preview still needs a summary for the token; nothing shows it.
fn blocked(reason: &str, operation: &'static str) -> (ReadView, crate::ConfirmationSummary) {
    (
        ReadView::Blocked(reason.to_string()),
        summary(operation, ""),
    )
}

/// An empty card, filled in by each preview.
fn summary(operation: &'static str, consequence: &str) -> crate::ConfirmationSummary {
    crate::ConfirmationSummary {
        operation: operation.to_string(),
        target_ids: Vec::new(),
        counts: Vec::new(),
        // **Neither write consumes a key** (F1, measured at both versions), so the card claims
        // none: `Trust` is Tier Three Typed, whose ceremony is the typed id.
        capability: None,
        consequence: consequence.to_string(),
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
    }
}

/// One adopted key, as a view shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustRow {
    /// prikk's own id for the key.
    pub key_id: String,
    /// The public key, as prikk reports it.
    pub public_key: String,
    /// Whether this is the key `key status` says would sign as MAINTAINER here.
    ///
    /// **Both halves must match** (RFC 054 decision 2, §7 trap 3): an id is a label anyone can
    /// choose, so a row carrying your id under someone else's public key is not yours, and
    /// marking it so would be the one mistake this view exists to prevent.
    pub yours: bool,
}

/// What the Trust view shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustView {
    /// prikk answered. The rows are **in prikk's order**, and the list may be empty.
    Listed {
        /// One row per adopted key.
        rows: Vec<TrustRow>,
        /// Set when `key status` could not be read, so no row is marked: [`OWN_KEY_UNKNOWN`].
        own_key_unknown: Option<String>,
    },
    /// Below [`TRUST_FROM_MINOR`]: stikk's own sentence, and nothing was spawned.
    Unavailable(String),
    /// prikk refused the list. **Its words, verbatim** (`ER-02`), and nothing else is shown.
    Failed(String),
}

/// Read the adopted keys, and mark your own.
///
/// **A failed `key status` costs the marking and nothing else** (RFC 035 A3's rule): the list is
/// prikk's and does not depend on it, so it is still shown, with one line saying why no row is
/// marked. A failed *list*, on the other hand, leaves nothing to show.
///
/// # Errors
/// Propagates a [`stikk_model::StikkError`] only from the handshake below the band; a refused
/// list becomes [`TrustView::Failed`], because a repository stikk cannot read the trust of is
/// not a broken session.
pub fn read(prikk: &impl Prikk, repo: &Path) -> Result<TrustView> {
    let keys = match prikk.trusted_keys(repo) {
        Ok(TrustedKeys::Listed(keys)) => keys,
        Ok(TrustedKeys::Unavailable) => {
            let minor = prikk.handshake()?.version.minor;
            return Ok(TrustView::Unavailable(trust_unavailable(minor)));
        }
        Err(error) => return Ok(TrustView::Failed(error.to_string())),
    };
    // **The marking is a second read, and a failing one is not fatal.** `key status` is how
    // stikk knows which key is yours (RFC 026); the list stands without it.
    let (mine, own_key_unknown) = match prikk.readiness(repo) {
        Ok(report) => (
            report.maintainer.key_id.zip(report.maintainer.public_key),
            None,
        ),
        Err(_) => (None, Some(OWN_KEY_UNKNOWN.to_string())),
    };
    let rows = keys
        .into_iter()
        .map(|key| TrustRow {
            yours: is_yours(&key, mine.as_ref()),
            key_id: key.key_id,
            public_key: key.public_key,
        })
        .collect();
    Ok(TrustView::Listed {
        rows,
        own_key_unknown,
    })
}

/// Whether `key` is the one `key status` reports for the MAINTAINER role — **id and public key
/// both** (§7 trap 3).
fn is_yours(key: &TrustedKey, mine: Option<&(String, String)>) -> bool {
    mine.is_some_and(|(id, public)| key.key_id == *id && key.public_key == *public)
}

/// **The sentence beneath the sealer rows**, wherever at least one of them names an adopted key
/// (decision 3 as amended, 2026-10-29).
///
/// **Measured, and the reason the rows say *names*.** A sender can adopt its own public key
/// under an id the receiver trusts and seal with it: `bundle preview` then reports that id with
/// exit 0 and `bundle import` exits 0, and only `verify` finds it —
/// `publication-trust`/`PRIKK-TRUST-PUBLICATION-INVALID-SIGNATURE`, *"carries a MAINTAINER
/// signature by adopted key … that does not verify"*
/// (`.git-exclude/reports/054-trust-read/spoof054-0.55.txt`, prikk 0.55.0).
pub const NAMES_BUT_DOES_NOT_PROVE: &str = "A bundle names its sealer but does not prove it. Import it, then Verify (V) checks each \
     signature against the key trusted here.";

/// What a bundle card says beside one sealer (RFC 054 decision 3, as amended).
///
/// **Every name here says *names*, and that is the point.** `sealed_by` is the `key_id` string
/// out of the signature envelope, checked against nothing at preview time; a variant called
/// `Trusted` would be read as a verdict by the next person who matched on it, and it is not one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealerTrust {
    /// The id names a key this repository has adopted, and it is not the reader's own.
    /// **Not a verdict on the signature** — see [`NAMES_BUT_DOES_NOT_PROVE`].
    NamesATrustedKey,
    /// The id names the reader's own MAINTAINER key. Also not a verdict: anyone can seal under
    /// a name.
    NamesYourKey,
    /// This repository has adopted no key of that name. **Still exact**, and not a refusal:
    /// prikk accepts no seal by an id it has not adopted, and the import runs either way.
    NotTrusted,
    /// The list could not be read. Said plainly rather than guessed either way.
    Unknown,
}

impl SealerTrust {
    /// stikk's own words for this answer, shown beside the id.
    #[must_use]
    pub const fn sentence(self) -> &'static str {
        match self {
            Self::NamesATrustedKey => "names a key this repository trusts",
            Self::NamesYourKey => "names your key",
            Self::NotTrusted => "not trusted by this repository",
            Self::Unknown => "whether this repository trusts it could not be read",
        }
    }

    /// Whether this answer is one that [`NAMES_BUT_DOES_NOT_PROVE`] must accompany.
    #[must_use]
    pub const fn names_an_adopted_key(self) -> bool {
        matches!(self, Self::NamesATrustedKey | Self::NamesYourKey)
    }
}

/// One sealer id with what this repository says about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sealer {
    /// prikk's recorded key id, verbatim.
    pub key_id: String,
    /// What this repository says about it.
    pub trust: SealerTrust,
}

/// What a card can say about its sealers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealerTrustRead {
    /// Judged, one per id and **in prikk's order**.
    Judged(Vec<Sealer>),
    /// **Below the band: stikk says nothing extra** (RFC 054 decision 4). The card shows
    /// prikk's ids and prikk's note, exactly as 0.16.0 did — *"could not be read"* would be
    /// stikk reporting a limit of its own as if it had tried and failed.
    Unavailable,
}

impl SealerTrustRead {
    /// The judged rows, or an empty slice below the band.
    #[must_use]
    pub fn judged(&self) -> &[Sealer] {
        match self {
            Self::Judged(sealers) => sealers,
            Self::Unavailable => &[],
        }
    }

    /// Whether [`NAMES_BUT_DOES_NOT_PROVE`] belongs beneath these rows: at least one of them
    /// names a key this repository has adopted.
    ///
    /// **Not shown where every row is `NotTrusted`**: nothing there invites the reader to take
    /// a name for a signature, and a caution about a claim nobody made is noise.
    #[must_use]
    pub fn names_an_adopted_key(&self) -> bool {
        self.judged()
            .iter()
            .any(|sealer| sealer.trust.names_an_adopted_key())
    }
}

/// Judge each of `sealed_by` against this repository's adopted keys — **one read for all of
/// them** (RFC 054 decision 3, as amended).
///
/// Three outcomes, and they are not the same thing:
/// - the list read → each id judged;
/// - the list **refused** → [`SealerTrust::Unknown`] for every id, so the card says what it does
///   not know and nothing else about it changes (RFC 035 A3's rule);
/// - **below the band** → [`SealerTrustRead::Unavailable`], and the card says nothing extra at
///   all (decision 4).
///
/// **A bundle records a name, so a name is what is matched.** `bundle-preview-v1`'s `sealed_by`
/// is the `key_id` string out of the signature envelope — ids, no public keys, and nothing
/// checked against anything at preview time. The id is therefore compared with the adopted
/// keys' ids and the sentence says what it found: this id **names** a key this repository
/// trusts. It does not say the signature is that key's, because preview cannot know that, and a
/// sender who adopts its own key under a name the receiver trusts gets a clean preview and a
/// clean import (measured — see [`NAMES_BUT_DOES_NOT_PROVE`]). **Verify after import is where
/// prikk checks it.**
#[must_use]
pub fn sealers(prikk: &impl Prikk, repo: &Path, sealed_by: &[String]) -> SealerTrustRead {
    let unknown = || {
        SealerTrustRead::Judged(
            sealed_by
                .iter()
                .map(|key_id| Sealer {
                    key_id: key_id.clone(),
                    trust: SealerTrust::Unknown,
                })
                .collect(),
        )
    };
    let rows = match read(prikk, repo) {
        Ok(TrustView::Listed { rows, .. }) => rows,
        Ok(TrustView::Unavailable(_)) => return SealerTrustRead::Unavailable,
        // Refused, or the handshake itself failed: stikk tried and does not know.
        Ok(TrustView::Failed(_)) | Err(_) => return unknown(),
    };
    SealerTrustRead::Judged(
        sealed_by
            .iter()
            .map(|key_id| Sealer {
                key_id: key_id.clone(),
                trust: match rows.iter().find(|row| row.key_id == *key_id) {
                    Some(row) if row.yours => SealerTrust::NamesYourKey,
                    Some(_) => SealerTrust::NamesATrustedKey,
                    None => SealerTrust::NotTrusted,
                },
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests;

// ============================================================ RFC 056: adopt and remove

/// **What adopting does**, as the adopt card's consequence (RFC 056 decision 2).
///
/// **No name in it** (RFC 044 C1): the key id and the public key are drawn as the card's own
/// rows, so this sentence stays stikk's chrome and the repository's text stays apart.
pub const ADOPT_CONSEQUENCE: &str =
    "Work this key sealed will verify here. It cannot move one of your branches.";

/// **What removing does**, as the remove card's consequence (decision 3).
pub const REMOVE_CONSEQUENCE: &str =
    "prikk will no longer accept this key's signatures in this repository.";

/// The guard for an id already adopted **with this very key** (decision 2). Not a refusal of
/// prikk's: stikk answers it from the list it just read, and spawns no `add`.
pub const ALREADY_TRUSTED: &str = "This repository already trusts this key.";

/// The guard for removing an id that is not adopted (decision 3). prikk exits 0 for this and
/// says *"maintainer key was not trusted"* (F1); stikk says it without the spawn.
pub const NO_LONGER_TRUSTED: &str = "This repository no longer trusts this key.";

/// The guard for removing the only adopted key — prikk refuses it, *"cannot remove the last
/// trusted maintainer key; a repository policy must not be explicitly empty"* (F4), and stikk
/// says what to do instead before asking.
pub const LAST_KEY: &str =
    "prikk keeps at least one trusted key; adopt another before removing this one.";

/// **Always on the remove card** (decision 3), above the consequence.
///
/// **It says *if*, because stikk cannot count.** `log-report-v1` does not say which key sealed
/// each block (F4), so the blocks a removal would un-verify are not knowable from JSON. The
/// measured worst case is the whole sealed history: removing the key that sealed it makes every
/// block, its ref-state and its ref-update read *"has no trusted MAINTAINER signature"* and the
/// verdict fail `publication-trust` — and adopting the same key under the same id again repairs
/// it.
pub const REMOVE_MAY_FAIL_VERIFY: &str = "If this key sealed blocks in this repository, Verify will fail them until the same key is \
     adopted again under the same id.";

/// Added to the remove card where the row is the reader's own key (decision 3).
pub const REMOVE_YOUR_OWN: &str =
    "This is your own key. After removing it, you cannot seal here until it is adopted again.";

/// Added after a removal: stikk points at Verify rather than running it (decision 7).
pub const REMOVED_SEE_VERIFY: &str = "Press `V` to see what Verify says now.";

/// **The security event** (decision 4), shown where an id is adopted under a *different* key.
///
/// **No id in the sentence** (`C-T2a`, RFC 044 C1): the id and both public keys are the card's
/// own rows, each repository text, so stikk's words stay stikk's.
pub const TWO_KEYS_ONE_ID: &str = "This repository already trusts a different key under that id. Two keys cannot share an id: \
     an id travels in every signature. If its owner really has a new key, they must sign under \
     a new id, and you adopt it under that one. If they did not tell you they changed keys, \
     treat this key as someone else's.";

/// stikk's heading over prikk's own words when an adopt is refused (decision 5).
pub const PRIKK_REFUSED_ADOPT: &str = "prikk refused to adopt this key";

/// The checks stikk makes on what was typed, before prikk is asked (decision 2).
pub const PUBLIC_KEY_SHAPE: &str =
    "A public key is exactly 64 characters, 0-9 and a-f. Ask its owner for the key prikk printed.";

/// Said where the id is empty or carries whitespace or a control character.
pub const KEY_ID_SHAPE: &str =
    "A key id cannot be empty and cannot contain spaces or control characters.";

/// **The id prikk itself would give this key** — `ed25519-` and the first 16 characters of the
/// public key (prikk 0.45.0's rule, the one `setup` and `key generate --out` follow).
///
/// Used twice: to prefill the id prompt, and to decide whether the adopt card carries
/// [`not_the_default_id`]'s notice. **prikk does not tie an id to its key** (RFC 056 F3) — it
/// accepted `ed25519-2222222222222222` for a key whose default id is quite different — so a
/// mismatch is legitimate when the owner signs under a chosen id, and is also what a mix-up
/// looks like. stikk says which it is looking at and lets the reader decide.
///
/// A key shorter than 16 characters cannot be prikk's, and gets whatever it has: this is a
/// suggestion, and [`check_public_key`] is what refuses a malformed one.
#[must_use]
pub fn default_key_id(public_key: &str) -> String {
    let head: String = public_key.chars().take(16).collect();
    format!("ed25519-{head}")
}

/// The notice for an id that is not [`default_key_id`] (decision 2).
#[must_use]
pub fn not_the_default_id(public_key: &str) -> String {
    format!(
        "This id is not this key's default ({}). That is allowed, but it must be the id its \
         owner signs with: ask them.",
        default_key_id(public_key)
    )
}

/// What stikk checks about a typed public key, before prikk is asked (decision 2).
///
/// **Trimmed first**, because a pasted key carries whatever the terminal added around it. The
/// shape is prikk's own: *"maintainer public key must be 64 lowercase hex characters"* (F1).
/// Returning stikk's sentence rather than an error is what lets the prompt stay open.
pub fn check_public_key(typed: &str) -> std::result::Result<String, String> {
    let trimmed = typed.trim();
    if trimmed.chars().count() == 64
        && trimmed
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        Ok(trimmed.to_string())
    } else {
        Err(PUBLIC_KEY_SHAPE.to_string())
    }
}

/// What stikk checks about a typed key id (decision 2).
///
/// **Not emptiness alone.** An id travels in every signature and is drawn in every card, so a
/// control character or a space in one would be a value stikk cannot show faithfully and prikk
/// would carry anyway.
pub fn check_key_id(typed: &str) -> std::result::Result<String, String> {
    let trimmed = typed.trim();
    if trimmed.is_empty() || trimmed.chars().any(|c| c.is_whitespace() || c.is_control()) {
        Err(KEY_ID_SHAPE.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

/// The two keys that cannot share an id (decision 4), for the card that says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwoKeysOneId {
    /// The id both claim. Repository text.
    pub key_id: String,
    /// The public key this repository has already adopted under it.
    pub adopted: String,
    /// The public key that was offered for it.
    pub given: String,
}

/// Whether adopting is offered, and the token that arms it (RFC 056 decision 2).
#[derive(Debug)]
pub enum AdoptPreview {
    /// stikk will not: its own sentence. **Nothing was spawned.**
    Blocked(String),
    /// The id is adopted under a different key. **Not a confirmation** (decision 4): the card
    /// has no confirm key, and no `add` was spawned.
    TwoKeys(Box<TwoKeysOneId>),
    /// Armed. Confirm by typing the key id.
    Ready {
        /// Single-use, step-scoped.
        token: Box<PreviewToken>,
    },
}

/// Whether removing is offered, and the token that arms it (decision 3).
#[derive(Debug)]
pub enum RemovePreview {
    /// stikk will not: its own sentence. **Nothing was spawned.**
    Blocked(String),
    /// Armed. Confirm by typing the key id.
    Ready {
        /// Single-use, step-scoped.
        token: Box<PreviewToken>,
    },
}

/// stikk's own name for each operation, on the card and in every error.
pub const ADOPT_OPERATION: &str = "Adopt key";
/// As [`ADOPT_OPERATION`].
pub const REMOVE_OPERATION: &str = "Remove key";

/// Preview adopting `public_key` under `key_id` (RFC 056 decision 2).
///
/// **Both guards live here**, inside `confirm::preview`, so `confirm` takes them again at the
/// moment the user answers — the shape `import_preview` uses. Each declines with **no `add`
/// spawned**: a guard that declined after the write would be no guard.
///
/// The inputs are checked before prikk is asked at all, and a failure is stikk's sentence, not
/// an error: the frontend keeps its prompt open and says it.
///
/// # Errors
/// Propagates a [`stikk_model::StikkError`] from the reads this preview makes.
pub fn adopt_preview(
    prikk: &impl Prikk,
    repo: &Path,
    readiness: stikk_model::Readiness,
    key_id: &str,
    public_key: &str,
) -> Result<AdoptPreview> {
    let (key_id, public_key) = match (check_key_id(key_id), check_public_key(public_key)) {
        (Ok(id), Ok(key)) => (id, key),
        (Err(said), _) | (_, Err(said)) => return Ok(AdoptPreview::Blocked(said)),
    };
    // **Read-only is declined before anything is read**, in the gate's own words. The gate runs
    // again inside `confirm`, which is the rule this preview follows throughout.
    if let Err(error) = confirm::capability_gate(ADOPT_OPERATION, RequestCategory::Trust, readiness)
    {
        return Ok(AdoptPreview::Blocked(error.to_string()));
    }
    let mut conflict = None;
    let (view, token): (ReadView, _) = confirm::preview(
        prikk,
        repo,
        Intent {
            category: RequestCategory::Trust,
            operation: ADOPT_OPERATION,
        },
        || {
            let keys = match prikk.trusted_keys(repo)? {
                TrustedKeys::Listed(keys) => keys,
                TrustedKeys::Unavailable => {
                    let minor = prikk.handshake()?.version.minor;
                    return Ok(blocked(&trust_unavailable(minor), ADOPT_OPERATION));
                }
            };
            if let Some(adopted) = keys.iter().find(|key| key.key_id == key_id) {
                if adopted.public_key == public_key {
                    return Ok(blocked(ALREADY_TRUSTED, ADOPT_OPERATION));
                }
                // **The security event** (decision 4). It is not a confirmation, so the card is
                // carried out of here rather than armed.
                conflict = Some(TwoKeysOneId {
                    key_id: key_id.clone(),
                    adopted: adopted.public_key.clone(),
                    given: public_key.clone(),
                });
                return Ok(blocked(TWO_KEYS_ONE_ID, ADOPT_OPERATION));
            }
            let mut summary = summary(ADOPT_OPERATION, ADOPT_CONSEQUENCE);
            summary.target_ids = vec![
                confirm::TargetId::named("key id", key_id.clone()),
                confirm::TargetId::named("public key", public_key.clone()),
            ];
            // **The typed evidence is the key id** (§7 trap 1), matched exactly by `confirm`.
            summary.target_name = Some(key_id.clone());
            summary.notices = Vec::new();
            if key_id != default_key_id(&public_key) {
                summary.notices.push(not_the_default_id(&public_key));
            }
            summary.asides = vec![confirm::Aside {
                before: "",
                name: String::new(),
                after: "",
                body: vec![
                    WHAT_TRUST_MEANS.to_string(),
                    WHERE_TO_GET_THE_KEY.to_string(),
                ],
            }];
            Ok((ReadView::Ready, summary))
        },
    )?;
    if let Some(conflict) = conflict {
        return Ok(AdoptPreview::TwoKeys(Box::new(conflict)));
    }
    Ok(match view {
        ReadView::Blocked(reason) => AdoptPreview::Blocked(reason),
        ReadView::Ready => AdoptPreview::Ready {
            token: Box::new(token),
        },
    })
}

/// Preview removing `key_id` (RFC 056 decision 3).
///
/// **Two guards, both inside the preview**: an id that is no longer adopted, and a list of one,
/// which prikk refuses (F4) and stikk declines without asking. Neither spawns `remove`.
///
/// # Errors
/// Propagates a [`stikk_model::StikkError`] from the reads this preview makes.
pub fn remove_preview(
    prikk: &impl Prikk,
    repo: &Path,
    readiness: stikk_model::Readiness,
    key_id: &str,
) -> Result<RemovePreview> {
    let key_id = match check_key_id(key_id) {
        Ok(id) => id,
        Err(said) => return Ok(RemovePreview::Blocked(said)),
    };
    if let Err(error) =
        confirm::capability_gate(REMOVE_OPERATION, RequestCategory::Trust, readiness)
    {
        return Ok(RemovePreview::Blocked(error.to_string()));
    }
    let (view, token): (ReadView, _) = confirm::preview(
        prikk,
        repo,
        Intent {
            category: RequestCategory::Trust,
            operation: REMOVE_OPERATION,
        },
        || {
            let keys = match prikk.trusted_keys(repo)? {
                TrustedKeys::Listed(keys) => keys,
                TrustedKeys::Unavailable => {
                    let minor = prikk.handshake()?.version.minor;
                    return Ok(blocked(&trust_unavailable(minor), REMOVE_OPERATION));
                }
            };
            let Some(row) = keys.iter().find(|key| key.key_id == key_id) else {
                return Ok(blocked(NO_LONGER_TRUSTED, REMOVE_OPERATION));
            };
            if keys.len() == 1 {
                return Ok(blocked(LAST_KEY, REMOVE_OPERATION));
            }
            let mut summary = summary(REMOVE_OPERATION, REMOVE_CONSEQUENCE);
            summary.target_ids = vec![
                confirm::TargetId::named("key id", key_id.clone()),
                confirm::TargetId::named("public key", row.public_key.clone()),
            ];
            summary.target_name = Some(key_id.clone());
            // **Always**, because stikk cannot count the blocks this would un-verify (F4).
            summary.notices = vec![REMOVE_MAY_FAIL_VERIFY.to_string()];
            // **And a failed `key status` costs only this notice** (RFC 035 A3's rule): the
            // removal itself does not depend on knowing whose key this is.
            let mine = prikk
                .readiness(repo)
                .ok()
                .and_then(|report| report.maintainer.key_id.zip(report.maintainer.public_key));
            if is_yours(
                &TrustedKey {
                    key_id: key_id.clone(),
                    public_key: row.public_key.clone(),
                },
                mine.as_ref(),
            ) {
                summary.notices.push(REMOVE_YOUR_OWN.to_string());
            }
            Ok((ReadView::Ready, summary))
        },
    )?;
    Ok(match view {
        ReadView::Blocked(reason) => RemovePreview::Blocked(reason),
        ReadView::Ready => RemovePreview::Ready {
            token: Box::new(token),
        },
    })
}

/// What a finished trust write leaves on screen (decision 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustWriteResult {
    /// prikk's own printed lines, verbatim.
    pub printed: Vec<String>,
    /// The list as it is **now**, re-read through `trusted_keys` — the answer, where prikk's
    /// prose is its account of it.
    pub view: TrustView,
    /// After a removal, [`REMOVED_SEE_VERIFY`]. stikk points at Verify and does not run it
    /// (decision 7): Verify can be slow on a large repository.
    pub see_verify: Option<&'static str>,
}

/// Adopt the key this token was previewed for, then re-read the list (decision 1).
///
/// # Errors
/// As [`adopt_preview`], plus every refusal prikk raises — the one decision 5 shows under
/// [`PRIKK_REFUSED_ADOPT`], verbatim and matched on by nothing.
pub fn adopt_confirm_and_execute(
    prikk: &impl Prikk,
    repo: &Path,
    token: PreviewToken,
    readiness: stikk_model::Readiness,
    evidence: Evidence,
    key_id: &str,
    public_key: &str,
) -> Result<Outcome<TrustWriteResult>> {
    let confirmed = confirm::confirm(prikk, repo, token, readiness, evidence)?;
    let outcome = confirm::execute(prikk, repo, confirmed, || {
        prikk.trust_add(repo, key_id, public_key)
    })?;
    written(prikk, repo, outcome, None)
}

/// Remove the key this token was previewed for, then re-read the list (decision 3).
///
/// # Errors
/// As [`remove_preview`], plus every refusal prikk raises.
pub fn remove_confirm_and_execute(
    prikk: &impl Prikk,
    repo: &Path,
    token: PreviewToken,
    readiness: stikk_model::Readiness,
    evidence: Evidence,
    key_id: &str,
) -> Result<Outcome<TrustWriteResult>> {
    let confirmed = confirm::confirm(prikk, repo, token, readiness, evidence)?;
    let outcome = confirm::execute(prikk, repo, confirmed, || prikk.trust_remove(repo, key_id))?;
    written(prikk, repo, outcome, Some(REMOVED_SEE_VERIFY))
}

/// Shape what a write left behind: prikk's lines, the list re-read, and the pointer a removal
/// carries.
fn written(
    prikk: &impl Prikk,
    repo: &Path,
    outcome: Outcome<stikk_prikk::TrustWritten>,
    see_verify: Option<&'static str>,
) -> Result<Outcome<TrustWriteResult>> {
    let printed = match outcome.result {
        stikk_prikk::TrustWritten::Done(lines) => lines,
        // Unreachable in practice -- the preview is banded -- and said rather than guessed.
        stikk_prikk::TrustWritten::Unavailable => {
            vec![trust_unavailable(prikk.handshake()?.version.minor)]
        }
    };
    Ok(Outcome {
        operation: outcome.operation,
        result: TrustWriteResult {
            printed,
            view: read(prikk, repo)?,
            see_verify,
        },
    })
}
