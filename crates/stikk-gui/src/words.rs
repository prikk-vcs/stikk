//! **Every sentence the desktop app writes of its own, keyed** (RFC 069 rules 11 and 13).
//!
//! **Three text sources, and no fourth** (rule 13): the action catalog
//! (`stikk_core::actions`), the glossary (`stikk_core::glossary`), and this table. A word that is
//! about an action comes from the catalog; a word that explains prikk's vocabulary comes from the
//! glossary; everything else is here.
//!
//! **No sentence is assembled from fragments** (rule 13). Each one is a whole template with named
//! placeholders in `{braces}`, filled by [`Say::fill`], so a second language is a second table and
//! not a hunt through the views. That is also why a view never calls `format!` on anything a user
//! reads — a source check holds it (rule 13's test).
//!
//! **The words are the terminal's** (rule 11). Where the terminal says the same thing, the
//! template is its sentence, character for character, and a test compares the two where the
//! terminal's own words are reachable as a constant.

/// One sentence the desktop app can write.
///
/// Every variant's template is in [`Say::template`], and nothing else in the crate holds a
/// user-facing sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Say {
    /// The window's and the header line's name for the app and the repository it has open.
    Header,
    /// A screen's own name, in the header line.
    ScreenName,
    /// prikk's version is inside stikk's validated range.
    PrikkSupported,
    /// prikk is outside the range stikk validated, below the floor.
    PrikkUnsupported,
    /// prikk is newer than the ceiling stikk validated against.
    PrikkUnvalidated,
    /// The signing summary's two roles.
    Signing,
    /// The repository is read-only for this session.
    SigningReadOnly,
    /// prikk 0.40 cannot report its signing keys, and 0.41 can.
    SigningUnverifiable,
    /// A `PRIKK_*_SEED` variable is set and this prikk ignores it.
    SigningStaleSeed,
    /// The queue's own target ref, beside its depth.
    QueuedTargeting,
    /// An interrupted checkout or branch switch.
    Interrupted,
    /// An interrupted commit left a torn tail.
    TornTail,
    /// prikk's current branch, named.
    CurrentBranch,
    /// `heads/main` has no published history yet.
    Unpublished,
    /// Which preset is drawing, and whether it followed the desktop.
    Drawing,
    /// The preset followed the desktop's own light-or-dark setting.
    DrawingFollows,
    /// The preset was chosen in stikk's configuration, and wins.
    DrawingChosen,
    /// The contrast level was chosen in stikk's configuration; the desktop decides the mode.
    DrawingChosenLevel,
    /// A History screen's own heading.
    HistoryHeading,
    /// One block's row in History.
    HistoryRow,
    /// History has no blocks to show.
    HistoryEmpty,
    /// A screen that has been asked for and has not arrived.
    LoadingScreen,
    /// A screen this handoff does not draw yet.
    NotDrawnYet,
    /// The Glossary's one claim, which rests on an invariant and not on a feature set.
    GlossaryClaim,
    /// The Glossary's "Keys" section.
    GlossaryKeys,
    /// A letter means one action, in either case.
    GlossaryLetters,
    /// Every other action is found by name.
    GlossaryByName,
    /// One key's row in the Glossary.
    GlossaryKeyRow,
    /// The Glossary's "Git → prikk" section.
    GlossaryTerms,
    /// One Git → prikk redirect.
    GlossaryTermRow,
    /// The Glossary's section for the codes prikk reports.
    GlossaryCodes,
    /// The Glossary's section for stikk's own notation.
    GlossaryNotation,
    /// One entry on the key line, and in the Glossary's key list.
    KeyLineEntry,
    /// The way out of a read-only card.
    CardWayOut,
    /// How much of a card is still below the window.
    CardMoreBelow,
}

impl Say {
    /// The sentence, with its placeholders.
    #[must_use]
    pub fn template(self) -> &'static str {
        match self {
            Self::Header => "stikk — {repository}",
            Self::ScreenName => "{screen}",
            Self::PrikkSupported => "supported",
            Self::PrikkUnsupported => "outside stikk's validated range — read-only",
            // The terminal's sentence, character for character (`view/orientation.rs`). The
            // number is never written here: it comes from `stikk_prikk`'s own constant, which is
            // the drift that bit the terminal once already (RFC 015).
            Self::PrikkUnvalidated => {
                "validated through {through} — this prikk is newer; its output shapes have not \
                 been checked against stikk"
            }
            Self::Signing => "author {author} · maintainer {maintainer}",
            Self::SigningReadOnly => "read-only",
            Self::SigningUnverifiable => {
                "prikk 0.40 moved signing keys to a key directory and does not report them; prikk \
                 0.41 answers this directly"
            }
            Self::SigningStaleSeed => "a PRIKK_*_SEED variable is set; this prikk ignores it",
            Self::QueuedTargeting => "targeting {ref}",
            Self::Interrupted => "{sentence}",
            Self::TornTail => {
                "{bytes} trailing partial WAL byte(s) — an interrupted commit left a torn tail"
            }
            Self::CurrentBranch => "prikk's current branch",
            Self::Unpublished => "<unpublished>",
            Self::Drawing => "{preset}, {source}",
            Self::DrawingFollows => "following this desktop's own setting",
            Self::DrawingChosen => "chosen in stikk's configuration",
            Self::DrawingChosenLevel => {
                "high contrast chosen in stikk's configuration, mode following this desktop"
            }
            Self::HistoryHeading => "History · {ref}",
            Self::HistoryRow => "{id}  #{seq}  {kind}  {patches} patch(es)",
            Self::HistoryEmpty => "No blocks on this ref yet.",
            Self::LoadingScreen => "Loading {screen}…",
            Self::NotDrawnYet => {
                "{screen} is drawn by the terminal front end; the desktop app draws it in a later \
                 increment of this release."
            }
            // RFC 018 F1: the line this replaced was true when written and false the moment stikk
            // gained a mutation. A user-facing claim may rest on an invariant this project
            // enforces (CON-1); it must never rest on the set of features that happen to exist.
            Self::GlossaryClaim => {
                "Every repository write happens inside prikk itself, never in stikk."
            }
            Self::GlossaryKeys => "Keys",
            Self::GlossaryLetters => "A letter means one action, in either case.",
            Self::GlossaryByName => {
                "Every other action is found by name: press : for the list, or Enter for what can \
                 be done with the selected row."
            }
            Self::GlossaryKeyRow => "{keys} — {action}",
            Self::GlossaryTerms => "Git → prikk",
            Self::GlossaryTermRow => "{git} → {prikk}",
            Self::GlossaryCodes => "Codes prikk reports",
            Self::GlossaryNotation => "stikk's own notation",
            Self::KeyLineEntry => "{key} {action}",
            Self::CardWayOut => "Esc closes this",
            Self::CardMoreBelow => "↓ scrolls — there is more below",
        }
    }

    /// **The sentence, with its placeholders filled.**
    ///
    /// A placeholder with no value is left as it is written, so a missing value shows up as
    /// `{name}` in the record and in the tree rather than as an empty space nobody notices. A
    /// value with no placeholder is ignored for the same reason: the test over [`Say::every`]
    /// catches both before a release.
    #[must_use]
    pub fn fill(self, values: &[(&str, &str)]) -> String {
        let mut text = self.template().to_string();
        for (name, value) in values {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        text
    }

    /// The names this sentence's template asks for.
    #[must_use]
    pub fn placeholders(self) -> Vec<String> {
        let mut names = Vec::new();
        let mut rest = self.template();
        while let Some((_, after)) = rest.split_once('{') {
            match after.split_once('}') {
                Some((name, tail)) => {
                    names.push(name.to_string());
                    rest = tail;
                }
                None => break,
            }
        }
        names
    }

    /// Every sentence, for the tests that hold the table whole.
    #[must_use]
    pub fn every() -> &'static [Self] {
        &[
            Self::Header,
            Self::ScreenName,
            Self::PrikkSupported,
            Self::PrikkUnsupported,
            Self::PrikkUnvalidated,
            Self::Signing,
            Self::SigningReadOnly,
            Self::SigningUnverifiable,
            Self::SigningStaleSeed,
            Self::QueuedTargeting,
            Self::Interrupted,
            Self::TornTail,
            Self::CurrentBranch,
            Self::Unpublished,
            Self::Drawing,
            Self::DrawingFollows,
            Self::DrawingChosen,
            Self::DrawingChosenLevel,
            Self::HistoryHeading,
            Self::HistoryRow,
            Self::HistoryEmpty,
            Self::LoadingScreen,
            Self::NotDrawnYet,
            Self::GlossaryClaim,
            Self::GlossaryKeys,
            Self::GlossaryLetters,
            Self::GlossaryByName,
            Self::GlossaryKeyRow,
            Self::GlossaryTerms,
            Self::GlossaryTermRow,
            Self::GlossaryCodes,
            Self::GlossaryNotation,
            Self::KeyLineEntry,
            Self::CardWayOut,
            Self::CardMoreBelow,
        ]
    }
}

/// The label a row in Orientation carries, from one list so no view writes one.
///
/// They are the terminal's own labels (`stikk-tui`'s `view/orientation.rs`), in its order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    /// prikk's version and whether stikk validated it.
    Prikk,
    /// The session's derived capability.
    Capability,
    /// Signing readiness, presence only.
    Signing,
    /// How many patches are queued.
    Queued,
    /// An interrupted materialization.
    Interrupted,
    /// A torn tail.
    Warning,
    /// prikk's current branch.
    Branch,
    /// What `heads/main` points at.
    HeadsMain,
    /// Which preset is drawing.
    Drawing,
}

impl Label {
    /// The label's word.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Prikk => "prikk",
            Self::Capability => "capability",
            Self::Signing => "signing",
            Self::Queued => "queued",
            Self::Interrupted => "interrupted",
            Self::Warning => "warning",
            Self::Branch => "branch",
            Self::HeadsMain => "heads/main",
            Self::Drawing => "drawing",
        }
    }
}

/// **The key line** (rule 7): what the catalog binds, here, in the catalog's own words.
///
/// In this module because it is text, and a view may not assemble one (rule 13's check). The
/// separator is punctuation between entries, not a sentence: the entries themselves are templates.
#[must_use]
pub fn key_line(actions: &[&'static stikk_core::Action]) -> String {
    actions
        .iter()
        .map(|action| match action.key {
            Some(key) => Say::KeyLineEntry.fill(&[("key", &key.label()), ("action", action.name)]),
            None => action.name.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

#[cfg(test)]
mod tests;
