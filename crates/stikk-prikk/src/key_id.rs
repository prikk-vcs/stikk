//! Signing **key ids**, read from the environment as values — the deliberate mirror image of
//! [`crate::env`], which reads the same family of variables as presence only.
//!
//! # Why this is a second module and not three lines in `env.rs`
//!
//! `env.rs` must never materialize an environment value, and that is not a preference: it is enforced
//! by `env/tests.rs::env_module_never_materializes_a_variable_value`, a source-level scan for
//! `env::var(`, `into_string`, `to_string_lossy`, `to_str(` and `to_owned(` — **precisely the calls
//! reading an id requires**. Putting this there would mean weakening, narrowing, or excepting the
//! single most load-bearing security test in the workspace, in order to add a display nicety. That
//! trade is never worth making, so the two live apart and each guards its own invariant:
//!
//! | Module | May read | Its guard asserts |
//! |---|---|---|
//! | [`crate::env`] | presence only, of anything | no value-materializing call |
//! | this one | `PRIKK_*_KEY_ID` **values** | no reference to a `*_SEED` variable at all |
//!
//! Neither guard weakens the other, and each points away from the other's hazard. This module's own
//! guard is `key_id/tests.rs::key_id_module_never_names_a_seed_variable`.
//!
//! # A key id is not key material
//!
//! `C-I1` is presence-only for **seeds**. An id is a *label* — prikk prints it itself, in `setup`'s
//! output and in its own refusals — and `C-I1b` already says the UI shows key ids (public) only
//! (`NFR-S03`). Reading one is permitted. Reading one *in the module that must not read values* is
//! not, which is the whole point above.
//!
//! # There is no read-time race here, and nothing should be built against one
//!
//! stikk spawns prikk, and prikk **inherits stikk's own process environment**: the id read here and
//! the id prikk signs with come from the same place in the same process, so nothing can change between
//! a preview and its execution. This is unlike repository state, where exactly that race is real and
//! is what RFC 003's change token exists for (`OPL-02`). Do not reach for the change token's equivalent
//! here — there is nothing for it to protect.
//!
//! # Absence is a state, not an error
//!
//! An unset id is the `NotReady` case `capability_gate` already refuses on, reached before any
//! confirmation is drawn. This module returns an absence and invents nothing; no caller may render a
//! placeholder where an id goes, because a placeholder is stikk claiming to know something it does not.

use std::ffi::OsString;

/// The environment variables prikk reads for signing **identity**. The corresponding secret variables
/// are named nowhere in this module — see the module doc, and the guard that enforces it.
const AUTHOR_KEY_ID: &str = "PRIKK_AUTHOR_KEY_ID";
const MAINTAINER_KEY_ID: &str = "PRIKK_MAINTAINER_KEY_ID";

/// Read one id from an injected lookup — the whole logic, so it is tested without touching
/// process-global state, the way [`crate::env::read_readiness_with`] is.
///
/// Two shapes become `None`:
///
/// - **unset**, the ordinary case; and
/// - **set but empty**, because an empty id is not an id, and rendering `""` where a key id belongs
///   would look like a rendering bug rather than like an absence.
///
/// A value that is not valid UTF-8 is **kept, lossily**, rather than discarded. Discarding it would
/// have stikk report "no signing id" while prikk goes on to sign with one — a wrong picture, and the
/// worse failure of the two. The replacement characters are honest about what was there, and every
/// caller renders the result through `text::inert` anyway (`C-T2a`).
fn key_id_with(read: impl Fn(&str) -> Option<OsString>, name: &str) -> Option<String> {
    let value = read(name)?;
    let text = value.to_string_lossy().into_owned();
    (!text.is_empty()).then_some(text)
}

/// The real environment lookup. The only place this module touches the process environment.
fn read(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

/// The AUTHOR key id that will sign, if one is configured (`FL-05` step 5).
#[must_use]
pub fn author_key_id() -> Option<String> {
    key_id_with(read, AUTHOR_KEY_ID)
}

/// The MAINTAINER key id that will sign, if one is configured (`FL-06`, amended by RFC 023 Handoff B).
#[must_use]
pub fn maintainer_key_id() -> Option<String> {
    key_id_with(read, MAINTAINER_KEY_ID)
}

#[cfg(test)]
mod tests;
