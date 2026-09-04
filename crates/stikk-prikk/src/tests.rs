//! Compile-time bound assertions for the seam's implementors (design SEAM-02; RFC 010 §8).
//!
//! This is a compile-time check disguised as a test: if either backend later gains interior mutability
//! (an `Rc`, `RefCell`, or non-`Sync` field) that breaks `Send + Sync`, this file fails to *build*, not
//! merely to pass — the loudest failure mode available.

use crate::{CliBackend, NullBackend};

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn cli_backend_and_null_backend_are_send_and_sync() {
    assert_send_sync::<CliBackend>();
    assert_send_sync::<NullBackend>();
}
