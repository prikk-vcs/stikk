//! The top-level views (design TU-01): [`orientation`], [`history`], [`block`] (block detail),
//! [`changes`], [`queue`], [`patch`] (patch detail) and [`compare`]. Each renders a `stikk-core`
//! view-model into a body region and computes nothing itself.

pub mod block;
pub mod changes;
pub mod compare;
pub mod history;
pub mod orientation;
pub mod patch;
pub mod queue;
