#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use super::*;

#[test]
fn a_published_example_key_is_recognized_and_names_where_it_came_from() {
    let found = published_example(PUBLISHED_EXAMPLE_KEYS[0].public_key).expect("recognized");
    assert!(found.provenance.contains("prikk docs/"));
}

#[test]
fn hex_case_is_not_part_of_the_value() {
    let upper = PUBLISHED_EXAMPLE_KEYS[0].public_key.to_uppercase();
    assert!(published_example(&upper).is_some());
    assert!(published_example(&format!("  {upper}  ")).is_some());
}

#[test]
fn an_ordinary_key_is_not_flagged() {
    assert!(published_example(&"f".repeat(64)).is_none());
    assert!(published_example("").is_none());
}

/// Every entry is a real 64-hex public key with real provenance — a list whose entries could not have
/// come out of prikk would flag the wrong values, which is worse than the honest marker it replaced.
///
/// **The count is pinned**, not merely "non-empty". Completeness of this list was established by
/// sweeping prikk's own documentation, and a sweep is not something to re-run from memory when
/// somebody edits the list. Re-establish it with, at prikk's tag:
///
/// ```sh
/// for f in $(git ls-tree -r --name-only 0.41.0 | grep -E '^(README|docs/src/)' | grep '\.md$'); do
///     git show "0.41.0:$f" | grep -oE '[0-9a-f]{64}'
/// done | sort | uniq -c
/// ```
///
/// Two values at 0.41.0, both below. If that ever returns three, this assertion is what says so.
#[test]
fn every_entry_is_well_formed_and_sourced() {
    assert_eq!(
        PUBLISHED_EXAMPLE_KEYS.len(),
        2,
        "prikk 0.41.0 publishes exactly two example public keys in its user-facing docs; if that \
         changed, re-run the sweep in this test's doc comment rather than adjusting the number"
    );
    for example in PUBLISHED_EXAMPLE_KEYS {
        assert_eq!(example.public_key.len(), 64, "{example:?}");
        assert!(
            example
                .public_key
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "stored lowercase, as prikk prints it: {example:?}"
        );
        assert!(
            example.provenance.contains("prikk ") && example.provenance.contains(':'),
            "provenance must name a file and a line: {example:?}"
        );
    }
    assert!(MEASURED_AT.contains("0.41.0"));
}
