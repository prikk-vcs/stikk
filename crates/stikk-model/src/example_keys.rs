//! prikk's published example public keys, and the check that flags them (`C-S2`).
//!
//! # What this control is, and what it is not
//!
//! prikk's own `security-setup.md` publishes the rule:
//!
//! > *"Any seed or key values published in Prikk's README, quick start, docs, tests, review packages,
//! > or issue comments are public examples. They are compromised by publication and must never be
//! > used for real signing."*
//!
//! **stikk flags the ones it can enumerate, and says so rather than implying more.** That rule's own
//! scope includes prikk's *tests* and *issue comments*, which no list can close over: a test fixture
//! added tomorrow is a published example by that definition and nothing here would know. What is
//! enumerable is the set published in prikk's **user-facing documentation**, which is where a person
//! following a guide would copy one from — and copying one from a guide is the threat (`T-S2`).
//!
//! # By the public value, never by storing a secret
//!
//! Every constant below is a **public key**. `C-S2`'s own wording is *"by pattern of the public value
//! — never by storing the secret"*, and that is literal here rather than argued: a public key is
//! public, and `C-I1a–e` is untouched.
//!
//! **RFC 023 Q1's derivation route is gone, and that is why these are copied rather than derived.**
//! Q1 resolved to derive the comparison values from prikk's *published seeds* using prikk's own
//! `key public`, so the stored values would be captured from the binary rather than transcribed. At
//! prikk 0.41 **there are no published seeds left in the user-facing docs** — `PRIKK_*_SEED` was
//! retired at 0.40 and the pages that exported them were rewritten (measured: the two seeds RFC 023
//! recorded return zero hits in `README.md` and `docs/src/` at the 0.41.0 tag). What the docs publish
//! now is public keys directly, which removes the derivation step: the value stikk compares is the
//! value prikk published, copied from a named file and line.
//!
//! **So the drift guard is provenance plus the re-baseline, not a derivation.** Each entry records
//! where it came from and at which version. A re-baseline re-checks them the way it re-checks a
//! captured fixture, and the sweep that finds one is
//! `git ls-files | xargs grep -ln "<value>"`. If prikk publishes a new example key, nothing here
//! notices — that residual drift is real, is the same one RFC 023 Q1 recorded as surviving, and the
//! upstream letter asking prikk to name its own published values remains the fix.

/// One example key prikk publishes, with where it was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExampleKey {
    /// The 64-hex public key, lowercase, exactly as prikk prints it.
    pub public_key: &'static str,
    /// Where prikk publishes it — file and line at [`MEASURED_AT`].
    pub provenance: &'static str,
}

/// The prikk version these were read from. **Re-check at each re-baseline**, as for any captured
/// fixture: an example that stops being published is one stikk should stop flagging, and a new one is
/// one stikk is not yet flagging.
pub const MEASURED_AT: &str = "prikk 0.41.0, read 2026-09-13";

/// Every example public key stikk knows prikk publishes in its user-facing documentation.
///
/// Read from the `0.41.0` tag. Both appear as the argument to `trust maintainer add --public-key`,
/// which is exactly the copy-paste path `T-S2` describes.
pub const PUBLISHED_EXAMPLE_KEYS: &[ExampleKey] = &[
    ExampleKey {
        public_key: "a00899dfd3357aee69729405913f9324dfc033cec04a2215239eda64ae6d9d91",
        provenance: "prikk docs/src/guide/tutorial.md:99, and backup-restore.md:27 and :211",
    },
    ExampleKey {
        public_key: "27b081593fa86489f9356ef4bc0cbf5f4a5a5b708aa1a10f1a8187fd56a34801",
        provenance: "prikk docs/src/guide/first-run.md:265 and :269",
    },
];

/// Whether `public_key` is one prikk publishes as an example — and so is compromised by publication.
///
/// Case-insensitive on the hex, because a value that reaches stikk through a user's own shell may be
/// upper-cased; the comparison is on the value, and hex case is not part of it.
#[must_use]
pub fn published_example(public_key: &str) -> Option<&'static ExampleKey> {
    PUBLISHED_EXAMPLE_KEYS
        .iter()
        .find(|example| example.public_key.eq_ignore_ascii_case(public_key.trim()))
}

#[cfg(test)]
mod tests;
