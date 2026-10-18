//! Tests for the session store (RFC 046).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use super::*;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "stikk-session-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the scratch dir");
    dir
}

fn record(root: &Path, focus: Option<&str>) -> SessionRecord {
    SessionRecord {
        root: root.to_path_buf(),
        focus: focus.map(ToString::to_string),
        view: LastView::Changes,
        hide_untracked: true,
    }
}

/// **The file name is pinned**, because it is on disk: a toolchain upgrade must not rename every
/// record and silently lose every session, which is what `DefaultHasher` would have done.
#[test]
fn the_hash_is_the_one_written_down() {
    assert_eq!(
        fnv1a64(b""),
        0xcbf2_9ce4_8422_2325,
        "the offset basis alone"
    );
    assert_eq!(format!("{:016x}", fnv1a64(b"/repo")), "f7556aa5a4543342");
    assert_eq!(
        format!("{:016x}", fnv1a64(b"/home/u/p")),
        "7ca9541b13084e72"
    );
    assert_eq!(file_name(Path::new("/repo")), "f7556aa5a4543342");
    assert_eq!(file_name(Path::new("/home/u/p")), "7ca9541b13084e72");
    assert_eq!(file_name(Path::new("/repo")).len(), 16);
}

/// **Every ref name prikk accepts round-trips** (RFC 046 decision 2). The hostile ones are the
/// point: a space (prikk accepts `heads/bad name`, RFC 044 F2), non-ASCII, an `=` that could end
/// a key, and text that *looks* like this module's own escapes.
#[test]
fn a_hostile_ref_name_comes_back_exactly() {
    let dir = scratch("hostile");
    let root = dir.join("repo");
    for name in [
        "heads/bad name",
        "heads/ゆき",
        "heads/a=b",
        "heads/back\\slash",
        "heads/looks\\nlike-a-newline",
        "heads/looks\\\\nlike-an-escaped-one",
        "heads/trailing ",
        "heads/\u{202e}evil",
        "tags/v1.0.0+build",
    ] {
        let original = record(&root, Some(name));
        save(&dir, &root, &original).expect("save");
        assert_eq!(
            load(&dir, &root).expect("a record"),
            original,
            "{name:?} did not round-trip"
        );
    }
    // And a real newline or carriage return, which the escape exists for — prikk would not accept
    // one in a ref name, but the encoding must not be the thing that decides that.
    for name in ["heads/with\nnewline", "heads/with\rreturn"] {
        let original = record(&root, Some(name));
        save(&dir, &root, &original).expect("save");
        assert_eq!(load(&dir, &root).expect("a record"), original, "{name:?}");
    }
}

/// Every field round-trips, including the absence of a focus.
#[test]
fn every_field_round_trips() {
    let dir = scratch("fields");
    let root = dir.join("repo");
    for view in [
        LastView::Orientation,
        LastView::History,
        LastView::Changes,
        LastView::Queue,
    ] {
        for hide_untracked in [true, false] {
            for focus in [None, Some("heads/main")] {
                let original = SessionRecord {
                    root: root.clone(),
                    focus: focus.map(ToString::to_string),
                    view,
                    hide_untracked,
                };
                save(&dir, &root, &original).expect("save");
                assert_eq!(load(&dir, &root).expect("a record"), original);
            }
        }
    }
}

/// **Anything stikk cannot decode is `None`**, whole — never partly applied, never an error.
#[test]
fn a_record_that_is_not_ours_is_ignored_whole() {
    let dir = scratch("broken");
    let root = dir.join("repo");
    let good = record(&root, Some("heads/main"));
    save(&dir, &root, &good).expect("save");
    let target = dir.join("sessions").join(file_name(&root));
    let whole = std::fs::read(&target).expect("read it back");

    let path_line = format!("path={}\n", root.display());
    for (case, bytes) in [
        (
            "truncated mid-line",
            whole.get(..whole.len() / 2).expect("a prefix").to_vec(),
        ),
        ("garbage", b"\x00\xff not a record at all".to_vec()),
        ("empty", Vec::new()),
        (
            "a future version",
            format!("version=2\n{path_line}view=history\nhide-untracked=no\n").into_bytes(),
        ),
        (
            "no version line",
            format!("{path_line}view=history\nhide-untracked=no\n").into_bytes(),
        ),
        (
            "no path line",
            b"version=1\nview=history\nhide-untracked=no\n".to_vec(),
        ),
        (
            "another repository's path",
            b"version=1\npath=/somewhere/else\nview=history\nhide-untracked=no\n".to_vec(),
        ),
        (
            "a view this stikk does not have",
            format!("version=1\n{path_line}view=verify\nhide-untracked=no\n").into_bytes(),
        ),
        (
            "a key this stikk does not know",
            format!("version=1\n{path_line}view=history\nhide-untracked=no\nanchor=abc\n")
                .into_bytes(),
        ),
        (
            "no view line",
            format!("version=1\n{path_line}hide-untracked=no\n").into_bytes(),
        ),
        (
            "no untracked line",
            format!("version=1\n{path_line}view=history\n").into_bytes(),
        ),
        (
            "a value with an escape we never write",
            format!("version=1\n{path_line}focus=heads/\\q\nview=history\nhide-untracked=no\n")
                .into_bytes(),
        ),
        (
            "a line with no =",
            format!("version=1\n{path_line}view=history\nhide-untracked=no\nbare\n").into_bytes(),
        ),
    ] {
        std::fs::write(&target, &bytes).expect("plant it");
        assert_eq!(load(&dir, &root), None, "{case} should be ignored whole");
    }

    // The control: the real record still loads, so the cases above fail for their own reason.
    std::fs::write(&target, &whole).expect("restore");
    assert_eq!(load(&dir, &root), Some(good));
}

/// A missing file is simply no record.
#[test]
fn a_missing_record_is_none() {
    let dir = scratch("missing");
    assert_eq!(load(&dir, &dir.join("never-saved")), None);
}

/// **A hash collision is a miss, not another repository's state.** The two roots here are given
/// the same file name on purpose, which is the only way to reach the case deliberately.
#[test]
fn a_collision_reads_as_no_record() {
    let dir = scratch("collision");
    let mine = dir.join("mine");
    let theirs = dir.join("theirs");
    save(&dir, &mine, &record(&mine, Some("heads/mine"))).expect("save");
    // Put my record where theirs would live. Its `path` line still names my root.
    let planted = dir.join("sessions").join(file_name(&theirs));
    std::fs::copy(dir.join("sessions").join(file_name(&mine)), &planted).expect("copy");
    assert_eq!(load(&dir, &theirs), None, "a foreign path is not read");
    assert!(load(&dir, &mine).is_some(), "and mine is untouched");
}

/// **`C-E2`, with the open repository's root** — this module is its first real caller. A state
/// directory inside the repository is refused, and **nothing is written**.
#[test]
fn a_state_dir_inside_the_repository_is_refused_and_writes_nothing() {
    let repo = scratch("ce2");
    for inside in [repo.join(".stikk"), repo.join(".prikk").join("stikk")] {
        let error = save(&inside, &repo, &record(&repo, Some("heads/main")))
            .expect_err("C-E2 refuses a target inside the repository");
        assert_eq!(error.class(), "stikk-internal", "{error}");
        assert!(error.to_string().contains("refusing to write"), "{error}");
        assert!(
            !inside.exists(),
            "{} was created despite the refusal",
            inside.display()
        );
    }
    // The repository holds nothing of stikk's: the refusal came before any directory was made.
    let left: Vec<String> = std::fs::read_dir(&repo)
        .expect("read the repository")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert!(left.is_empty(), "the repository is not empty: {left:?}");
}

/// The replace is atomic and leaves nothing behind.
#[test]
fn a_save_leaves_no_temporary_file() {
    let dir = scratch("atomic");
    let root = dir.join("repo");
    save(&dir, &root, &record(&root, Some("heads/one"))).expect("save");
    save(&dir, &root, &record(&root, Some("heads/two"))).expect("overwrite");
    let names: Vec<String> = std::fs::read_dir(dir.join("sessions"))
        .expect("read sessions/")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        vec![file_name(&root)],
        "one file, no .tmp: {names:?}"
    );
    assert_eq!(
        load(&dir, &root).expect("a record").focus.as_deref(),
        Some("heads/two"),
        "and it is the newer one"
    );
}

/// **The bound holds, oldest first, and never the record just written.**
#[test]
fn the_bound_prunes_the_oldest() {
    let dir = scratch("bound");
    let sessions = dir.join("sessions");
    std::fs::create_dir_all(&sessions).expect("create sessions/");
    // `MAX_RECORDS` plausible records, aged so the order is unambiguous: the file name is hex, so
    // these are what `is_record_name` accepts and `prune` therefore counts.
    for i in 0..MAX_RECORDS {
        let path = sessions.join(format!("{i:016x}"));
        std::fs::write(&path, b"version=1\n").expect("plant");
        let when =
            std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000 + i as u64);
        set_modified(&path, when);
    }
    // A temporary file must not count against the bound.
    std::fs::write(sessions.join("deadbeefdeadbeef.99.tmp"), b"x").expect("plant a tmp");

    let root = dir.join("repo");
    save(&dir, &root, &record(&root, Some("heads/main"))).expect("save");

    let records: Vec<String> = std::fs::read_dir(&sessions)
        .expect("read sessions/")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .filter(|name| name.len() == 16)
        .collect();
    assert_eq!(
        records.len(),
        MAX_RECORDS,
        "the bound holds: {}",
        records.len()
    );
    assert!(
        records.contains(&file_name(&root)),
        "and never prunes the one just written"
    );
    assert!(
        !records.contains(&format!("{:016x}", 0)),
        "the oldest went first"
    );
    assert!(
        records.contains(&format!("{:016x}", MAX_RECORDS - 1)),
        "and the newest stayed"
    );
}

#[cfg(unix)]
fn set_modified(path: &Path, when: std::time::SystemTime) {
    let file = std::fs::File::options()
        .write(true)
        .open(path)
        .expect("open to set the time");
    file.set_modified(when).expect("set the modification time");
}

#[cfg(not(unix))]
fn set_modified(path: &Path, when: std::time::SystemTime) {
    let file = std::fs::File::options()
        .write(true)
        .open(path)
        .expect("open to set the time");
    file.set_modified(when).expect("set the modification time");
}

/// **The file holds the record's fields and nothing else** (`INV-5`, `INV-9`): no content, no
/// object id, no prose.
#[test]
fn the_file_holds_only_the_records_fields() {
    let dir = scratch("fields-only");
    let root = dir.join("repo");
    save(&dir, &root, &record(&root, Some("heads/main"))).expect("save");
    let text = std::fs::read_to_string(dir.join("sessions").join(file_name(&root)))
        .expect("the record is UTF-8 here, because this root and this ref name are");
    let mut keys: Vec<&str> = text
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.split_once('=').expect("key=value").0)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["focus", "hide-untracked", "path", "version", "view"]);
    assert_eq!(
        text,
        format!(
            "version=1\npath={}\nfocus=heads/main\nview=changes\nhide-untracked=yes\n",
            root.display()
        ),
        "the whole file, exactly"
    );
}
