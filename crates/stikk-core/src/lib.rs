//! The operation layer — one shared operation set both frontends drive (design `stikk-04` AR-03,
//! MOD-05).
//!
//! This crate owns no I/O and no widgets. It turns a user intent into a sequence of seam requests
//! plus state reads and view-model productions, applying capability gating and (in later increments)
//! the preview-first rule and confirmation tiers. Because both the TUI and the GUI drive *this* API
//! and neither defines operations of its own, an operation present in one frontend and not the other
//! is impossible (the mechanical guarantee behind TUI/GUI parity, FR-123).
//!
//! This foundation increment implements one operation — [`orient`] — the read-only orientation a
//! session opens with. More operation families land against the same shape.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod orient;

pub use orient::{OrientationView, orient};
