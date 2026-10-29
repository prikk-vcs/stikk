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

use stikk_model::Result;
use stikk_prikk::{Prikk, TrustedKey, TrustedKeys};

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

/// What a bundle card says beside one sealer (RFC 054 decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealerTrust {
    /// This repository has adopted that key, and it is not yours.
    Trusted,
    /// It is your own MAINTAINER key.
    Yours,
    /// This repository has not adopted it. **Not a refusal**: the import still runs.
    NotTrusted,
    /// The list could not be read. Said plainly rather than guessed either way.
    Unknown,
}

impl SealerTrust {
    /// stikk's own words for this answer, shown beside the id.
    #[must_use]
    pub const fn sentence(self) -> &'static str {
        match self {
            Self::Trusted => "trusted by this repository",
            Self::Yours => "yours",
            Self::NotTrusted => "not trusted by this repository",
            Self::Unknown => "whether this repository trusts it could not be read",
        }
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
}

/// Judge each of `sealed_by` against this repository's adopted keys — **one read for all of
/// them** (RFC 054 decision 3).
///
/// Three outcomes, and they are not the same thing:
/// - the list read → each id judged;
/// - the list **refused** → [`SealerTrust::Unknown`] for every id, so the card says what it does
///   not know and nothing else about it changes (RFC 035 A3's rule);
/// - **below the band** → [`SealerTrustRead::Unavailable`], and the card says nothing extra at
///   all (decision 4).
///
/// **Matched by id**, because that is all a bundle records: `bundle-preview-v1`'s `sealed_by`
/// carries ids and no public keys (RFC 054 F4), so there is no second half to compare. *Yours*
/// is then the row the Trust view itself marked, where both halves did match.
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
                    Some(row) if row.yours => SealerTrust::Yours,
                    Some(_) => SealerTrust::Trusted,
                    None => SealerTrust::NotTrusted,
                },
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests;
