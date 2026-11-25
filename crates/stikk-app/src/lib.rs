//! **The program every stikk front end drives** (RFC 067, accepted 2026-11-24).
//!
//! The application state and its transitions: the view-model stack, the overlay stack, the focused
//! ref, the session's refusal history, and the worker protocol that reaches prikk through
//! [`stikk_core`]. It holds **no repository authority** (design INV-8) — every view-model is a
//! rendered snapshot of what prikk reported — and it owns **no widgets and no terminal**.
//!
//! # What a front end supplies, and what it gets back
//!
//! - **Intents, never key events.** [`App::handle`] takes an [`Intent`]; translating a keyboard into
//!   one is the front end's (`stikk-tui`'s `keys::dispatch` does it for crossterm). So the next
//!   front end is not obliged to invent terminal events to drive the program.
//! - **Colours are not here.** What a screen can display is the front end's business (RFC 067
//!   decision 5): the drawing holds the palette and composes Orientation's `colours` row itself.
//! - **The reader's place is measured by the front end and decided here** (decision 4). After it
//!   lays a surface out, the front end calls [`App::report_reading`] with a [`Reading`]; the program
//!   answers an intent that might scroll with an [`Effect`], which the front end performs. The
//!   offsets — how far a text is scrolled, and how far it goes — live in the front end, because
//!   only it knows how the text came out on a screen of a given size.
//! - **`Seen::WholeCard` is made here and nowhere else**, from an `at_end` report for the current
//!   surface. A front end that never reports leaves every confirmation closed, which is the safe
//!   failure.
//!
//! # Why it is its own crate
//!
//! So that an operation present in one front end and absent from another is impossible, and so that
//! nothing in the program can quietly start depending on a terminal: `deny.toml` bans ratatui and
//! crossterm outside `stikk-tui` (decision 6), checked by the supply-chain workflow on every push
//! rather than remembered.
//!
//! **One path per item.** Everything public is named at the crate root — `stikk_app::App`,
//! `stikk_app::Overlay`, `stikk_app::Intent` — and the modules are private, so each thing has one
//! name (RFC 067 Handoff B §1.4).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod app;
mod cards;
mod overlay;
mod worker;

pub use app::{
    App, CompareFocus, CompareSource, Direction, Effect, Focus, Intent, Operation, OperationStatus,
    OrientationState, PatchSource, Reading, RefFocus, Screen, Session, Surface,
};
pub use overlay::{Overlay, RefNameKind, RefWriteKind, SavePathKind};
pub use worker::{OrientRead, Request, RequestKind, Response, ResponseKind, run};
