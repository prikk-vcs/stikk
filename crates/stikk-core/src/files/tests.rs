//! Tests for getting files back (RFC 053).
//!
//! The reports below are prikk 0.54.1's own, from
//! `.git-exclude/reports/053-get-files-back/measure053-0.54.txt`: `a.txt` at the first block is
//! `mode 33188, size 4`, `run.sh` is `33261`, and the plan for that block is four files and
//! thirty bytes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use stikk_prikk::{ExtractReport, FileMeta, NullBackend};

use super::*;

const BLOCK: &str = "f687c8ff7394b57414d88b191700eb67b4a15b5598cb11fb1bd848f0c1746243";

fn meta(mode: u32, size: u64) -> FileMeta {
    FileMeta {
        kind: "file".to_string(),
        encoding: "text".to_string(),
        mode,
        size,
    }
}

fn plan(provisional: bool) -> ExtractReport {
    ExtractReport {
        target_block_id: BLOCK.to_string(),
        provisional,
        written: false,
        file_count: 4,
        total_bytes: 30,
        largest_file_bytes: 18,
    }
}

/// A backend at the ceiling that answers both reads and both writes.
fn saving(mode: u32, provisional: bool) -> NullBackend {
    NullBackend::supported()
        .with_version(0, 55, 0)
        .with_file_meta(meta(mode, 4))
        .with_save_file()
        .with_extract_plan(plan(provisional))
        .with_extract(ExtractReport {
            written: true,
            ..plan(provisional)
        })
}

/// A scratch directory that stands in for a repository, removed on drop.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "stikk-files-{tag}-{}-{:?}",
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
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ---------------------------------------------------------------- the two destination rules

/// **Inside the repository is declined, and nothing is spawned** — for `F` and for `A`
/// (RFC 053 §7 trap 5). prikk 0.54 would write there, and 0.55 refuses it only for `extract`.
#[test]
fn a_destination_inside_the_repository_is_declined_without_spawning_prikk() {
    let scratch = Scratch::new("inside");
    let repo = scratch.path();
    let backend = saving(0o100644, false);

    for inside in [repo.join("a-old.txt"), repo.join("deeper/a-old.txt")] {
        let reason = match save_file_preview(&backend, repo, BLOCK, "a.txt", &inside) {
            Ok(SaveFilePreview::Blocked(reason)) => reason,
            other => panic!("{}: expected a block, got {other:?}", inside.display()),
        };
        assert_eq!(reason, INSIDE_THE_REPOSITORY, "{}", inside.display());

        let reason = match save_all_preview(&backend, repo, BLOCK, &inside) {
            Ok(SaveAllPreview::Blocked(reason)) => reason,
            other => panic!("{}: expected a block, got {other:?}", inside.display()),
        };
        assert_eq!(reason, INSIDE_THE_REPOSITORY, "{}", inside.display());
    }
    assert_eq!(backend.save_file_calls(), 0, "no write was spawned");
    assert_eq!(backend.extract_calls(), 0, "no write was spawned");
    assert_eq!(
        backend.extract_plan_calls(),
        0,
        "not even the plan: the rule is decided before prikk is asked anything"
    );
}

/// **An existing path is declined, and nothing is spawned** — a file for `F`, a directory for
/// `A`. prikk's `--plan-only` does not check this at all (F6).
#[test]
fn an_existing_destination_is_declined_without_spawning_prikk() {
    let scratch = Scratch::new("exists");
    let outside = scratch.path().parent().expect("a parent").join(format!(
        "stikk-files-taken-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&outside).expect("a destination that is taken");
    let taken_file = outside.join("a-old.txt");
    std::fs::write(&taken_file, b"not mine to overwrite").expect("a file that is taken");
    let backend = saving(0o100644, false);
    let repo = scratch.path();

    match save_file_preview(&backend, repo, BLOCK, "a.txt", &taken_file) {
        Ok(SaveFilePreview::Blocked(reason)) => assert_eq!(reason, DESTINATION_EXISTS),
        other => panic!("expected a block, got {other:?}"),
    }
    match save_all_preview(&backend, repo, BLOCK, &outside) {
        Ok(SaveAllPreview::Blocked(reason)) => assert_eq!(reason, DESTINATION_EXISTS),
        other => panic!("expected a block, got {other:?}"),
    }
    assert_eq!(backend.save_file_calls(), 0);
    assert_eq!(backend.extract_calls(), 0);
    assert_eq!(backend.extract_plan_calls(), 0);
    // And the file that was there is untouched.
    assert_eq!(
        std::fs::read(&taken_file).expect("still there"),
        b"not mine to overwrite"
    );
    let _ = std::fs::remove_dir_all(&outside);
}

// ---------------------------------------------------------------- the suggestions

/// The four shapes decision 4 names, each outside the repository by construction.
#[test]
fn the_suggested_file_destination_keeps_the_extension_and_drops_the_directories() {
    let repo = Path::new("/home/someone/work/repo");
    for (path, wanted) in [
        ("a.txt", "/home/someone/work/a-f687c8ff7394.txt"),
        ("dir/sub/d.txt", "/home/someone/work/d-f687c8ff7394.txt"),
        ("run", "/home/someone/work/run-f687c8ff7394"),
        (".env", "/home/someone/work/.env-f687c8ff7394"),
        ("a.tar.gz", "/home/someone/work/a.tar-f687c8ff7394.gz"),
    ] {
        assert_eq!(
            suggested_file_destination(repo, path, BLOCK),
            Path::new(wanted),
            "{path}"
        );
    }
}

#[test]
fn the_suggested_folder_is_the_repositorys_name_beside_it() {
    assert_eq!(
        suggested_all_destination(Path::new("/home/someone/work/repo"), BLOCK),
        Path::new("/home/someone/work/repo-f687c8ff7394")
    );
}

/// **Outside the repository by construction** — the suggestion never meets stikk's own first
/// rule, which is the point of suggesting one at all.
#[test]
fn neither_suggestion_is_inside_the_repository() {
    let scratch = Scratch::new("suggest");
    let repo = scratch.path();
    for destination in [
        suggested_file_destination(repo, "a.txt", BLOCK),
        suggested_all_destination(repo, BLOCK),
    ] {
        assert_eq!(
            destination_refused(repo, &destination),
            None,
            "{}",
            destination.display()
        );
    }
}

// ---------------------------------------------------------------- the cards

fn file_summary(backend: &NullBackend, repo: &Path, out: &Path) -> ConfirmationSummary {
    match save_file_preview(backend, repo, BLOCK, "a.txt", out) {
        Ok(SaveFilePreview::Ready { token }) => token.summary().clone(),
        other => panic!("expected a card, got {other:?}"),
    }
}

fn all_summary(backend: &NullBackend, repo: &Path, dir: &Path) -> ConfirmationSummary {
    match save_all_preview(backend, repo, BLOCK, dir) {
        Ok(SaveAllPreview::Ready { token }) => token.summary().clone(),
        other => panic!("expected a card, got {other:?}"),
    }
}

/// **The execute notice turns on the mode bit, and on nothing else** (F3).
#[test]
fn the_execute_notice_is_on_an_executable_file_and_not_on_an_ordinary_one() {
    let scratch = Scratch::new("exec");
    let repo = scratch.path();
    let out = suggested_file_destination(repo, "a.txt", BLOCK);

    let summary = file_summary(&saving(0o100755, false), repo, &out);
    assert_eq!(summary.notices, vec![LOSES_THE_EXECUTE_BIT]);

    let summary = file_summary(&saving(0o100644, false), repo, &out);
    assert!(
        summary.notices.is_empty(),
        "an ordinary file loses nothing, so there is nothing to say"
    );
}

/// **The provisional notice turns on prikk's own flag, and on nothing else** (F4).
#[test]
fn the_provisional_notice_is_on_a_provisional_plan_only() {
    let scratch = Scratch::new("prov");
    let repo = scratch.path();
    let dir = suggested_all_destination(repo, BLOCK);

    let summary = all_summary(&saving(0o100644, true), repo, &dir);
    assert_eq!(summary.notices, vec![PROVISIONAL_REPLAY]);

    let summary = all_summary(&saving(0o100644, false), repo, &dir);
    assert!(summary.notices.is_empty());
}

/// Both cards name what they act on, and say the same consequence in stikk's words — **naming
/// nothing**, as RFC 044 C1 requires.
#[test]
fn both_cards_name_their_targets_and_neither_consequence_names_anything() {
    let scratch = Scratch::new("cards");
    let repo = scratch.path();
    let out = suggested_file_destination(repo, "a.txt", BLOCK);
    let dir = suggested_all_destination(repo, BLOCK);
    let backend = saving(0o100644, false);

    let one = file_summary(&backend, repo, &out);
    assert_eq!(one.operation, "Save file");
    let labels: Vec<&str> = one
        .target_ids
        .iter()
        .map(|t| t.role.unwrap_or(""))
        .collect();
    assert_eq!(labels, vec!["block", "file", "size", "to"]);
    assert_eq!(one.target_ids[0].id, "f687c8ff7394");
    assert_eq!(one.target_ids[1].id, "a.txt");
    assert_eq!(one.target_ids[2].id, "4 bytes");
    assert_eq!(one.target_ids[3].id, out.display().to_string());

    let all = all_summary(&backend, repo, &dir);
    assert_eq!(all.operation, "Save files");
    let labels: Vec<&str> = all
        .target_ids
        .iter()
        .map(|t| t.role.unwrap_or(""))
        .collect();
    assert_eq!(labels, vec!["block", "files", "bytes", "to"]);
    assert_eq!(all.target_ids[1].id, "4");
    assert_eq!(all.target_ids[2].id, "30");

    for summary in [&one, &all] {
        assert_eq!(summary.consequence, COPY_CONSEQUENCE);
        assert!(
            !summary.consequence.contains("a.txt")
                && !summary.consequence.contains(&dir.display().to_string()),
            "the consequence names nothing (RFC 044 C1): {}",
            summary.consequence
        );
        // **No key, on either** (decision 2).
        assert!(summary.signing_key_id.is_none());
        assert!(summary.capability.is_none());
    }
}

// ---------------------------------------------------------------- the band, and read-only

/// **Below the band both answer in stikk's own voice, and nothing is spawned** (decision 7).
#[test]
fn below_the_band_both_decline_and_spawn_nothing() {
    let scratch = Scratch::new("band");
    let repo = scratch.path();
    let backend = NullBackend::supported().with_version(0, 53, 0);
    let out = suggested_file_destination(repo, "a.txt", BLOCK);
    let dir = suggested_all_destination(repo, BLOCK);

    match save_file_preview(&backend, repo, BLOCK, "a.txt", &out) {
        Ok(SaveFilePreview::Blocked(reason)) => {
            assert_eq!(reason, files_unavailable(53));
            assert!(reason.contains("0.54"), "the band is named: {reason}");
        }
        other => panic!("expected a block, got {other:?}"),
    }
    match save_all_preview(&backend, repo, BLOCK, &dir) {
        Ok(SaveAllPreview::Blocked(reason)) => assert_eq!(reason, files_unavailable(53)),
        other => panic!("expected a block, got {other:?}"),
    }
    assert_eq!(backend.save_file_calls(), 0);
    assert_eq!(backend.extract_calls(), 0);
}

/// **Tier Two, and no key** — the Exchange category's own answer, which is why this reuses it
/// rather than adding a category (decision 2).
#[test]
fn saving_is_tier_two_and_needs_no_key() {
    use stikk_model::{RequestCategory, SigningRequirement, Tier};
    assert_eq!(RequestCategory::Exchange.tier(), Tier::Two);
    assert_eq!(
        RequestCategory::Exchange.signing_requirement(),
        // **`ReadWrite` is "no signing key"**: a human at the machine, not in read-only mode.
        // There is no `None` in this enum — the floor is operating at all.
        SigningRequirement::ReadWrite
    );
}

/// **Read-only declines both**, as it declines export (decision 2) — the gate is
/// `capability_gate`'s, keyed on the Exchange category, so the two saves cannot drift from it.
#[test]
fn read_only_declines_both_saves() {
    use stikk_model::{Readiness, RequestCategory};
    let read_only = Readiness {
        read_only: true,
        ..Readiness::none()
    };
    assert!(!read_only.may_operate(), "the premise");
    for operation in [SAVE_FILE_OPERATION, SAVE_ALL_OPERATION] {
        let err = confirm::capability_gate(operation, RequestCategory::Exchange, read_only)
            .expect_err("read-only declines a write");
        assert!(err.to_string().contains("read-only"), "{operation}: {err}");
    }
}

// ---------------------------------------------------------------- the result sentences

#[test]
fn the_result_sentences_say_what_was_written_and_where() {
    let target = SaveFileTarget {
        block: BLOCK.to_string(),
        path: "dir/sub/d.txt".to_string(),
        output: Path::new("/home/someone/work/d-f687c8ff7394.txt").to_path_buf(),
    };
    assert_eq!(
        saved_file_line(&target),
        "Saved dir/sub/d.txt as it was at f687c8ff7394 to /home/someone/work/d-f687c8ff7394.txt."
    );

    let target = SaveAllTarget {
        block: BLOCK.to_string(),
        dir: Path::new("/home/someone/work/repo-f687c8ff7394").to_path_buf(),
    };
    let report = ExtractReport {
        written: true,
        ..plan(false)
    };
    assert_eq!(
        saved_all_lines(&target, &report),
        vec!["Wrote 4 files (30 bytes) to /home/someone/work/repo-f687c8ff7394.".to_string()]
    );

    // **The provisional notice is repeated from the report**, not from the plan the reader
    // confirmed on: the run is what actually replayed.
    let report = ExtractReport {
        written: true,
        ..plan(true)
    };
    let lines = saved_all_lines(&target, &report);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1], PROVISIONAL_REPLAY);
}

/// prikk's refusal of a path that is not at that block reaches the reader **verbatim**.
#[test]
fn prikks_refusal_of_a_missing_path_is_carried_word_for_word() {
    let scratch = Scratch::new("missing");
    let repo = scratch.path();
    let out = suggested_file_destination(repo, "d.txt", BLOCK);
    let refusal = format!("precondition not met: path dir/sub/d.txt does not exist at {BLOCK}");
    let backend = NullBackend::supported()
        .with_version(0, 55, 0)
        .with_file_meta_refusal(refusal.clone());

    let err = save_file_preview(&backend, repo, BLOCK, "dir/sub/d.txt", &out)
        .expect_err("prikk refused the read");
    let stikk_model::StikkError::Refusal { message } = err else {
        panic!("prikk's refusal must stay a refusal");
    };
    assert_eq!(message, refusal);
}
