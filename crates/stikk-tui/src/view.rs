//! The top-level views (design TU-01): [`orientation`], [`history`], [`block`] (block detail),
//! [`changes`], [`queue`] and [`patch`] (patch detail). Compare lands against the same shape — each
//! renders a `stikk-core` view-model into a body region and computes nothing itself.

pub mod block;
pub mod changes;
pub mod history;
pub mod orientation;
pub mod patch;
pub mod queue;
