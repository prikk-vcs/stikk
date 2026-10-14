//! Tests for the Patch-detail view-model (RFC 035 §7).
//!
//! The values here are the shapes `cli_backend::parse_json` produces from reports captured off a real
//! prikk 0.51.0 binary — the reader's own tests hold those fixtures. What is tested here is what stikk
//! *says* about them: the sentences, the label, the bound, and the three states a message has.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use super::*;
use stikk_prikk::{NullBackend, ShowReport};

const PATCH_A: &str = "4db1669a176b64136838975ef7e2724e2161ad78b89c6dabc4efa24cd6818f66";
const PATCH_B: &str = "c23a5d7d4d5d639b3e41e76806158e4bc475f113f3ed85858501010c2d830999";
const BLOCK: &str = "fc583771beda77401062553767f20b1d17f313f28e3add6c3722ff2ad85e2cb7";
const BLOB_OLD: &str = "a04c7b5b1c1bb9c0b28bee2d5ab4dc5707c3583199301c5e3eb7a069cbb28dc2";
const BLOB_NEW: &str = "8d71c569e89311208708dae4774995c92e9f15fd9ed2f026defa0db3ca45ba7d";
const NODE: &str = "ab011b1bb05d2b0a527fe45c4fc20eba9604233dea617c1551031cf2fc01acd2";

/// One operation's rendered rows as plain text, for the assertions that are about the words rather
/// than about which rows are content. The ones that *are* about that compare `OperationRow` itself.
fn texts(view: &PatchDetailView, at: usize) -> Vec<String> {
    view.operations[at].rows.iter().map(row_text).collect()
}

/// One row's text, whichever kind it is — a `Named` row flattened, which is what a test about the
/// words wants and what a renderer must never do.
fn row_text(row: &OperationRow) -> String {
    match row {
        OperationRow::Named(pieces) => crate::flatten(pieces),
        other => other.text().unwrap_or_default().to_string(),
    }
}

fn operation(kind: &str, paths: Vec<QueuedPath>, content: OperationContent) -> PatchOperation {
    PatchOperation {
        kind: kind.to_string(),
        paths,
        content,
        raw: format!("{{\n  \"kind\": \"{kind}\"\n}}"),
    }
}

fn path(p: &str) -> QueuedPath {
    QueuedPath::Path(p.to_string())
}

fn patch(id: &str, queued: bool, operations: Vec<PatchOperation>) -> ShownPatch {
    ShownPatch {
        patch_id: id.to_string(),
        queued,
        operations,
    }
}

fn edit(path_or_node: QueuedPath, old: &str, new: &str) -> PatchOperation {
    operation(
        "edit-text",
        vec![path_or_node],
        OperationContent::TextEdit {
            old_span: old.to_string(),
            replacement: new.to_string(),
        },
    )
}

/// Every operation kind, rendered in stikk's words with prikk's values (RFC 035 decision 2).
#[test]
fn every_kind_is_rendered_from_prikks_own_fields() {
    let patch = patch(
        PATCH_A,
        false,
        vec![
            edit(path("a.txt"), "two", "TWO"),
            operation(
                "create-file",
                vec![path("created.txt")],
                OperationContent::FileCreated {
                    content: FileContent::Text("new file\n".to_string()),
                    mode: 33188,
                },
            ),
            operation(
                "delete-node",
                vec![path("del.txt")],
                OperationContent::NodeDeleted {
                    node_kind: "file".to_string(),
                    content: FileContent::Text("gone\n".to_string()),
                },
            ),
            operation(
                "replace-binary",
                vec![path("b.bin")],
                OperationContent::BinaryReplaced {
                    old: BinaryBlob {
                        blob_id: BLOB_OLD.to_string(),
                        size: 6,
                    },
                    new: BinaryBlob {
                        blob_id: BLOB_NEW.to_string(),
                        size: 7,
                    },
                },
            ),
            operation(
                "change-perm",
                vec![path("mode.sh")],
                OperationContent::PermChanged {
                    old_mode: 33188,
                    new_mode: 33261,
                },
            ),
            operation(
                "rename-path",
                vec![path("mode.sh"), path("moved.sh")],
                OperationContent::PathRenamed {
                    author_key_id: "author".to_string(),
                    author_key_ids: vec!["author".to_string()],
                },
            ),
        ],
    );
    // A block of one: the block's end *is* this patch's end, so no label (decision 3a).
    let view = patch_detail_in_block(
        &patch,
        BLOCK,
        &PatchMessageState::Text("every kind".into()),
        1,
    );

    assert_eq!(view.patch_id, PATCH_A);
    assert_eq!(
        view.heading,
        [
            format!("sealed · in block {BLOCK}"),
            "every kind".to_string()
        ]
    );
    let titles: Vec<String> = view
        .operations
        .iter()
        .map(|o| crate::flatten(&o.title))
        .collect();
    assert_eq!(
        titles,
        [
            "edit-text a.txt",
            "create-file created.txt",
            "delete-node del.txt",
            "replace-binary b.bin",
            "change-perm mode.sh",
            "rename-path mode.sh → moved.sh",
        ],
        "prikk's kind word verbatim, and its own path"
    );
    assert_eq!(
        texts(&view, 0),
        ["span replaced:", "\"two\"", "replacement:", "\"TWO\""],
        "spans, labelled as spans — no line numbers, no context, no line-diff (RFC 021 F2)"
    );
    assert_eq!(
        texts(&view, 1),
        ["mode 100644", "content:", "\"new file\\n\""],
        "the created file's own trailing newline is visible, inside the quotes"
    );
    assert_eq!(texts(&view, 2), ["removed content:", "\"gone\\n\""]);
    assert_eq!(
        texts(&view, 3),
        [
            "old blob — 6 byte(s)",
            BLOB_OLD,
            "new blob — 7 byte(s)",
            BLOB_NEW
        ],
        "blob ids and sizes, and never bytes"
    );
    assert_eq!(
        texts(&view, 4),
        ["mode 100644 → 100755"],
        "octal here; prikk's own integers are in the raw view"
    );
    assert_eq!(texts(&view, 5), ["asserted by (AUTHOR key id) author"]);
    // prikk's own integers, in the raw view, beside the octal above — never instead of it.
    assert!(view.operations.iter().all(|o| !o.raw.is_empty()));
}

/// **An empty span reads as empty, never as a blank row** (RFC 035 decision 2). Measured live at
/// 0.51.0: an append reports `old_span_text: ""`.
#[test]
fn an_empty_span_says_it_is_empty() {
    let view = patch_detail_in_block(
        &patch(PATCH_A, false, vec![edit(path("z.txt"), "", "more\n")]),
        BLOCK,
        &PatchMessageState::Text("append".into()),
        1,
    );
    assert_eq!(
        texts(&view, 0),
        ["span replaced: (empty)", "replacement:", "\"more\\n\""]
    );
}

/// **Decision 3a: the label is about the block's patch count**, nothing else — not about whether a
/// later patch renames anything (§5 trap 3), and never on a queued patch or a block of one.
#[test]
fn the_block_end_label_follows_the_blocks_patch_count_and_the_kind() {
    let rename = operation(
        "rename-path",
        vec![path("a.txt"), path("z.txt")],
        OperationContent::PathRenamed {
            author_key_id: "author".to_string(),
            author_key_ids: vec!["author".to_string()],
        },
    );
    let perm = operation(
        "change-perm",
        vec![path("m.sh")],
        OperationContent::PermChanged {
            old_mode: 33188,
            new_mode: 33261,
        },
    );
    let sealed = patch(
        PATCH_A,
        false,
        vec![
            edit(path("z.txt"), "", "more\n"),
            perm.clone(),
            rename.clone(),
        ],
    );

    // One patch: no label anywhere.
    let alone = patch_detail_in_block(&sealed, BLOCK, &PatchMessageState::NotReported, 1);
    assert!(
        alone
            .operations
            .iter()
            .all(|o| !crate::flatten(&o.title).contains(AS_AT_BLOCK_END)),
        "{:?}",
        alone.operations
    );

    // Two patches: the three node-addressed kinds carry it, and the other three do not — they carry
    // their own paths in the payload (F0), so there is nothing resolved and nothing to label.
    let in_two = patch_detail_in_block(&sealed, BLOCK, &PatchMessageState::NotReported, 2);
    assert_eq!(
        crate::flatten(&in_two.operations[0].title),
        format!("edit-text z.txt — {AS_AT_BLOCK_END}")
    );
    assert_eq!(
        crate::flatten(&in_two.operations[1].title),
        format!("change-perm m.sh — {AS_AT_BLOCK_END}")
    );
    assert_eq!(
        crate::flatten(&in_two.operations[2].title),
        "rename-path a.txt → z.txt",
        "a rename names its own two paths, unlabelled"
    );

    // A queued patch is never labelled: it has no block (F2).
    let queued = patch(PATCH_A, true, vec![edit(path("a.txt"), "", "more\n")]);
    let queued = patch_detail_in_block(&queued, BLOCK, &PatchMessageState::Absent, 2);
    assert_eq!(
        crate::flatten(&queued.operations[0].title),
        "edit-text a.txt"
    );
    assert_eq!(queued.heading[0], "queued", "and it names no block");
}

/// **An unresolved path gets one sentence and no reason** (RFC 035 decision 3, as amended): stikk
/// cannot know which of F1's or F7's reasons applies, and prikk's node id is shown beside it.
#[test]
fn an_unresolved_path_gets_one_sentence_and_no_reason() {
    let view = patch_detail_in_block(
        &patch(
            PATCH_A,
            false,
            vec![edit(
                QueuedPath::UnresolvedNode(NODE.to_string()),
                "one",
                "two",
            )],
        ),
        BLOCK,
        &PatchMessageState::Text("edit d".into()),
        2,
    );
    assert_eq!(
        crate::flatten(&view.operations[0].title),
        "edit-text",
        "no path to name"
    );
    assert_eq!(
        texts(&view, 0),
        [
            NO_PATH_REPORTED.to_string(),
            format!("node {NODE}"),
            "span replaced:".to_string(),
            "\"one\"".to_string(),
            "replacement:".to_string(),
            "\"two\"".to_string(),
        ]
    );
    // No reason, at any version — and no label either: there is no name to label.
    let rendered = texts(&view, 0).join(" ");
    assert!(!rendered.contains("no block yet"), "{rendered}");
    assert!(!rendered.contains("deleted"), "{rendered}");
    assert!(!rendered.contains(AS_AT_BLOCK_END), "{rendered}");
}

/// **`unavailable` is prikk's own word for a blob it does not hold**, and the blob id is still shown —
/// so a reader has the id prikk could not read (review v1 §2.4). Measured at 0.51.0 for a preimage an
/// earlier patch of the same block authored; prikk's `DC-65` makes it deliberate, not a fault.
#[test]
fn an_unavailable_blob_is_named_with_the_id_prikk_could_not_read() {
    let view = patch_detail_in_block(
        &patch(
            PATCH_A,
            false,
            vec![operation(
                "delete-node",
                vec![path("d.txt")],
                OperationContent::NodeDeleted {
                    node_kind: "file".to_string(),
                    content: FileContent::Unavailable(
                        "0bd52da375c2211f015d1d740130d5e93f903cec906b1cbaf65b80e1d44a16e8"
                            .to_string(),
                    ),
                },
            )],
        ),
        BLOCK,
        &PatchMessageState::Text("delete d".into()),
        2,
    );
    assert_eq!(
        texts(&view, 0),
        [
            "removed content: prikk reports the content as unavailable",
            "blob 0bd52da375c2211f015d1d740130d5e93f903cec906b1cbaf65b80e1d44a16e8"
        ]
    );
}

/// An operation kind stikk does not render still appears, with prikk's word and a pointer to the raw
/// view (§3 rule 3). **Never an empty operation.**
#[test]
fn an_operation_kind_stikk_does_not_render_still_appears() {
    let view = patch_detail_in_block(
        &patch(
            PATCH_A,
            false,
            vec![operation(
                "rewrite-span",
                vec![path("a.txt")],
                OperationContent::Unrendered,
            )],
        ),
        BLOCK,
        &PatchMessageState::NotReported,
        1,
    );
    assert_eq!(
        crate::flatten(&view.operations[0].title),
        "rewrite-span a.txt"
    );
    assert_eq!(
        texts(&view, 0),
        ["stikk does not render this operation kind — v shows prikk's own fields"]
    );
}

/// **The bound, at it, under it and over it** (decision 7), including a multi-byte character astride
/// the cut. A silent truncation is the defect the rule exists to prevent (`C-T2c′`).
#[test]
fn content_is_bounded_and_the_cut_is_said_out_loud() {
    let under = "x".repeat(CONTENT_DISPLAY_BYTES - 1);
    let at = "x".repeat(CONTENT_DISPLAY_BYTES);
    let over = "x".repeat(CONTENT_DISPLAY_BYTES + 10);
    let rows = |text: &str| -> Vec<String> {
        patch_detail_in_block(
            &patch(
                PATCH_A,
                false,
                vec![operation(
                    "create-file",
                    vec![path("big.txt")],
                    OperationContent::FileCreated {
                        content: FileContent::Text(text.to_string()),
                        mode: 33188,
                    },
                )],
            ),
            BLOCK,
            &PatchMessageState::NotReported,
            1,
        )
        .operations[0]
            .rows
            .iter()
            .map(row_text)
            .collect::<Vec<_>>()
    };

    assert_eq!(
        rows(&under),
        [
            "mode 100644".to_string(),
            "content:".to_string(),
            format!("\"{under}\"")
        ]
    );
    assert_eq!(
        rows(&at),
        [
            "mode 100644".to_string(),
            "content:".to_string(),
            format!("\"{at}\"")
        ]
    );
    let cut = rows(&over);
    assert_eq!(
        cut[2],
        format!("\"{at}\""),
        "cut at the bound — and the quotes are stikk's, so the count below is of prikk's bytes"
    );
    assert_eq!(cut[3], "… 10 more bytes not shown");

    // A multi-byte character astride the cut: the bound steps back to a `char` boundary, and the count
    // is of the **bytes** withheld, which is what the reader would have to ask prikk for.
    let astride = format!("{}\u{1f600}tail", "x".repeat(CONTENT_DISPLAY_BYTES - 2));
    let cut = rows(&astride);
    assert_eq!(
        cut[2],
        format!("\"{}\"", "x".repeat(CONTENT_DISPLAY_BYTES - 2)),
        "the emoji starts inside the bound and does not fit, so it is withheld whole"
    );
    assert_eq!(
        cut[3], "… 8 more bytes not shown",
        "4 for the emoji, 4 for `tail`"
    );
}

/// **Every row of review v1 §2.1's table: a change that used to render as no change.**
///
/// Spans are where whitespace edits live, so each of these is the common case rather than an edge
/// one — and each rendered identically on both sides until content rows became quoted and escaped.
/// What is asserted is the invariant, not the spelling: **two spans that differ in any byte never
/// render the same.**
#[test]
fn two_spans_that_differ_in_any_byte_never_render_identically() {
    let rows = |old: &str, new: &str| -> Vec<String> {
        patch_detail_in_block(
            &patch(PATCH_A, false, vec![edit(path("a.txt"), old, new)]),
            BLOCK,
            &PatchMessageState::NotReported,
            1,
        )
        .operations[0]
            .rows
            .iter()
            .map(row_text)
            .collect()
    };
    let cases: [(&str, &str, &str); 6] = [
        (
            "a re-indent",
            "  x
",
            "    x
",
        ),
        (
            "a trailing newline removed",
            "x
",
            "x",
        ),
        ("a double space closed up", "a  b", "a b"),
        ("a tab turned into spaces", "\tx", "    x"),
        (
            "trailing whitespace removed",
            "x  
",
            "x
",
        ),
        ("whitespace-only against empty-looking", "   ", " "),
    ];
    for (what, old, new) in cases {
        let rendered = rows(old, new);
        // Rows 1 and 3 are the two spans; rows 0 and 2 are stikk's labels.
        assert_ne!(
            rendered[1], rendered[3],
            "{what}: prikk reported a change and stikk rendered none — {rendered:?}"
        );
        // And each row really is the span, byte for byte, inside the quotes.
        for (span, row) in [(old, &rendered[1]), (new, &rendered[3])] {
            assert_eq!(
                row,
                &quoted(span),
                "{what}: the row is the span, escaped and quoted"
            );
        }
    }
}

/// **Review v2 §2: the invariant above was not true yet.** Quoting made whitespace visible; it did
/// nothing about characters that are invisible or that lie about which way the row reads, because
/// the escape set was [`char::is_control`] — Unicode category `Cc` alone. Each pair here differs in
/// a byte and looked identical on screen.
#[test]
fn an_invisible_character_never_renders_as_its_visible_look_alike() {
    let row = |text: &str| -> String {
        patch_detail_in_block(
            &patch(PATCH_A, false, vec![edit(path("a.txt"), text, "unused")]),
            BLOCK,
            &PatchMessageState::NotReported,
            1,
        )
        .operations[0]
            .rows[1]
            .text()
            .unwrap_or_default()
            .to_string()
    };
    let cases: [(&str, &str, &str); 6] = [
        ("a non-breaking space", "a b", "a\u{a0}b"),
        ("a zero-width space", "x", "x\u{200b}"),
        ("a zero-width joiner", "ab", "a\u{200d}b"),
        ("a soft hyphen", "co-op", "co\u{ad}-op"),
        ("an ideographic space", "a b", "a\u{3000}b"),
        ("a tag character", "A", "A\u{e0041}"),
    ];
    for (what, visible, invisible) in cases {
        assert_ne!(
            row(visible),
            row(invisible),
            "{what}: two different spans render alike"
        );
    }
}

/// **A bidi override cannot make Patch detail show a line in a different order from its bytes**
/// (review v2 §2; "Trojan Source", CVE-2021-42574). This is the first stikk view that renders whole
/// files of code, so it is the one that has to resist this (`T-T2`, `T-T4`).
#[test]
fn a_bidi_override_in_a_span_renders_in_byte_order() {
    let span = "if level != \"user\u{202e} \u{2066}// Check if admin\u{2069} \u{2066}\" {";
    let detail = patch_detail_in_block(
        &patch(
            PATCH_A,
            false,
            vec![operation(
                "create-file",
                vec![path("src/auth.rs")],
                OperationContent::FileCreated {
                    content: FileContent::Text(span.to_string()),
                    mode: 33188,
                },
            )],
        ),
        BLOCK,
        &PatchMessageState::NotReported,
        1,
    );
    let rendered: Vec<String> = detail.operations[0].rows.iter().map(row_text).collect();
    assert!(
        rendered
            .iter()
            .all(|row| !row.contains('\u{202e}') && !row.contains('\u{2066}')),
        "no override may reach a row: {rendered:?}"
    );
    assert!(
        rendered.iter().any(|row| row.contains("\\u202e")),
        "and the override is still reported, as an escape: {rendered:?}"
    );
    // The content row is the span's own bytes, in the span's own order.
    assert!(
        rendered.contains(&quoted(span)),
        "the content row is the span, escaped and quoted: {rendered:?}"
    );
}

/// Indented source keeps its indentation, in a create and in a preimage — the other half of §2.1's
/// table, where a whole file's structure used to go flush left.
#[test]
fn indented_file_content_keeps_its_indentation() {
    let source = "fn main() {\n    let x = 1;\n\tlet y = 2;\n}\n";
    let created = patch_detail_in_block(
        &patch(
            PATCH_A,
            false,
            vec![
                operation(
                    "create-file",
                    vec![path("src/main.rs")],
                    OperationContent::FileCreated {
                        content: FileContent::Text(source.to_string()),
                        mode: 33188,
                    },
                ),
                operation(
                    "delete-node",
                    vec![path("old.rs")],
                    OperationContent::NodeDeleted {
                        node_kind: "file".to_string(),
                        content: FileContent::Text(source.to_string()),
                    },
                ),
            ],
        ),
        BLOCK,
        &PatchMessageState::NotReported,
        1,
    );
    let want = [
        "\"fn main() {\\n\"",
        "\"    let x = 1;\\n\"",
        "\"\\tlet y = 2;\\n\"",
        "\"}\\n\"",
    ];
    assert_eq!(
        texts(&created, 0),
        ["mode 100644", "content:"]
            .iter()
            .chain(want.iter())
            .copied()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        texts(&created, 1),
        ["removed content:"]
            .iter()
            .chain(want.iter())
            .copied()
            .collect::<Vec<_>>()
    );
    // Each content row is marked as content, so the frontend knows not to word-wrap it.
    assert!(
        created.operations[0].rows[2..]
            .iter()
            .all(OperationRow::is_content),
        "{:?}",
        created.operations[0].rows
    );
    assert!(
        !created.operations[0].rows[1].is_content(),
        "the `content:` label is stikk's own words"
    );
}

/// **A binary create or delete shows what `replace-binary` shows** — identity and size, never bytes
/// (review v1 §2.4). Measured at 0.51.0: `{kind: "binary", blob_id, size}`, queued and sealed alike.
#[test]
fn binary_file_content_shows_its_blob_and_size_and_never_bytes() {
    let blob = BinaryBlob {
        blob_id: BLOB_NEW.to_string(),
        size: 12,
    };
    let view = patch_detail_in_block(
        &patch(
            PATCH_A,
            false,
            vec![
                operation(
                    "create-file",
                    vec![path("new.bin")],
                    OperationContent::FileCreated {
                        content: FileContent::Binary(blob.clone()),
                        mode: 33188,
                    },
                ),
                operation(
                    "delete-node",
                    vec![path("old.bin")],
                    OperationContent::NodeDeleted {
                        node_kind: "file".to_string(),
                        content: FileContent::Binary(BinaryBlob {
                            blob_id: BLOB_OLD.to_string(),
                            size: 8,
                        }),
                    },
                ),
            ],
        ),
        BLOCK,
        &PatchMessageState::NotReported,
        1,
    );
    assert_eq!(
        texts(&view, 0),
        ["mode 100644", "content — 12 byte(s)", BLOB_NEW]
    );
    assert_eq!(texts(&view, 1), ["removed content — 8 byte(s)", BLOB_OLD]);
    // Nothing is claimed about what the bytes are, and no row is content to be wrapped verbatim.
    assert!(
        view.operations
            .iter()
            .flat_map(|o| &o.rows)
            .all(|row| !row.is_content()),
        "a blob has no content rows: {view:?}"
    );
}

/// A content kind beyond prikk's three is named with prikk's word, and **claims nothing about whether
/// it carries content** (review v1 §3) — stikk does not know that.
#[test]
fn a_content_kind_stikk_has_never_seen_claims_nothing_about_it() {
    let view = patch_detail_in_block(
        &patch(
            PATCH_A,
            false,
            vec![operation(
                "create-file",
                vec![path("odd")],
                OperationContent::FileCreated {
                    content: FileContent::Unrendered("braille".to_string()),
                    mode: 33188,
                },
            )],
        ),
        BLOCK,
        &PatchMessageState::NotReported,
        1,
    );
    assert_eq!(
        texts(&view, 0),
        [
            "mode 100644",
            "content: prikk reports it as braille, which stikk does not render; v shows prikk's own \
             fields"
        ]
    );
}

/// The three states of a message never collapse into each other (`C-T2c′`).
#[test]
fn a_messages_three_states_stay_three() {
    let one = patch(PATCH_A, true, Vec::new());
    let row = |message: PatchMessageState| {
        patch_detail_in_block(&one, BLOCK, &message, 1).heading[1].clone()
    };
    assert_eq!(
        row(PatchMessageState::Text("a message".into())),
        "a message"
    );
    assert_eq!(row(PatchMessageState::Absent), "(no message)");
    assert_eq!(row(PatchMessageState::NotReported), "(no message reported)");
    assert_eq!(
        PatchMessageState::from_queued(&QueuedMessage::NotReported),
        PatchMessageState::NotReported
    );
    assert_eq!(
        PatchMessageState::from_queued(&QueuedMessage::None),
        PatchMessageState::Absent
    );
    assert_eq!(
        PatchMessageState::from_block_join(None),
        PatchMessageState::NotReported,
        "a patch with no `patch_messages` entry carries no message at all (RFC 015 F4)"
    );
}

/// **Below stikk's JSON floor, the view says so in stikk's own voice** (`C-T4d`) — and does not claim
/// prikk reports nothing, which would be false at 0.36–0.38.
#[test]
fn below_the_json_floor_the_view_names_stikks_own_limit() {
    // The default `NullBackend` reports prikk 0.30 and an `Unreported` show report.
    let backend = NullBackend::supported();
    let view = patch_detail(
        &backend,
        std::path::Path::new("/repo"),
        PATCH_A,
        &PatchMessageState::NotReported,
    )
    .expect("the band is not an error");
    assert!(view.unreported);
    assert!(view.operations.is_empty());
    let foot = view.foot.expect("a reason");
    assert_eq!(
        foot,
        "stikk reads a patch's content from prikk 0.39 on; this is 0.30."
    );
    assert!(!foot.contains("prikk does not report"), "{foot}");
}

/// **A re-read renders what prikk now reports, and carries nothing over** (RFC 035 §4.3, A1 F7 row 2).
///
/// The same patch id, read twice: queued with its path, then — once sealed — `queued: false` with the
/// path gone. The second screen is worse than the first and is the truth.
#[test]
fn a_re_read_of_a_sealed_patch_loses_the_path_it_used_to_name() {
    let queued = patch(PATCH_B, true, vec![edit(path("a.txt"), "", "more\n")]);
    let sealed = patch(
        PATCH_B,
        false,
        vec![edit(
            QueuedPath::UnresolvedNode(NODE.to_string()),
            "",
            "more\n",
        )],
    );
    let repo = std::path::Path::new("/repo");
    let message = PatchMessageState::Text("patch one".to_string());

    let before = patch_detail(
        &NullBackend::supported()
            .with_version(0, 51, 0)
            .with_show(ShowReport::Listed(vec![queued])),
        repo,
        PATCH_B,
        &message,
    )
    .expect("reads");
    assert_eq!(before.heading[0], "queued");
    assert_eq!(
        crate::flatten(&before.operations[0].title),
        "edit-text a.txt"
    );

    let after = patch_detail(
        &NullBackend::supported()
            .with_version(0, 51, 0)
            .with_show(ShowReport::Listed(vec![sealed])),
        repo,
        PATCH_B,
        &message,
    )
    .expect("reads");
    assert_eq!(
        after.heading[0], "sealed",
        "prikk's own word, not stikk's memory"
    );
    assert_eq!(crate::flatten(&after.operations[0].title), "edit-text");
    assert_eq!(texts(&after, 0)[0], NO_PATH_REPORTED);
    assert!(
        !after.operations[0]
            .rows
            .iter()
            .any(|row| row_text(row).contains("a.txt")),
        "nothing is carried over from the read before it: {:?}",
        after.operations[0].rows
    );
}

/// A report that does not list the id it was asked about is said plainly, not rendered as an empty
/// patch.
#[test]
fn a_report_without_the_patch_asked_for_says_so() {
    let backend = NullBackend::supported()
        .with_version(0, 51, 0)
        .with_show(ShowReport::Listed(vec![patch(PATCH_A, true, Vec::new())]));
    let view = patch_detail(
        &backend,
        std::path::Path::new("/repo"),
        PATCH_B,
        &PatchMessageState::NotReported,
    )
    .expect("not an error");
    assert!(!view.unreported);
    assert_eq!(
        view.foot.as_deref(),
        Some("prikk's report for this id does not list it (1 patch(es) listed).")
    );
}
