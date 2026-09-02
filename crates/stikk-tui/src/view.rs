//! The top-level views (design TU-01). This increment ships three: [`orientation`], [`history`], and
//! [`block`] (block detail). Compare and Changes land against the same shape — each renders a
//! `stikk-core` view-model into a body region and computes nothing itself.

pub mod block;
pub mod changes;
pub mod history;
pub mod orientation;
