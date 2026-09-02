//! The top-level views (design TU-01). This increment ships one: [`orientation`]. More views
//! (History, Patch/Block detail, Compare, Changes) land against the same shape — each renders a
//! `stikk-core` view-model into a body region and computes nothing itself.

pub mod orientation;
