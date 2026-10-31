//! Tests for undoing one block (RFC 058).
//!
//! The fixtures are the shapes `measure-undo.sh` provoked against prikk 0.54.1 and 0.55.0
//! (`.git-exclude/reports/058-scope-0-20/`): one block that modified a text file, deleted a file,
//! replaced a binary, added a file, deleted a nested folder, created one, changed a mode, renamed
//! a file, and renamed and edited another — inverted, that measurement gave a tip identical to the
//! parent, every path, mode and byte, with `verify` passing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use stikk_model::{ChangeToken, CurrentBranch, Readiness, RoleReadiness};
use stikk_prikk::{
    BlockRow, CommitResult, Diff, DiffEntry, DiffFile, DiffSide, DiffTarget, History, NullBackend,
    Orientation, PatchMessage, QueuedOnSide, UnsupportedPath, WorktreeEntry, WorktreeStatus,
};

use super::*;

const TIP: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const BLOCK: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const PARENT: &str = "3333333333333333333333333333333333333333333333333333333333333333";

// ------------------------------------------------------------------ fixtures

fn file(mode: u32) -> DiffFile {
    DiffFile {
        kind: "file".to_string(),
        encoding: "text".to_string(),
        mode,
        size: 4,
        content_id: None,
    }
}

/// One entry, with the parent's mode on the left side where prikk reports one.
fn entry(path: &str, status: &str, from_path: Option<&str>, from_mode: Option<u32>) -> DiffEntry {
    DiffEntry {
        path: path.to_string(),
        status: status.to_string(),
        from_path: from_path.map(ToString::to_string),
        from: from_mode.map(file),
        to: Some(file(0o100644)),
        minimal: true,
        hunks: Vec::new(),
        raw: "{}".to_string(),
    }
}

/// An ordinary entry: the parent's mode is a plain file's.
fn plain(path: &str, status: &str) -> DiffEntry {
    entry(path, status, None, Some(0o100644))
}

/// An `added` entry, which prikk reports with **no earlier side at all** — there was no file.
fn added(path: &str) -> DiffEntry {
    entry(path, "added", None, None)
}

fn side(point: &str) -> DiffSide {
    DiffSide {
        point: point.to_string(),
        target: DiffTarget::Block(BLOCK.to_string()),
        queued: QueuedOnSide::NotApplicable,
    }
}

fn diff(entries: Vec<DiffEntry>) -> Diff {
    Diff {
        from: side(PARENT),
        to: side(BLOCK),
        entries,
        unsupported_paths: Vec::new(),
    }
}

fn row(id: &str, kind: &str, parents: u64, messages: &[&str]) -> BlockRow {
    BlockRow {
        block_id: id.to_string(),
        ref_state_id: "r".repeat(64),
        update_seq: 1,
        kind: kind.to_string(),
        rollback_block: false,
        parents,
        patches: messages.len() as u64,
        rollback_patches: 0,
        required_attestations: 0,
        messages: messages
            .iter()
            .map(|message| PatchMessage {
                patch_id: "p".repeat(64),
                message: (*message).to_string(),
            })
            .collect(),
        previous_ref_state: None,
    }
}

fn lineage(blocks: Vec<BlockRow>) -> History {
    History {
        reff: "heads/main".to_string(),
        blocks,
    }
}

fn clean() -> WorktreeStatus {
    WorktreeStatus {
        reff: "heads/main".to_string(),
        clean: true,
        tracked: 3,
        unchanged: 3,
        missing: 0,
        modified: 0,
        untracked: 0,
        unsupported: 0,
        refused: Some(0),
        refused_declarations: Some(0),
        entries: Vec::new(),
        queued_elsewhere: None,
        declarations: Vec::new(),
    }
}

fn untracked(path: &str) -> WorktreeEntry {
    WorktreeEntry {
        kind: "untracked".to_string(),
        path: path.to_string(),
        note: "not in the baseline".to_string(),
        authoring: stikk_prikk::Authoring::Authored,
    }
}

fn on_main() -> CurrentBranch {
    CurrentBranch::Branch(stikk_model::RefName::parse("heads/main").expect("a ref name"))
}

fn orientation(queued: u64) -> Orientation {
    Orientation {
        interrupted_materialization: None,
        queued_patches: queued,
        queued_target: (queued > 0).then(|| "heads/main".to_string()),
        main_ref_state: None,
        trailing_partial_wal_bytes: 0,
        active_patch_warning: None,
        current_branch: on_main(),
    }
}

/// A 0.55 session on `heads/main`, with `BLOCK` as the tip and `PARENT` below it.
fn backend() -> NullBackend {
    NullBackend::supported()
        .with_version(0, 55, 0)
        .with_orientation(orientation(0))
        .with_history(lineage(vec![
            row(BLOCK, "Normal", 1, &["the mistake"]),
            row(PARENT, "Normal", 1, &["before it"]),
        ]))
        .with_worktree_status(clean())
        .with_diff_from_to(PARENT, BLOCK, diff(vec![plain("a.txt", "modified")]))
        .with_change_token(ChangeToken::compose(
            [("heads/main", "0".repeat(64).as_str())],
            0,
            None,
            &on_main(),
        ))
}

/// The same session with `TIP` sealed after `BLOCK`, so there is a later comparison to read.
fn with_a_later_block(later: Vec<DiffEntry>) -> NullBackend {
    backend()
        .with_history(lineage(vec![
            row(TIP, "Normal", 1, &["after it"]),
            row(BLOCK, "Normal", 1, &["the mistake"]),
            row(PARENT, "Normal", 1, &["before it"]),
        ]))
        .with_diff_from_to(BLOCK, TIP, diff(later))
}

fn author() -> Readiness {
    Readiness {
        author: RoleReadiness::Unknown,
        maintainer: RoleReadiness::NotReady,
        read_only: false,
    }
}

fn preview_of(backend: &NullBackend, repo: &Path) -> UndoPreviewOutcome {
    undo_preview(backend, repo, BLOCK, "Undo: the mistake").expect("the reads succeed")
}

fn blocked_reason(outcome: UndoPreviewOutcome) -> String {
    match outcome {
        UndoPreviewOutcome::Blocked(reason) => reason,
        UndoPreviewOutcome::Ready { .. } => panic!("expected a blocked preview"),
    }
}

fn ready(outcome: UndoPreviewOutcome) -> (Box<UndoPreview>, Box<UndoToken>) {
    match outcome {
        UndoPreviewOutcome::Ready { preview, token } => (preview, token),
        UndoPreviewOutcome::Blocked(reason) => panic!("expected Ready, got blocked: {reason}"),
    }
}

/// Nothing was written and nothing that writes was spawned — what every guard must leave behind.
fn wrote_nothing(backend: &NullBackend) {
    assert_eq!(backend.save_file_calls(), 0, "no file was fetched");
    assert!(
        backend.rename_path_calls().is_empty(),
        "no rename was asked for"
    );
    assert_eq!(backend.commit_calls(), 0, "nothing was committed");
    assert!(backend.calls().is_empty(), "no seam call that writes ran");
}

fn repo() -> &'static Path {
    Path::new("/repo")
}

/// A scratch directory that stands in for a repository, removed on drop. Named `stikk-undo-test-`
/// so it can never be mistaken for — or sit inside — the `stikk-undo-` directory the operation
/// itself fetches into.
struct Worktree(PathBuf);

impl Worktree {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "stikk-undo-test-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch repository");
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, relative: &str, bytes: &[u8]) {
        let target = self.0.join(relative);
        std::fs::create_dir_all(target.parent().expect("a parent")).expect("the directories");
        std::fs::write(target, bytes).expect("a worktree file");
    }

    fn read(&self, relative: &str) -> Vec<u8> {
        std::fs::read(self.0.join(relative)).expect("the file is there")
    }

    fn exists(&self, relative: &str) -> bool {
        self.0.join(relative).exists()
    }

    /// Every path under the scratch directory with its bytes, sorted — a byte-identity check.
    fn snapshot(&self) -> Vec<(String, Vec<u8>)> {
        fn walk(root: &Path, at: &Path, into: &mut Vec<(String, Vec<u8>)>) {
            let Ok(entries) = std::fs::read_dir(at) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    into.push((
                        format!("{}/", path.strip_prefix(root).unwrap().display()),
                        Vec::new(),
                    ));
                    walk(root, &path, into);
                } else {
                    into.push((
                        path.strip_prefix(root).unwrap().display().to_string(),
                        std::fs::read(&path).unwrap_or_default(),
                    ));
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.0, &self.0, &mut out);
        out.sort();
        out
    }
}

impl Drop for Worktree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn commit_result() -> CommitResult {
    CommitResult {
        baseline_ref: "heads/main".to_string(),
        patch_id: "c".repeat(64),
        wal_sequence: 1,
        operations: 2,
        referenced_blobs: 1,
        text_edits: 0,
        changes: Vec::new(),
        notes: Vec::new(),
    }
}

// ------------------------------------------------------------------ the band and the branch

/// **Below the band nothing is offered, and the reason says the number** (decision 2, `C-T4d`).
#[test]
fn below_the_band_the_undo_is_unavailable_with_its_reason() {
    let backend = backend().with_version(0, 28, 0);
    let reason = blocked_reason(preview_of(&backend, repo()));
    assert_eq!(reason, undo_unavailable(28));
    assert!(reason.contains("0.54"), "the band's own number: {reason}");
    wrote_nothing(&backend);
}

/// **Decision 3: the current branch, or nothing.** A block that is not in this branch's history is
/// not this operation's block, whatever ref it belongs to — and prikk reporting no current branch
/// at all is the same answer.
#[test]
fn a_block_that_is_not_on_the_current_branch_is_not_offered() {
    let elsewhere = backend().with_history(lineage(vec![row(TIP, "Normal", 1, &["another"])]));
    assert_eq!(
        blocked_reason(preview_of(&elsewhere, repo())),
        NOT_THE_CURRENT_BRANCH
    );
    wrote_nothing(&elsewhere);

    let unreported = backend().with_orientation(Orientation {
        current_branch: CurrentBranch::NotReported,
        ..orientation(0)
    });
    assert_eq!(
        blocked_reason(preview_of(&unreported, repo())),
        NOT_THE_CURRENT_BRANCH
    );
    wrote_nothing(&unreported);
}

/// **The commit's own precondition.** prikk's `commit` refuses while a materialization is
/// interrupted, so an undo offered here would write the worktree and fail at the last step.
#[test]
fn an_interrupted_materialization_is_not_offered_and_carries_prikks_routes() {
    let backend = backend().with_orientation(Orientation {
        interrupted_materialization: Some(vec!["prikk branch switch heads/main".to_string()]),
        ..orientation(0)
    });
    let reason = blocked_reason(preview_of(&backend, repo()));
    assert!(reason.starts_with(INTERRUPTED), "{reason}");
    assert!(
        reason.contains("`prikk branch switch heads/main`"),
        "prikk's own route, verbatim: {reason}"
    );
    wrote_nothing(&backend);
}

// ------------------------------------------------------------------ the seven guards

/// **Guard 1, exactly as decision 4 defines it**: a missing file, a modified file, an entry that is
/// not untracked, or a refused declaration. Each one, its sentence, and nothing written.
#[test]
fn guard_1_a_worktree_with_changes() {
    let cases: Vec<(&str, WorktreeStatus)> = vec![
        (
            "a missing file",
            WorktreeStatus {
                missing: 1,
                clean: false,
                ..clean()
            },
        ),
        (
            "a modified file",
            WorktreeStatus {
                modified: 1,
                clean: false,
                ..clean()
            },
        ),
        (
            "an entry that is not untracked",
            WorktreeStatus {
                entries: vec![WorktreeEntry {
                    kind: "unsupported-path".to_string(),
                    ..untracked("odd\u{7}name")
                }],
                ..clean()
            },
        ),
        (
            "a refused declaration, counted",
            WorktreeStatus {
                refused_declarations: Some(1),
                ..clean()
            },
        ),
        (
            "a refused declaration, named",
            WorktreeStatus {
                declarations: vec![stikk_prikk::RenameDeclaration {
                    old_path: "old.txt".to_string(),
                    new_path: "new.txt".to_string(),
                    resolution: Some(stikk_prikk::DeclarationResolution::Refused),
                    refusal: Some("refusing: the destination exists".to_string()),
                    content_changed: None,
                    mode_changed: None,
                }],
                ..clean()
            },
        ),
    ];
    for (what, status) in cases {
        let backend = backend().with_worktree_status(status);
        assert_eq!(
            blocked_reason(preview_of(&backend, repo())),
            WORKTREE_HAS_CHANGES,
            "{what}"
        );
        wrote_nothing(&backend);
    }
}

/// **An untracked file anywhere blocks** (the review of v1, finding A, measured at 0.55.0): the
/// undo's own `commit --from-worktree` authored a `create-file` for it, because there is no staging
/// (`FR-050`) -- so the undo would carry a file that is no part of it.
///
/// This is the opposite of what RFC 058 said at acceptance, and of what v1 shipped.
#[test]
fn guard_1_an_untracked_file_anywhere_blocks() {
    let backend = backend().with_worktree_status(WorktreeStatus {
        untracked: 1,
        clean: false,
        entries: vec![untracked("notes.txt.swp")],
        ..clean()
    });
    assert_eq!(
        blocked_reason(preview_of(&backend, repo())),
        WORKTREE_HAS_CHANGES
    );
    assert!(
        WORKTREE_HAS_CHANGES.contains("any new file lying around"),
        "the sentence says why an untracked file matters: {WORKTREE_HAS_CHANGES}"
    );
    wrote_nothing(&backend);
}

/// **prikk's own `clean` is asked first**, so a report that is not clean blocks even when every
/// count stikk reads is zero: the counts summarise the entries, and `clean` is prikk's verdict.
#[test]
fn guard_1_blocks_on_prikks_own_verdict() {
    let backend = backend().with_worktree_status(WorktreeStatus {
        clean: false,
        ..clean()
    });
    assert_eq!(
        blocked_reason(preview_of(&backend, repo())),
        WORKTREE_HAS_CHANGES
    );
    wrote_nothing(&backend);
}

/// **A reported refusal blocks, and an unreported verdict does not** (`C-T2c′`): `None` below
/// the band is not a report of zero, and stikk refuses nothing on an absence.
#[test]
fn guard_1_reads_a_refusal_count_but_not_its_absence() {
    let refused = backend().with_worktree_status(WorktreeStatus {
        refused: Some(1),
        ..clean()
    });
    assert_eq!(
        blocked_reason(preview_of(&refused, repo())),
        WORKTREE_HAS_CHANGES
    );
    wrote_nothing(&refused);

    let unreported = backend().with_worktree_status(WorktreeStatus {
        refused: None,
        refused_declarations: None,
        ..clean()
    });
    let (preview, _) = ready(preview_of(&unreported, repo()));
    assert_eq!(preview.plan.steps.len(), 1, "an absence blocks nothing");
}

/// **Guard 2**: the undo must be recorded after what is already queued.
#[test]
fn guard_2_work_is_queued() {
    let backend = backend().with_orientation(orientation(2));
    assert_eq!(blocked_reason(preview_of(&backend, repo())), WORK_IS_QUEUED);
    wrote_nothing(&backend);
}

/// **Guard 3** (F4): `log` names only one parent of a merge — by its kind, and by its parent
/// count, because either one on its own is enough.
#[test]
fn guard_3_a_merge_block() {
    for block in [row(BLOCK, "Merge", 2, &[]), row(BLOCK, "Normal", 2, &[])] {
        let backend = backend().with_history(lineage(vec![
            block,
            row(PARENT, "Normal", 1, &["before it"]),
        ]));
        assert_eq!(blocked_reason(preview_of(&backend, repo())), A_MERGE_BLOCK);
        wrote_nothing(&backend);
    }
}

/// **Guard 4** (F4): with no row below it, or with no parent of its own, there is nothing to
/// compare against.
///
/// **A root block answers this, not guard 3** (leg 6, measured at 0.55.0): prikk reports a root as
/// `parents: 0`, and decision 4's order read literally would have told the reader stikk cannot
/// undo a merge -- which is not true of a root, and names nothing they could act on.
#[test]
fn guard_4_the_first_block_on_the_branch() {
    for block in [
        row(BLOCK, "Normal", 1, &["the first"]),
        row(BLOCK, "Root", 0, &["the root"]),
    ] {
        let backend = backend().with_history(lineage(vec![block]));
        assert_eq!(
            blocked_reason(preview_of(&backend, repo())),
            THE_FIRST_BLOCK
        );
        wrote_nothing(&backend);
    }
    // And a root with a row below it -- which prikk's own `log` cannot produce, a root being the
    // oldest block there is -- still answers guard 4, rather than inverting a comparison against
    // a block that is not its parent.
    let backend = backend().with_history(lineage(vec![
        row(BLOCK, "Root", 0, &["the root"]),
        row(PARENT, "Normal", 1, &["below a root"]),
    ]));
    assert_eq!(
        blocked_reason(preview_of(&backend, repo())),
        THE_FIRST_BLOCK
    );
    wrote_nothing(&backend);
}

/// **Guard 4's other half**: the oldest row of a lineage that came back **full** is not knowably
/// the first block on its branch, so stikk says what is true instead of claiming it is.
#[test]
fn guard_4_a_lineage_that_came_back_full_says_so_rather_than_claiming_the_first_block() {
    let mut blocks: Vec<BlockRow> = (0..UNDO_HISTORY_LIMIT - 1)
        .map(|n| row(&format!("{n:064}"), "Normal", 1, &[]))
        .collect();
    blocks.push(row(BLOCK, "Normal", 1, &["the oldest one read"]));
    assert_eq!(blocks.len(), UNDO_HISTORY_LIMIT);
    let backend = backend().with_history(lineage(blocks));
    assert_eq!(
        blocked_reason(preview_of(&backend, repo())),
        DEEPER_THAN_STIKK_LOOKED
    );
    wrote_nothing(&backend);
}

/// **Guard 5** (F3): a later block changed one of these paths, so a plain inverse would overwrite
/// that change. The sentence names every overlapping path, sorted.
#[test]
fn guard_5_a_later_block_changed_the_same_paths() {
    let backend = with_a_later_block(vec![plain("n.txt", "modified"), plain("a.txt", "modified")])
        .with_diff_from_to(
            PARENT,
            BLOCK,
            diff(vec![plain("a.txt", "modified"), added("n.txt")]),
        );
    let reason = blocked_reason(preview_of(&backend, repo()));
    assert_eq!(
        reason,
        later_block_changed(&["a.txt".to_string(), "n.txt".to_string()])
    );
    assert!(
        reason.contains("`a.txt`") && reason.contains("`n.txt`"),
        "{reason}"
    );
    wrote_nothing(&backend);
}

/// **Guard 5 compares `from_path` too** (§7 trap 4): a rename back onto a path a later block
/// re-created would otherwise slip through, because the earlier name appears on neither side's
/// `path`.
#[test]
fn guard_5_compares_the_earlier_name_of_a_rename() {
    let backend = with_a_later_block(vec![added("old.txt")]).with_diff_from_to(
        PARENT,
        BLOCK,
        diff(vec![entry(
            "new.txt",
            "renamed",
            Some("old.txt"),
            Some(0o100644),
        )]),
    );
    assert_eq!(
        blocked_reason(preview_of(&backend, repo())),
        later_block_changed(&["old.txt".to_string()])
    );
    wrote_nothing(&backend);
}

/// **The tip itself has nothing later**, so undoing it reads one comparison and no second one.
#[test]
fn undoing_the_tip_reads_no_later_comparison() {
    let backend = backend();
    let (preview, _) = ready(preview_of(&backend, repo()));
    assert_eq!(preview.parent_id, PARENT);
    assert_eq!(preview.block_id, BLOCK);
}

/// **Guard 6 asks the filesystem** (the review of v1, finding C): a path `.prikkignore` names is
/// not listed by `worktree-status` at all, so an undo that read the status would have overwritten
/// it. Here the status is **clean** and the file is on disk.
#[test]
fn guard_6_something_already_where_the_undo_would_create() {
    let worktree = Worktree::new("guard6");
    worktree.write("b.txt", b"not the block's\n");
    let cases: Vec<(&str, Vec<DiffEntry>)> = vec![
        ("a deleted file's path", vec![plain("b.txt", "deleted")]),
        (
            "a rename's earlier name",
            vec![entry("new.txt", "renamed", Some("b.txt"), Some(0o100644))],
        ),
    ];
    for (what, entries) in cases {
        let backend = backend().with_diff_from_to(PARENT, BLOCK, diff(entries));
        assert_eq!(
            blocked_reason(preview_of(&backend, worktree.path())),
            in_the_way(&["b.txt".to_string()]),
            "{what}"
        );
        wrote_nothing(&backend);
    }
}

/// **An ancestor that is not a real directory is in the way too**: the undo's own
/// `create_dir_all` would fail part-way, and a symlink there would write outside the worktree.
#[test]
fn guard_6_an_ancestor_that_is_not_a_directory_is_in_the_way() {
    let worktree = Worktree::new("guard6-ancestor");
    worktree.write("dir", b"a file where a directory should be\n");
    let backend =
        backend().with_diff_from_to(PARENT, BLOCK, diff(vec![plain("dir/sub/d.txt", "deleted")]));
    assert_eq!(
        blocked_reason(preview_of(&backend, worktree.path())),
        in_the_way(&["dir/sub/d.txt".to_string()])
    );
    wrote_nothing(&backend);

    // A real directory in the same place is not in the way.
    std::fs::remove_file(worktree.path().join("dir")).expect("remove the file");
    std::fs::create_dir_all(worktree.path().join("dir/sub")).expect("the directories");
    let (preview, _) = ready(preview_of(&backend, worktree.path()));
    assert_eq!(preview.plan.steps.len(), 1);
}

/// **A dangling symlink counts as something being there** -- `symlink_metadata`'s own rule, which
/// is prikk's and `files::already_there`'s.
#[cfg(unix)]
#[test]
fn guard_6_a_dangling_symlink_is_in_the_way() {
    let worktree = Worktree::new("guard6-symlink");
    std::os::unix::fs::symlink("nowhere", worktree.path().join("b.txt")).expect("a symlink");
    let backend = backend().with_diff_from_to(PARENT, BLOCK, diff(vec![plain("b.txt", "deleted")]));
    assert_eq!(
        blocked_reason(preview_of(&backend, worktree.path())),
        in_the_way(&["b.txt".to_string()])
    );
    wrote_nothing(&backend);
}

// ---------------------------------------------------- guard 7: what `.prikkignore` keeps out

/// **A rule that names a path the undo would create blocks** (the review of v1, finding B):
/// measured at 0.55.0, the file goes back, `worktree-status` stays clean, and `commit` records
/// nothing -- so the undo would not be exact.
#[test]
fn guard_7_a_created_path_an_ignore_rule_names_blocks() {
    let worktree = Worktree::new("ignore-named");
    worktree.write(".prikkignore", b"keep.log\n");
    let backend =
        backend().with_diff_from_to(PARENT, BLOCK, diff(vec![plain("keep.log", "deleted")]));
    assert_eq!(
        blocked_reason(preview_of(&backend, worktree.path())),
        ignored_at_commit(&["keep.log".to_string()])
    );
    wrote_nothing(&backend);
}

/// **prikk's own matching rule**, copied from its `ignore.rs`: a whole path component, a run of
/// trailing slashes stripped, blank lines skipped -- and a differently-named sibling untouched.
#[test]
fn guard_7_matches_prikks_own_rule_and_no_more() {
    let worktree = Worktree::new("ignore-rule");
    worktree.write(".prikkignore", b"\ntarget/\n  build/output  \n");
    let under = backend().with_diff_from_to(
        PARENT,
        BLOCK,
        diff(vec![
            plain("target/x.txt", "deleted"),
            plain("build/output", "deleted"),
        ]),
    );
    assert_eq!(
        blocked_reason(preview_of(&under, worktree.path())),
        ignored_at_commit(&["build/output".to_string(), "target/x.txt".to_string()]),
        "a trailing slash is stripped, a blank line is skipped, and both paths are named"
    );

    // **`target2` is not under `target`** -- a whole component, never a string prefix.
    let sibling =
        backend().with_diff_from_to(PARENT, BLOCK, diff(vec![plain("target2", "deleted")]));
    let (preview, _) = ready(preview_of(&sibling, worktree.path()));
    assert_eq!(preview.plan.steps.len(), 1);
}

/// **The rules that matter are the ones in force after the undo.** Where the plan removes
/// `.prikkignore`, there are none; where it writes one the preview cannot read, stikk says so
/// rather than applying rules to a file that will not be there.
#[test]
fn guard_7_reads_the_ignore_file_the_undo_would_leave() {
    let worktree = Worktree::new("ignore-plan");
    worktree.write(".prikkignore", b"keep.log\n");

    // The block **added** `.prikkignore`, so the undo removes it: no rules afterwards.
    let removed = backend().with_diff_from_to(
        PARENT,
        BLOCK,
        diff(vec![added(".prikkignore"), plain("keep.log", "deleted")]),
    );
    let (preview, _) = ready(preview_of(&removed, worktree.path()));
    assert_eq!(preview.plan.steps.len(), 2, "no rule can hide anything");

    // The block **changed** it, so the undo puts the parent's version back -- which the preview
    // cannot read (see `IGNORE_RULES_NOT_READ`).
    let written = backend().with_diff_from_to(
        PARENT,
        BLOCK,
        diff(vec![
            plain(".prikkignore", "modified"),
            plain("keep.log", "deleted"),
        ]),
    );
    assert_eq!(
        blocked_reason(preview_of(&written, worktree.path())),
        IGNORE_RULES_NOT_READ
    );

    // **With nothing to create, there is nothing a rule could hide**, so that block undone on its
    // own is offered.
    let alone =
        backend().with_diff_from_to(PARENT, BLOCK, diff(vec![plain(".prikkignore", "modified")]));
    let (preview, _) = ready(preview_of(&alone, worktree.path()));
    assert_eq!(preview.plan.steps.len(), 1);

    // A rename **onto** `.prikkignore` is undone by renaming it away: no rules afterwards.
    let renamed_onto = backend().with_diff_from_to(
        PARENT,
        BLOCK,
        diff(vec![
            entry(".prikkignore", "renamed", Some("rules.txt"), Some(0o100644)),
            plain("keep.log", "deleted"),
        ]),
    );
    let (preview, _) = ready(preview_of(&renamed_onto, worktree.path()));
    assert_eq!(preview.plan.steps.len(), 2);

    // A rename **away from** it is undone by putting it back, and that content is unread. The
    // worktree's own copy goes first: putting `.prikkignore` back is *creating* a path, so with a
    // file already there guard 6 answers before this one does -- correctly, and tested above.
    std::fs::remove_file(worktree.path().join(".prikkignore")).expect("remove the ignore file");
    let renamed_away = backend().with_diff_from_to(
        PARENT,
        BLOCK,
        diff(vec![
            entry("rules.txt", "renamed", Some(".prikkignore"), Some(0o100644)),
            plain("keep.log", "deleted"),
        ]),
    );
    assert_eq!(
        blocked_reason(preview_of(&renamed_away, worktree.path())),
        IGNORE_RULES_NOT_READ
    );
}

/// **A `.prikkignore` that is not text blocks**, as prikk's own commit refuses it.
#[test]
fn guard_7_an_ignore_file_that_is_not_text_blocks() {
    let worktree = Worktree::new("ignore-bytes");
    worktree.write(".prikkignore", &[0xff, 0xfe, b'\n']);
    let backend =
        backend().with_diff_from_to(PARENT, BLOCK, diff(vec![plain("keep.log", "deleted")]));
    assert_eq!(
        blocked_reason(preview_of(&backend, worktree.path())),
        IGNORE_NOT_UTF8
    );
    wrote_nothing(&backend);
}

/// **A block that changed no file has no inverse**, and its commit would refuse on a clean
/// worktree -- so the preview says so instead of reaching step 4.
#[test]
fn a_block_that_changed_no_file_is_not_offered() {
    let backend = backend().with_diff_from_to(PARENT, BLOCK, diff(Vec::new()));
    assert_eq!(
        blocked_reason(preview_of(&backend, repo())),
        NOTHING_TO_UNDO
    );
    wrote_nothing(&backend);
}

/// **Guard 7**: an entry stikk cannot invert exactly — a status it has not measured (`symlink`
/// included), a path prikk could not compare, a report with no earlier side, and a rename with no
/// earlier name. Each reason names the path.
#[test]
fn guard_7_an_entry_stikk_cannot_invert_exactly() {
    let unmeasured = backend().with_diff_from_to(PARENT, BLOCK, diff(vec![plain("l", "symlink")]));
    let reason = blocked_reason(preview_of(&unmeasured, repo()));
    assert!(
        reason.starts_with("stikk cannot undo this block exactly:"),
        "{reason}"
    );
    assert!(
        reason.contains("`l`") && reason.contains("`symlink`"),
        "{reason}"
    );
    wrote_nothing(&unmeasured);

    let unsupported = backend().with_diff_from_to(
        PARENT,
        BLOCK,
        Diff {
            unsupported_paths: vec![UnsupportedPath {
                path: "odd".to_string(),
                refusal: "refusing: path is not valid UTF-8".to_string(),
            }],
            ..diff(vec![plain("a.txt", "modified")])
        },
    );
    let reason = blocked_reason(preview_of(&unsupported, repo()));
    assert!(reason.contains("`odd`"), "{reason}");
    assert!(
        reason.contains("refusing: path is not valid UTF-8"),
        "prikk's own refusal, verbatim: {reason}"
    );

    let no_earlier = backend().with_diff_from_to(
        PARENT,
        BLOCK,
        diff(vec![entry("a.txt", "modified", None, None)]),
    );
    let reason = blocked_reason(preview_of(&no_earlier, repo()));
    assert!(reason.contains("no earlier file for `a.txt`"), "{reason}");

    let no_name = backend().with_diff_from_to(
        PARENT,
        BLOCK,
        diff(vec![entry("new.txt", "renamed", None, Some(0o100644))]),
    );
    let reason = blocked_reason(preview_of(&no_name, repo()));
    assert!(
        reason.contains("without naming where it came from"),
        "{reason}"
    );
}

/// **F5's two Windows rows**, asked only where `cfg!(windows)` holds and tested everywhere: a
/// `mode` entry has no inverse there, and a deleted executable would come back non-executable.
#[test]
fn guard_7_windows_refuses_the_two_rows_f5_names() {
    assert!(windows_refusal("mode", "run.sh", Some(0o100755)).is_some());
    assert!(windows_refusal("deleted", "run.sh", Some(0o100755)).is_some());
    assert!(
        windows_refusal("deleted", "a.txt", Some(0o100644)).is_none(),
        "a deleted file that was not executable comes back exact"
    );
    assert!(windows_refusal("modified", "a.txt", Some(0o100644)).is_none());
    assert!(windows_refusal("added", "c.txt", None).is_none());
}

// ------------------------------------------------------------------ the inverse table

/// **F2's table, entry by entry** — the measured inverse of each of prikk's statuses, with the
/// parent's own mode carried from the entry's `from.mode` and nothing read for it.
#[test]
fn the_inverse_of_every_entry_f2_names() {
    let plan = UndoPlan::from_diff(
        repo(),
        &diff(vec![
            plain("a.txt", "modified"),
            plain("b.txt", "deleted"),
            plain("bin.dat", "binary"),
            added("c.txt"),
            entry("run.sh", "mode", None, Some(0o100755)),
            entry("new.txt", "renamed", Some("old.txt"), Some(0o100644)),
        ]),
    )
    .expect("every status is one F2 names");

    let rows: Vec<(&str, &str, Option<u32>)> = plan
        .steps
        .iter()
        .map(|step| (step.verb(), step.path.as_str(), step.mode))
        .collect();
    assert_eq!(
        rows,
        vec![
            ("put back", "a.txt", Some(0o100644)),
            ("put back", "b.txt", Some(0o100644)),
            ("put back", "bin.dat", Some(0o100644)),
            ("remove", "c.txt", None),
            ("put back the permissions", "run.sh", Some(0o100755)),
            ("rename back", "new.txt", Some(0o100644)),
        ]
    );
    assert_eq!(
        plan.steps[5].action,
        UndoAction::RenameBack {
            earlier: "old.txt".to_string()
        },
        "a rename back names the parent's own path"
    );
    assert_eq!(
        plan.steps[5].parent_path(),
        "old.txt",
        "which is where prikk fetches its bytes from, and where they go"
    );
    assert_eq!(plan.steps[3].parent_path(), "c.txt");
    assert!(
        !plan.steps[3].puts_bytes_back(),
        "a removal fetches nothing"
    );
    assert!(plan.steps[0].puts_bytes_back());
}

/// **The path check refuses the whole undo** (decision 6), naming the path — and it is part of
/// building a plan at all, so a plan that exists has passed it.
#[test]
fn the_path_check_refuses_a_path_stikk_will_not_write() {
    for path in [
        "../x",
        "/x",
        ".prikk/x",
        "a/../../x",
        "",
        "./x",
        ".prikk/objects/deep",
    ] {
        let refusal = UndoPlan::from_diff(repo(), &diff(vec![plain(path, "modified")]))
            .expect_err("the check refuses it");
        assert_eq!(refusal, unsafe_path(path), "{path}");
    }
    // The earlier name of a rename goes through the same check.
    let refusal = UndoPlan::from_diff(
        repo(),
        &diff(vec![entry(
            "new.txt",
            "renamed",
            Some("../old.txt"),
            Some(0o100644),
        )]),
    )
    .expect_err("the check refuses the earlier name too");
    assert_eq!(refusal, unsafe_path("../old.txt"));
    // And an ordinary nested path passes it.
    assert!(UndoPlan::from_diff(repo(), &diff(vec![plain("dir/sub/d.txt", "modified")])).is_ok());
}

/// **The path check runs in the preview**, before any token exists.
#[test]
fn the_path_check_blocks_the_preview_with_no_token() {
    let backend = backend().with_diff_from_to(PARENT, BLOCK, diff(vec![plain("../x", "modified")]));
    assert_eq!(
        blocked_reason(preview_of(&backend, repo())),
        unsafe_path("../x")
    );
    wrote_nothing(&backend);
}

// ------------------------------------------------------------------ the card

/// **The card, in decision 5's order**: the two sentences, the block id and its messages, the plan
/// one line per path with its verb, then the consequence. Tier Three, and AUTHOR consumed.
#[test]
fn the_card_says_what_decision_5_asks_for() {
    let backend = backend().with_diff_from_to(
        PARENT,
        BLOCK,
        diff(vec![
            plain("a.txt", "modified"),
            added("c.txt"),
            entry("new.txt", "renamed", Some("old.txt"), Some(0o100644)),
        ]),
    );
    let (preview, token) = ready(preview_of(&backend, repo()));
    let summary = token.summary();

    assert_eq!(summary.operation, "Undo this block");
    assert_eq!(summary.preamble.as_deref(), Some(UNDO_LEAD));
    assert_eq!(
        summary.preamble.as_deref().unwrap(),
        "This adds a new change that puts back what this block changed. History keeps both."
    );
    assert_eq!(
        summary
            .target_ids
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        vec![BLOCK],
        "the block, and nothing else, under Targets"
    );
    assert_eq!(
        summary.asides.first().map(|aside| aside.body.clone()),
        Some(vec!["the mistake".to_string()]),
        "prikk's own patch messages, apart from the targets"
    );
    let plan = summary.undo_plan.as_ref().expect("the plan is on the card");
    assert_eq!(
        plan.steps
            .iter()
            .map(|step| (step.verb(), step.path.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("put back", "a.txt"),
            ("remove", "c.txt"),
            ("rename back", "new.txt"),
        ]
    );
    assert_eq!(summary.consequence, UNDO_CONSEQUENCE);
    assert_eq!(
        summary.message_note, None,
        "the message is not restated on the card: the prompt before it collected it, and \
         commit's own card does not restate one either"
    );
    assert_eq!(
        preview.message, "Undo: the mistake",
        "the message the undo will carry travels with the preview and its token"
    );
    assert_eq!(summary.capability, Some(Capability::Author));
    assert_eq!(token.tier(), Tier::Three);
    assert_eq!(token.reff(), "heads/main");
    assert!(
        summary.counts.is_empty(),
        "the plan is the list, not a count"
    );
    assert!(summary.materializes.is_none() && summary.merge_plan.is_none());
    assert_eq!(preview.summary(), *summary, "one summary, built once");
}

/// A block whose patches carry no message (sealed below prikk 0.32) has no aside, rather than an
/// empty one (RFC 015 F4).
#[test]
fn a_block_with_no_messages_shows_no_message_block() {
    let backend = backend().with_history(lineage(vec![
        row(BLOCK, "Normal", 1, &[]),
        row(PARENT, "Normal", 1, &[]),
    ]));
    let (_, token) = ready(preview_of(&backend, repo()));
    assert!(token.summary().asides.is_empty());
}

// ------------------------------------------------------------------ the gate

/// **The gate is commit's own** (§3.2): the undo ends in a commit, so it needs AUTHOR readiness —
/// and nothing runs without it.
#[test]
fn the_gate_asks_for_author_readiness_as_commit_does() {
    let scratch = Worktree::new("gate");
    scratch.write("a.txt", b"the block's bytes");
    let backend = executing();
    let (_, token) = ready(preview_of(&backend, scratch.path()));
    let viewer = Readiness {
        author: RoleReadiness::NotReady,
        maintainer: RoleReadiness::NotReady,
        read_only: false,
    };
    let error = undo_confirm_and_execute(
        &backend,
        scratch.path(),
        *token,
        viewer,
        Evidence::ExplicitYes,
    )
    .expect_err("an undo without AUTHOR readiness is not run");
    assert!(matches!(error, StikkError::NotReady { .. }), "{error:?}");
    wrote_nothing(&backend);

    let (_, token) = ready(preview_of(&backend, scratch.path()));
    let read_only = Readiness {
        read_only: true,
        ..author()
    };
    let error = undo_confirm_and_execute(
        &backend,
        scratch.path(),
        *token,
        read_only,
        Evidence::ExplicitYes,
    )
    .expect_err("read-only mode runs no undo");
    assert!(matches!(error, StikkError::NotReady { .. }), "{error:?}");
    wrote_nothing(&backend);
}

// ------------------------------------------------------------------ execution

/// A backend that answers every call an undo makes: the fetch writes the file it was asked for,
/// the rename succeeds, and the commit returns prikk's result.
fn executing() -> NullBackend {
    backend()
        .with_save_file_writing()
        .with_rename_path(vec!["moved new.txt -> old.txt".to_string()])
        .with_commit(commit_result())
}

/// The measured block, inverted: a modified file, a deleted file, an added file, a nested folder
/// the block deleted, a mode, and a rename.
fn measured_entries() -> Vec<DiffEntry> {
    vec![
        plain("a.txt", "modified"),
        plain("b.txt", "deleted"),
        added("c.txt"),
        plain("dir/sub/d.txt", "deleted"),
        entry("run.sh", "mode", None, Some(0o100755)),
        entry("new.txt", "renamed", Some("old.txt"), Some(0o100644)),
    ]
}

/// **Decision 6's order, from the seam's own record**: every byte is fetched before the worktree is
/// touched at all, then the renames, then the removals, then the copies, and the commit last.
#[test]
fn the_order_is_fetch_everything_then_rename_then_remove_then_copy_then_commit() {
    let scratch = Worktree::new("order");
    scratch.write("a.txt", b"the block's a.txt");
    scratch.write("c.txt", b"the file the block added");
    scratch.write("run.sh", b"#!/bin/sh\n");
    scratch.write("new.txt", b"the renamed file");
    let backend = executing().with_diff_from_to(PARENT, BLOCK, diff(measured_entries()));

    let (_, token) = ready(preview_of(&backend, scratch.path()));
    let outcome = undo_confirm_and_execute(
        &backend,
        scratch.path(),
        *token,
        author(),
        Evidence::ExplicitYes,
    )
    .expect("the undo runs");

    assert_eq!(outcome.operation, UNDO_OPERATION);
    match outcome.result {
        UndoResult::Done(result) => assert_eq!(result.patch_id, "c".repeat(64)),
        UndoResult::Stopped(stopped) => panic!("expected a commit: {stopped:?}"),
    }
    assert_eq!(
        backend.calls(),
        vec![
            // Step 1: every earlier file, from the parent, before anything is written.
            "save-file a.txt".to_string(),
            "save-file b.txt".to_string(),
            "save-file dir/sub/d.txt".to_string(),
            "save-file run.sh".to_string(),
            "save-file old.txt".to_string(),
            // Step 2: the rename first.
            "rename-path new.txt -> old.txt".to_string(),
            // Step 4, after the removals and the copies, which are stikk's own writes.
            "commit heads/main".to_string(),
        ],
        "the order is the safety argument"
    );
    assert_eq!(
        backend.rename_path_calls(),
        vec![("new.txt".to_string(), "old.txt".to_string())]
    );

    // The worktree: the parent's bytes are back (the scripted fetch writes the path it was asked
    // for), the added file is gone, and the directory the block deleted is back.
    assert_eq!(scratch.read("a.txt"), b"a.txt");
    assert_eq!(scratch.read("b.txt"), b"b.txt");
    assert_eq!(scratch.read("dir/sub/d.txt"), b"dir/sub/d.txt");
    assert_eq!(scratch.read("old.txt"), b"old.txt");
    assert!(!scratch.exists("c.txt"), "the file the block added is gone");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(scratch.path().join("run.sh"))
            .expect("the file")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755, "the parent's own mode, from from.mode");
        let plain_mode = std::fs::metadata(scratch.path().join("a.txt"))
            .expect("the file")
            .permissions()
            .mode();
        assert_eq!(plain_mode & 0o777, 0o644);
    }
}

/// **A failure in step 1 leaves the worktree byte-identical** (§7 trap 1), and travels as itself:
/// nothing had been written, so there is nothing to report but prikk's own refusal.
#[test]
fn a_failure_fetching_leaves_the_worktree_byte_identical() {
    let scratch = Worktree::new("fetch-fails");
    scratch.write("a.txt", b"the block's a.txt");
    scratch.write("c.txt", b"the file the block added");
    scratch.write("new.txt", b"the renamed file");
    let before = scratch.snapshot();
    let backend = executing()
        .with_diff_from_to(PARENT, BLOCK, diff(measured_entries()))
        .with_save_file_refusal("refusing: no such path at that block");

    let (_, token) = ready(preview_of(&backend, scratch.path()));
    let error = undo_confirm_and_execute(
        &backend,
        scratch.path(),
        *token,
        author(),
        Evidence::ExplicitYes,
    )
    .expect_err("a fetch that failed is an error, not a stopped undo");
    match &error {
        StikkError::Refusal { message } => {
            assert_eq!(message, "refusing: no such path at that block");
        }
        other => panic!("prikk's refusal, verbatim: {other:?}"),
    }
    assert_eq!(scratch.snapshot(), before, "not one byte of the worktree");
    assert!(
        backend.rename_path_calls().is_empty() && backend.commit_calls() == 0,
        "nothing was applied and nothing was committed"
    );
}

/// **A failure in step 2 stops at once**, commits nothing, and names exactly the paths it had
/// already changed — here the rename's two names, with the removal that follows failing because
/// the file is not there.
#[test]
fn a_failure_in_step_2_names_exactly_the_paths_it_changed() {
    let scratch = Worktree::new("apply-fails");
    scratch.write("a.txt", b"the block's a.txt");
    scratch.write("new.txt", b"the renamed file");
    // `c.txt` is absent, so the removal fails — the shape a worktree that moved under the
    // operation takes.
    let backend = executing().with_diff_from_to(
        PARENT,
        BLOCK,
        diff(vec![
            plain("a.txt", "modified"),
            added("c.txt"),
            entry("new.txt", "renamed", Some("old.txt"), Some(0o100644)),
        ]),
    );

    let (_, token) = ready(preview_of(&backend, scratch.path()));
    let outcome = undo_confirm_and_execute(
        &backend,
        scratch.path(),
        *token,
        author(),
        Evidence::ExplicitYes,
    )
    .expect("a stopped undo is a result, not an error");
    let stopped = match outcome.result {
        UndoResult::Stopped(stopped) => stopped,
        UndoResult::Done(result) => panic!("expected a stop, got {result:?}"),
    };
    assert_eq!(
        stopped.changed,
        vec!["new.txt".to_string(), "old.txt".to_string()],
        "the rename had run; nothing else had"
    );
    assert!(
        stopped.cause.to_string().contains("`c.txt`"),
        "stikk's own line names the path: {}",
        stopped.cause
    );
    assert_eq!(backend.commit_calls(), 0, "nothing was committed");
}

/// **A failure in step 4 stops at once too**, with prikk's own refusal carried verbatim and every
/// path the undo changed named.
#[test]
fn a_failure_committing_names_every_path_changed_and_carries_prikks_words() {
    let scratch = Worktree::new("commit-fails");
    scratch.write("a.txt", b"the block's a.txt");
    scratch.write("c.txt", b"the file the block added");
    let backend = executing()
        .with_diff_from_to(
            PARENT,
            BLOCK,
            diff(vec![plain("a.txt", "modified"), added("c.txt")]),
        )
        .with_commit_refusal("refusing: nothing to commit");

    let (_, token) = ready(preview_of(&backend, scratch.path()));
    let outcome = undo_confirm_and_execute(
        &backend,
        scratch.path(),
        *token,
        author(),
        Evidence::ExplicitYes,
    )
    .expect("a stopped undo is a result, not an error");
    let stopped = match outcome.result {
        UndoResult::Stopped(stopped) => stopped,
        UndoResult::Done(result) => panic!("expected a stop, got {result:?}"),
    };
    assert_eq!(
        stopped.changed,
        vec!["c.txt".to_string(), "a.txt".to_string()],
        "the removal, then the copy — in the order they happened"
    );
    match &stopped.cause {
        StikkError::Refusal { message } => assert_eq!(message, "refusing: nothing to commit"),
        other => panic!("prikk's words, verbatim: {other:?}"),
    }
    assert_eq!(backend.commit_calls(), 1, "asked once, and never retried");
}

/// **§3.4: only the directories this undo emptied.** The one the removed file was in goes, and so
/// does its parent, which the removal left empty too — and an unrelated empty directory stays.
#[test]
fn only_the_directories_the_undo_emptied_are_removed() {
    let scratch = Worktree::new("dirs");
    scratch.write("dir/sub/d.txt", b"the file the block added");
    scratch.write("dir/kept/k.txt", b"not this undo's business");
    std::fs::create_dir_all(scratch.path().join("empty-already")).expect("an empty directory");
    let backend = executing().with_diff_from_to(PARENT, BLOCK, diff(vec![added("dir/sub/d.txt")]));

    let (_, token) = ready(preview_of(&backend, scratch.path()));
    undo_confirm_and_execute(
        &backend,
        scratch.path(),
        *token,
        author(),
        Evidence::ExplicitYes,
    )
    .expect("the undo runs");

    assert!(
        !scratch.exists("dir/sub"),
        "the directory it emptied is gone"
    );
    assert!(
        scratch.exists("dir/kept/k.txt") && scratch.exists("dir"),
        "a directory with something else in it stays, and so does its parent"
    );
    assert!(
        scratch.exists("empty-already"),
        "an empty directory this undo did not empty is left alone"
    );
}

/// **The scratch directory is outside the repository, and gone afterwards** (decision 6).
#[test]
fn the_fetch_directory_is_outside_the_repository_and_removed() {
    let scratch = Worktree::new("temp");
    let fetch = super::Scratch::fresh(scratch.path()).expect("a fresh directory");
    let dir = fetch.dir.clone();
    assert!(dir.is_dir(), "created, by this process");
    assert!(
        !dir.starts_with(scratch.path()),
        "outside the repository: {}",
        dir.display()
    );
    assert!(
        std::fs::create_dir(&dir).is_err(),
        "it was created fresh, not adopted"
    );
    drop(fetch);
    assert!(!dir.exists(), "deleted whatever happened");
}
