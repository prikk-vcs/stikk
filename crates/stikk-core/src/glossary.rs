//! The glossary product asset (design DM-09, FR-111; RFC 007).
//!
//! Two kinds of content ship *with* stikk (a product asset, versioned with the release — never user
//! data, never read from a repository or a network):
//!
//! - **Terminology mapping** — Git → prikk, so Git-shaped expectations are redirected in copy and in
//!   Help (external-design §0). Seeded in full now.
//! - **Code entries** — witness kinds (merge, FR-080), verify finding codes (FR-100), and stable
//!   substrings of a known refusal shape (`.prikkignore`, RFC 009 F5; envelope-schema skew, RFC 012
//!   F-e), keyed by whatever text of prikk's own reliably names the condition. These arrive with the
//!   operations that surface them; a representative sample is seeded now to build and test the lookup
//!   and its degradation.
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
        prikk: "block lineage + message (prikk ≥ 0.32)",
        note: "prikk records lineage and key ids; commit messages persist on prikk ≥ 0.32 (validated, \
               then discarded, below it — `UD-01`). No author name/email or date, ever — a permanent, \
               no-clock design, not a gap.",
    },
];

/// The substring RFC 012 F-e's envelope-schema-skew refusal always contains, regardless of which
/// schema-2 object kind (`Patch`, `Blob`, ...) is involved, which schema number was rejected, or
/// whether prikk wrapped the message inside a `lifecycle replay:` context (as `worktree-status` does).
/// Deliberately not the literal schema numbers or the "accepted: [...]" list — those change the moment
/// prikk ships a new schema, and this string must not.
pub(crate) const SCHEMA_SKEW_CODE: &str = "does not accept envelope schema";

/// The substring a **bundle** offered directly refuses with when it was written by a newer prikk than
/// the one decoding it (RFC 015 F5) — a different, earlier refusal shape than [`SCHEMA_SKEW_CODE`]'s,
/// because a bundle decodes its canonical form before any repository-level schema check ever runs
/// (`bundle verify` "writes nothing, needs no repository"). Deliberately not the literal field-tag
/// number or object-kind name (`PatchPayload`, here; a future schema bump could skew a different kind)
/// — those change with the specific field added, and this string must not.
pub(crate) const BUNDLE_DECODE_SKEW_CODE: &str = "canonical encoding error: unknown";

/// The substring prikk's full-queue precondition always contains (RFC 017 F4) — captured live on the
/// commit path (`node_authoring.rs`'s `"…queued patches, at or above the configured limit…"`) and, by
/// source reading, shared by the rollback-append path's own wording (`active.rs`, "run doctor or seal"
/// instead of "run `prikk seal`"). Both wordings carry prikk's `lock conflict:` class prefix — nothing
/// is locked and no other writer is active, and this is the live defect this increment fixes: shipped
/// stikk was showing `FR-106`'s "another writer is active" gloss directly above prikk's own words
/// telling the user to seal. Deliberately not the queue count or the configured limit, which vary.
pub(crate) const FULL_QUEUE_CODE: &str = "at or above the configured limit";

/// The substring both of prikk's maintainer trust refusals share (RFC 016 §9/v2; RFC 017 F5), captured
/// live at both 0.28.0 and 0.33.0: `"maintainer signer key id … is not trusted by policy"` and
/// `"maintainer signer public key does not match trusted key …"`. Deliberately not either full clause —
/// matching the shorter, shared fragment means one code covers both captured wordings rather than two,
/// and neither wording's own key id is part of the match.
pub(crate) const TRUST_REFUSAL_CODE: &str = "maintainer signer";

/// The substring prikk's repository-path validator always uses when a path contains a backslash
/// (`prikk-object/src/path.rs`, byte-identical at 0.28.0 and 0.38.0 — checked against both tags).
/// **Captured, not transcribed**: it came off the Windows leg of the real-binary suite's first widened
/// matrix run (RFC 022, run `34675099061`), where prikk 0.28 refused a path it had built itself —
/// `error: invalid name: backslashes are not allowed in repository paths`. Deliberately not the
/// `invalid name:` class prefix, which prikk shares with every other name refusal, and deliberately not
/// the offending path, which varies.
pub(crate) const BACKSLASH_PATH_CODE: &str = "backslashes are not allowed in repository paths";

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
    GlossaryEntry {
        code: SCHEMA_SKEW_CODE,
        title: "This repository was written by a newer prikk",
        explanation: "prikk's compatibility guarantee runs one way: a newer prikk can always read what \
                      an older one wrote, but an older prikk reading a newer one's schema is the \
                      direction that is not promised (RFC 012 F-e). Some content here was sealed by a \
                      prikk newer than the one this session is running, and this prikk cannot translate \
                      that schema. stikk cannot change what prikk you run — upgrade the prikk binary \
                      this session uses to one that supports the schema, then retry.",
        see_also: &[BUNDLE_DECODE_SKEW_CODE],
    },
    GlossaryEntry {
        code: BUNDLE_DECODE_SKEW_CODE,
        title: "This bundle was written by a newer prikk",
        explanation: "The same one-way compatibility guarantee as an ordinary repository (a newer \
                      prikk can always read what an older one wrote, never the reverse), but a bundle \
                      hits it earlier: it fails while decoding the bundle's own canonical form, before \
                      any repository is even opened (RFC 015 F5). stikk cannot translate the schema — \
                      upgrade the prikk binary this session uses to one that supports it, then retry.",
        see_also: &[SCHEMA_SKEW_CODE],
    },
    GlossaryEntry {
        code: FULL_QUEUE_CODE,
        title: "The active queue is full",
        explanation: "Nothing is locked and no other writer is involved: the active WAL already holds \
                      as many patches as it is configured to allow, and prikk refuses to add another \
                      one until the queue is sealed (RFC 017 F4). Seal the queue — or, if you control \
                      the threshold, raise `PRIKK_ACTIVE_PATCH_LIMIT` — then retry.",
        see_also: &[],
    },
    GlossaryEntry {
        code: BACKSLASH_PATH_CODE,
        title: "A repository path with a backslash",
        explanation: "prikk stores repository paths with forward slashes on every platform and \
                      refuses any path containing a backslash. There are two ways to see this, and \
                      they need opposite responses. On Windows with prikk 0.28, it is prikk's own \
                      defect and you typed no backslash: its commit-side worktree scan built the path \
                      with the platform separator (`src\\main.rs`) and its own validator then refused \
                      it. prikk fixed that in 0.29.0 — upgrade the prikk binary; nothing about your \
                      repository or your file names is wrong. Only files inside a subdirectory are \
                      affected: a file at the top level of the worktree commits normally. \
                      Anywhere else, a file in the worktree really does have a backslash in its \
                      name; rename it outside stikk (`CON-1`: stikk never edits a repository file), \
                      then retry.",
        see_also: &[],
    },
    GlossaryEntry {
        code: TRUST_REFUSAL_CODE,
        title: "The maintainer key is not adopted",
        explanation: "This is object trust, not ref authority: prikk accepts an adopted key's \
                      signatures on objects, but adopting a key never lets it move a ref (RFC 016 §3, \
                      amended on prikk's reply). The key named above must be adopted in this \
                      repository's trust policy, which is done outside stikk (`prikk trust maintainer \
                      add`) — stikk never generates, reads, or adopts key material itself (`C-I1e`). \
                      stikk cannot verify adoption afterwards on any supported prikk (RFC 016 F3): the \
                      `[MNT]` badge may still read \"adoption unknown\" once the key genuinely is \
                      trusted, and that is not stikk reporting a failure.",
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
