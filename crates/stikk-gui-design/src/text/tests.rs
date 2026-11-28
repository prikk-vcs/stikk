//! The role table, and the isolating text function.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{isolate, role_sizes};

/// **Rule 2's role table, and nothing between the roles.**
#[test]
fn the_role_table_is_the_four_roles() {
    assert_eq!(role_sizes(), [14.0, 16.0, 18.0, 24.0]);
    // **Nothing below 12** (rule 2), and nothing a user must read to act below 16.
    for size in role_sizes() {
        assert!(size >= 12.0, "{size} is below 12 px");
    }
}

/// **Rule 12: the isolates, around the value and nowhere else.**
#[test]
fn a_value_is_wrapped_in_the_isolates() {
    assert_eq!(isolate("heads/main"), "\u{2066}heads/main\u{2069}");
    // Empty is still isolated: a name that is absent is a fact, and the isolates cost nothing.
    assert_eq!(isolate(""), "\u{2066}\u{2069}");
    // The value is untouched inside them — the escaping rule is the front end's, and it runs
    // before this.
    assert_eq!(isolate("a\\u0020b.txt"), "\u{2066}a\\u0020b.txt\u{2069}");
}
