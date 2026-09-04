//! The glossary product asset (design DM-09, FR-111; RFC 007).
//!
//! Two kinds of content ship *with* stikk (a product asset, versioned with the release — never user
//! data, never read from a repository or a network):
//!
//! - **Terminology mapping** — Git → prikk, so Git-shaped expectations are redirected in copy and in
//!   Help (external-design §0). Seeded in full now.
//! - **Code entries** — witness kinds (merge, FR-080) and verify finding codes (FR-100), keyed by
//!   prikk's own code. These arrive with the operations that surface them; a representative sample is
//!   seeded now to build and test the lookup and its degradation.
//!
//! **The degradation is the point** (RR-5/NFR-I03): a code with no entry returns `None`, and the
//! caller shows prikk's message verbatim — the message is never hidden behind a missing gloss.

/// One glossary entry, keyed by a prikk code (a witness kind or a verify finding code).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlossaryEntry {
    /// prikk's own code for the witness/finding.
    pub code: &'static str,
    /// A short human title.
    pub title: &'static str,
    /// The plain-language explanation.
    pub explanation: &'static str,
    /// Related codes worth reading next.
    pub see_also: &'static [&'static str],
}

/// One Git → prikk terminology redirect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermMapping {
    /// The Git term a user might arrive with.
    pub git: &'static str,
    /// What prikk offers instead (or that it does not exist).
    pub prikk: &'static str,
    /// A one-line note on the difference.
    pub note: &'static str,
}

/// The Git → prikk terminology mapping (external-design §0), seeded in full.
static TERMS: &[TermMapping] = &[
    TermMapping {
        git: "HEAD",
        prikk: "(none — there is no HEAD)",
        note: "prikk has no current-branch pointer; stikk focuses a named ref as a client-side preference.",
    },
    TermMapping {
        git: "checkout / switch branch",
        prikk: "focused ref + checkout plan",
        note: "Switching what you look at is a client-side focus change; materializing files is a plan-first checkout.",
    },
    TermMapping {
        git: "staging area / index",
        prikk: "(none)",
        note: "There is no staging step; a commit captures the whole worktree against the baseline.",
    },
    TermMapping {
        git: "stash",
        prikk: "(none)",
        note: "prikk has no stash; keep work in a separate ref or worktree instead.",
    },
    TermMapping {
        git: "commit --amend",
        prikk: "append a patch",
        note: "History is append-only at block granularity; you add a patch rather than rewriting one.",
    },
    TermMapping {
        git: "revert",
        prikk: "rollback flow",
        note: "Undoing is a rollback that records its own patches — history is never rewritten.",
    },
    TermMapping {
        git: "rebase / force-push",
        prikk: "(none — history is not rewritten)",
        note: "prikk does not rewrite or move published history; there is no force-push.",
    },
    TermMapping {
        git: "merge conflict / resolve",
        prikk: "merge evidence + refusal",
        note: "A non-confluent merge refuses with typed conflict witnesses; there are no conflict markers to resolve.",
    },
    TermMapping {
        git: "tag",
        prikk: "tag (signed pointer)",
        note: "Tags exist, but a received tag is untrusted until an explicit maintainer adoption re-signs it.",
    },
    TermMapping {
        git: "clone / fetch / push",
        prikk: "bundle export / import + sync",
        note: "Exchange is via bundles and a sync assistant; prikk moves no bytes over a network itself.",
    },
    TermMapping {
        git: "blame / log message",
        prikk: "block lineage (no message/author/date yet)",
        note: "prikk records lineage and key ids; commit messages, authors and dates are not yet persisted.",
    },
];

/// Code entries (witness/finding). A representative sample now; the full sets land with FR-080/FR-100.
static CODE_ENTRIES: &[GlossaryEntry] = &[
    GlossaryEntry {
        code: "unverifiable-author-signature",
        title: "Unverifiable author signature",
        explanation: "No key material is recorded for the author, so the signature cannot be checked. \
                      This is NOT a failure — verify still passes — but it must never be shown as a \
                      green/sound state. It means continuity is unknown here, not that the author is \
                      fake.",
        see_also: &["sound-author-signature"],
    },
    GlossaryEntry {
        code: ".prikkignore",
        title: "A malformed .prikkignore file",
        explanation: "prikk excludes matching worktree paths from commit's walk and \
                      worktree-status's untracked scan using this file, but refuses to proceed when a \
                      rule it cannot use — an absolute path, for example — appears in it (RFC 009 F5). \
                      Fix or remove the offending line outside stikk, then retry: stikk itself never \
                      edits a repository file (CON-1).",
        see_also: &[],
    },
];

/// Look up a code entry (witness kind or verify finding). `None` is the RR-5 degradation: the caller
/// shows prikk's verbatim message and says no gloss exists yet — it never hides the message.
#[must_use]
pub fn lookup(code: &str) -> Option<&'static GlossaryEntry> {
    CODE_ENTRIES.iter().find(|entry| entry.code == code)
}

/// The full Git → prikk terminology mapping, for Help/Terminology.
#[must_use]
pub fn terminology() -> &'static [TermMapping] {
    TERMS
}

/// The code entries stikk currently ships (for the glossary browser's index).
#[must_use]
pub fn code_entries() -> &'static [GlossaryEntry] {
    CODE_ENTRIES
}

/// Which shipped code entries are named in `message`, so a refusal card can link them (FR-111).
/// This only *links* known codes; it never derives an action from the message (C-T2b).
#[must_use]
pub fn codes_in(message: &str) -> Vec<&'static str> {
    CODE_ENTRIES
        .iter()
        .filter(|entry| message.contains(entry.code))
        .map(|entry| entry.code)
        .collect()
}

#[cfg(test)]
mod tests;
