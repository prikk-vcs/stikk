//! Deciding what a remembered session may actually restore (design `LC-3`/`LC-6`; RFC 046
//! decision 5).
//!
//! **Re-validation first, display second.** A record holds names, and a name is a claim about a
//! repository that may have moved on since it was written: the branch may be gone, closed, or
//! renamed. [`restore`] is the one place that decides, and it is a **pure function** of what was
//! stored and what prikk has just reported — so the whole rule is testable without a terminal,
//! and the real-binary suite can drive it against prikk's own lists.
//!
//! **Nothing stored is ever a fact before prikk confirms it** (`T-T4`). Where the remembered ref
//! is gone the fallback is today's rule, unchanged, and [`Restored::note`] says so in one line
//! rather than leaving a reader to notice that they are somewhere else.

use stikk_model::CurrentBranch;
use stikk_prikk::RefEntry;
use stikk_state::{LastView, SessionRecord};

/// What stikk may open, after re-validating what it remembered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restored {
    /// The ref to focus, or `None` for "no ref focused" — today's answer when prikk names no
    /// current branch and `heads/main` is unpublished.
    pub focus: Option<String>,
    /// The top-level view to open.
    pub view: LastView,
    /// Whether the Changes view hides untracked entries.
    pub hide_untracked: bool,
    /// One line for the reader when the remembered ref is gone, in stikk's own voice.
    pub note: Option<String>,
}

/// **stikk's own words when the ref a session remembered is no longer there** (decision 5).
///
/// It names no ref: a banner is one line of stikk's own voice, and the ref it would name is
/// repository text that is, by this point, not in any list. What the reader needs is why they are
/// somewhere else than they left.
pub const REMEMBERED_REF_IS_GONE: &str = "the ref this repository was left on is no longer listed, so stikk opened on prikk's current \
     branch instead.";

/// The default ref stikk falls back to when prikk names no current branch but `heads/main` is
/// published — today's rule, stated once.
const DEFAULT_REF: &str = "heads/main";

/// Decide what to open.
///
/// **The stored focus is used only if `refs` lists it and it is not closed.** Anything else — no
/// record, a ref prikk no longer lists, a closed one — falls back to the rule that was there
/// before sessions existed: prikk's current branch at ≥ 0.42, else published `heads/main`, else
/// nothing focused.
///
/// **A valid stored focus wins over prikk's current branch**, which is what *resume* means. The
/// status bar already shows the difference, in the words RFC 029 B gave it.
#[must_use]
pub fn restore(
    stored: Option<&SessionRecord>,
    refs: &[RefEntry],
    current: &CurrentBranch,
    main_is_published: bool,
) -> Restored {
    let fallback = || match current {
        CurrentBranch::Branch(branch) => Some(branch.as_str().to_string()),
        CurrentBranch::NotReported | CurrentBranch::Unresolved(_) if main_is_published => {
            Some(DEFAULT_REF.to_string())
        }
        CurrentBranch::NotReported | CurrentBranch::Unresolved(_) => None,
    };
    let Some(stored) = stored else {
        return Restored {
            focus: fallback(),
            view: LastView::Orientation,
            hide_untracked: false,
            note: None,
        };
    };
    // A ref prikk lists and has not closed. A tag qualifies: focusing one is a read, and the
    // picker offers it.
    let remembered = stored.focus.as_ref().filter(|name| {
        refs.iter()
            .any(|entry| &&entry.name == name && !entry.closed)
    });
    let gone = stored.focus.is_some() && remembered.is_none();
    Restored {
        focus: remembered.cloned().or_else(fallback),
        // **The view is restored even when the ref is not.** It is a preference, not a claim
        // about the repository, and every one of the four re-reads from prikk when it opens.
        view: stored.view,
        hide_untracked: stored.hide_untracked,
        note: gone.then(|| REMEMBERED_REF_IS_GONE.to_string()),
    }
}

#[cfg(test)]
mod tests;
