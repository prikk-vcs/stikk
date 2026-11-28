//! **Rules 11 and 13:** the words are the terminal's, there is one text source, and every template
//! is whole.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{Label, Say, key_line_as_one_string, key_line_entries};

/// **Every template fills every placeholder it names**, and names no placeholder nobody fills.
///
/// A missing value would show as `{name}` on the screen; this catches it before a release rather
/// than in a capture.
#[test]
fn every_template_fills_every_placeholder_it_names() {
    for say in Say::every() {
        let names = say.placeholders();
        let values: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "·")).collect();
        let filled = say.fill(&values);
        assert!(
            !filled.contains('{') && !filled.contains('}'),
            "{say:?} left something unfilled: {filled:?} (names {names:?})"
        );
    }
}

/// **Every sentence is one template**, so a second language is a second table: none is built by
/// joining two of these together.
#[test]
fn no_template_is_a_fragment_of_another() {
    for say in Say::every() {
        let text = say.template();
        assert!(
            !text.is_empty(),
            "{say:?} has no words, so something else must be writing them"
        );
        assert!(
            !text.starts_with(' ') && !text.ends_with(' '),
            "{say:?} begins or ends with a space, which is how a sentence gets joined to another: \
             {text:?}"
        );
    }
}

/// **Rule 11: every label the desktop app draws for an action is the catalog's label for it.**
#[test]
fn every_action_the_key_line_names_is_the_catalogs() {
    let actions: Vec<&'static stikk_core::Action> = stikk_core::actions::all().iter().collect();
    let line = key_line_as_one_string(&actions);
    for action in &actions {
        assert!(
            line.contains(action.name),
            "the key line names {:?} in the catalog's own words",
            action.name
        );
        if let Some(key) = action.key {
            assert!(
                line.contains(&key.label()),
                "and its key is the catalog's label for it: {:?}",
                key.label()
            );
        }
    }
    // **No vocabulary of stikk's own**: every entry of the line is one catalog action, written as
    // the template writes it, and there is no entry from anywhere else.
    //
    // Entry by entry, and not by deleting the known words from the whole line: a key label is one
    // letter, and deleting a letter everywhere takes it out of the other entries too (measured —
    // the first version of this test reported a line of mangled words as *"words from nowhere"*).
    for entry in key_line_entries(&actions) {
        let entry = entry
            .trim_end_matches(Say::KeyLineSeparator.template())
            .trim();
        let known = actions.iter().any(|action| {
            let whole = match action.key {
                Some(key) => {
                    Say::KeyLineEntry.fill(&[("key", &key.label()), ("action", action.name)])
                }
                None => action.name.to_string(),
            };
            entry == whole
        });
        assert!(known, "the key line holds an entry from nowhere: {entry:?}");
    }
    // **And every entry is one key with one action**, which is what makes the pair unbreakable
    // when the drawing lays each of them out as its own text (C2).
    let entries = key_line_entries(&actions);
    assert_eq!(entries.len(), actions.len());
    for (entry, action) in entries.iter().zip(&actions) {
        assert!(
            entry.contains(action.name),
            "{entry:?} carries its action's name"
        );
        if let Some(key) = action.key {
            assert!(
                entry.starts_with(&key.label()),
                "{entry:?} begins with its own key"
            );
        }
    }
}

/// Orientation's row labels are the terminal's own, in its order.
#[test]
fn the_row_labels_are_the_terminals() {
    let labels: Vec<&str> = [
        Label::Prikk,
        Label::Capability,
        Label::Signing,
        Label::Queued,
        Label::Interrupted,
        Label::Warning,
        Label::Branch,
        Label::HeadsMain,
        Label::Drawing,
    ]
    .iter()
    .map(|label| label.word())
    .collect();
    assert_eq!(
        labels,
        vec![
            "prikk",
            "capability",
            "signing",
            "queued",
            "interrupted",
            "warning",
            "branch",
            "heads/main",
            // The one row that is the desktop's own, and the only one the terminal has no
            // counterpart for: the terminal's is `colours` (RFC 059 decision 10).
            "drawing",
        ]
    );
}
