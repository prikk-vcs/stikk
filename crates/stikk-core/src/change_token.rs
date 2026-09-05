//! The change-token operation-layer bridge (design `LC-4`; RFC 003).
//!
//! [`change_token`] is a thin pass-through to [`Prikk::change_token`] — the operation layer's own place
//! for this read, matching [`crate::history::list_refs`]'s shape, so `LC-10`'s cache-validity check
//! (not built this increment) has somewhere to hang once it exists.
//!
//! [`staleness_notice`] is the `FR-106`/`OP-04` comparison: given the token a view or cache was
//! computed under and the current one, it says whether the repository changed and, if so, produces the
//! passive notice through the existing [`Presentation::Banner`] — no new presentation variant for this
//! (RFC 003 handoff §4).
//!
//! **Detection only, never a lock** (`CT-05`/`NFR-R02`). Between reading a token and acting on it, the
//! repository can change again — prikk's own locking is the real guard against a mutation racing
//! another writer. Nothing here makes an action atomic, and nothing above this module may present the
//! result as if it does.

use std::path::Path;

use stikk_model::{ChangeToken, Result};
use stikk_prikk::Prikk;

use crate::present::Presentation;

/// The `OP-04` passive notice's exact wording (design `external-design.md` `OP-04`) — quoted from the
/// design set, not a paraphrase this module invented.
const STALENESS_MESSAGE: &str = "repository changed outside stikk — refreshed";

/// Read the repository's current change token (design `LC-4`; category `read-history`).
///
/// # Errors
/// Propagates any [`stikk_model::StikkError`] the seam raises.
pub fn change_token(prikk: &impl Prikk, repo: &Path) -> Result<ChangeToken> {
    prikk.change_token(repo)
}

/// Compare a previously-captured token against the current one and produce the `FR-106`/`OP-04`
/// passive notice if the repository changed since. `None` when they are equal — nothing to say.
///
/// Reuses [`Presentation::Banner`] rather than adding a new `Presentation` variant (RFC 003 handoff
/// §4): "repository changed outside stikk — refreshed" is exactly a banner, not a modal or a routed
/// view, and the frontend already knows how to show one.
#[must_use]
pub fn staleness_notice(previous: &ChangeToken, current: &ChangeToken) -> Option<Presentation> {
    if previous == current {
        return None;
    }
    Some(Presentation::Banner {
        message: STALENESS_MESSAGE.to_string(),
        jump: None,
    })
}

#[cfg(test)]
mod tests;
