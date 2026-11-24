//! **Where the program and the drawing meet** (RFC 067 Handoff B §2).
//!
//! RFC 067 split the two: `stikk_app` is the program, and this crate draws it and turns key events
//! into intents. Most of what was `app::tests` went with the program. **These did not**, because
//! each one either presses a key through [`crate::keys::handle_key`] or draws a frame and looks at
//! it — and that is the point of them.
//!
//! *A test that calls a handler proves the handler; only a test that presses a key proves the
//! wiring* (RFC 060 Handoff B's lesson, kept by RFC 067 decision 3). The wiring is here, so the
//! tests of it are here.
//!
//! They reach the program through `stikk_app`'s public API and its `test-fixtures` feature, which
//! is what `App::from_state` is behind: a dev-dependency, off in every normal build.

#[cfg(test)]
mod tests;
