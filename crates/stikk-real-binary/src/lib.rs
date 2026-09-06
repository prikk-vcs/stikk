//! The real-binary integration suite (`TS-07`; RFC 019): drives the same [`stikk_prikk::CliBackend`]
//! the product uses against a real `prikk` binary, through a temporary repository, at both ends of the
//! range [`stikk_prikk::version::supported_minor_range`] reports — never a hardcoded version, so
//! raising the validated ceiling widens what this suite exercises without anyone having to remember a
//! second place to update.
//!
//! **Not a replacement for looking**, and it makes the manual verification ritual this project has
//! performed three times *repeatable*, not obsolete: it asserts repository *state* after commit and
//! seal (re-reading, never trusting stikk's own parse), and that the two refusals commit/seal prevent
//! client-side are refusals a real prikk actually gives when the prevention is bypassed. The test
//! functions themselves live in `tests/real_binary.rs`; this crate's own code is the machinery they
//! share.
//!
//! **Does not run by default.** `cargo test --workspace --locked` (the workspace's existing gate) never
//! runs the tests this crate's `tests/real_binary.rs` marks `#[ignore]`, and never needs a real `prikk`
//! binary to build or pass the one test it does not ignore. See that file's own module doc for exactly
//! how to run the rest on demand.
//!
//! **Why this crate exists separately from `stikk-prikk`'s own tests, rather than living in
//! `stikk-prikk/tests/`:** driving `CliBackend`'s real signing-readiness gate means actually setting
//! *this process's* environment variables (`stikk-prikk::env` reads them directly, by design — that is
//! the product behaviour under test), and `std::env::set_var`/`remove_var` are `unsafe fn` under this
//! workspace's edition (2024). Every other crate in this workspace forbids `unsafe_code` outright
//! (`unsafe is forbidden (no FFI in stikk yet)`); rather than weaken that for code that ships, this
//! crate's own `Cargo.toml` `deny`s it instead (review C2: `deny`, unlike `forbid`, can be locally
//! overridden) with `#[allow(unsafe_code)]` on exactly the three `set_var`/`remove_var` call sites in
//! [`support`] that need it — a fourth `unsafe` anywhere else in this crate still fails the build. This
//! crate is never built into any shipped binary, is never a dependency of a published crate, and is
//! excluded from `cargo package --workspace` (`publish = false`; see the release-prep checklist in
//! `.git-exclude/specs/` for the exact invocation, review C3).
//!
//! **The `C-I1e` boundary, held structurally, not by comment, even here:** `prikk key generate`,
//! `prikk key public --seed-env`, and `prikk setup` are invoked directly with [`std::process::Command`]
//! in [`support`], never through [`stikk_prikk::CliBackend`] — the product's command surface never
//! learns those subcommands, so `stikk-prikk`'s own
//! `cli_backend::tests::the_command_surface_never_names_key_or_setup` keeps passing **unchanged**.
//! Seed values are read into memory only long enough to set a child process's environment or write a
//! `0600` file inside a temp directory this suite deletes on drop; they are never printed, matched into
//! a panic message, or included in a [`std::process::Command`]'s echoed output.

pub mod support;
