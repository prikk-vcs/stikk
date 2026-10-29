//! Tests for the worker loop (design CC-01; RFC 010). No thread is spawned here — `run` is a plain
//! function, so calling it directly on the test thread proves the state machine without proving the
//! `std::thread` plumbing, which does not need proving (handoff §8).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;
use std::sync::mpsc;

use stikk_prikk::NullBackend;

use super::*;

#[test]
fn processes_one_request_and_replies_with_the_matching_seq() {
    let backend = NullBackend::supported();
    let (req_tx, req_rx) = mpsc::channel();
    let (res_tx, res_rx) = mpsc::channel();
    req_tx
        .send(Request {
            seq: 7,
            kind: RequestKind::Orient,
        })
        .expect("send succeeds");
    drop(req_tx); // closes the channel so `run` returns once the one request is drained

    run(&backend, Path::new("/repo"), req_rx, res_tx);

    let response = res_rx.try_recv().expect("a response was sent");
    assert_eq!(response.seq, 7);
    assert!(matches!(
        response.kind,
        ResponseKind::Orient(OrientRead {
            token: Ok(_),
            view: Ok(_)
        })
    ));
}

#[test]
fn processes_requests_in_order_and_stops_when_the_sender_is_dropped() {
    let backend = NullBackend::supported();
    let (req_tx, req_rx) = mpsc::channel();
    let (res_tx, res_rx) = mpsc::channel();
    req_tx
        .send(Request {
            seq: 1,
            kind: RequestKind::Orient,
        })
        .unwrap();
    req_tx
        .send(Request {
            seq: 2,
            kind: RequestKind::Refs,
        })
        .unwrap();
    drop(req_tx);

    run(&backend, Path::new("/repo"), req_rx, res_tx);

    let first = res_rx.try_recv().expect("first response");
    let second = res_rx.try_recv().expect("second response");
    assert_eq!(first.seq, 1);
    assert!(matches!(first.kind, ResponseKind::Orient(_)));
    assert_eq!(second.seq, 2);
    assert!(matches!(second.kind, ResponseKind::Refs(_)));
    assert!(res_rx.try_recv().is_err());
}

#[test]
fn a_refusal_is_carried_through_as_an_error_response() {
    let backend = NullBackend::supported().with_history_refusal("ref does not exist");
    let (req_tx, req_rx) = mpsc::channel();
    let (res_tx, res_rx) = mpsc::channel();
    req_tx
        .send(Request {
            seq: 0,
            kind: RequestKind::History {
                reff: "heads/main".into(),
            },
        })
        .unwrap();
    drop(req_tx);

    run(&backend, Path::new("/repo"), req_rx, res_tx);

    let response = res_rx.try_recv().expect("a response was sent");
    match response.kind {
        ResponseKind::History(Err(err)) => assert_eq!(err.class(), "refusal"),
        other => panic!("expected a History error, got {other:?}"),
    }
}

#[test]
fn stops_cleanly_when_the_response_receiver_is_gone() {
    // If the UI thread has already quit (dropped its `Receiver<Response>`), the worker's send fails
    // and it must stop rather than loop forever trying to deliver to nobody.
    let backend = NullBackend::supported();
    let (req_tx, req_rx) = mpsc::channel();
    let (res_tx, res_rx) = mpsc::channel();
    drop(res_rx);
    req_tx
        .send(Request {
            seq: 0,
            kind: RequestKind::Orient,
        })
        .unwrap();
    req_tx
        .send(Request {
            seq: 1,
            kind: RequestKind::Orient,
        })
        .unwrap();
    drop(req_tx);

    // Must return promptly rather than hang; a test timeout would catch a regression here.
    run(&backend, Path::new("/repo"), req_rx, res_tx);
}

/// Delegates to a `NullBackend`, recording the order the seam is called in.
struct Recording {
    inner: NullBackend,
    calls: std::sync::Mutex<Vec<&'static str>>,
}

impl Recording {
    fn record(&self, call: &'static str) {
        self.calls.lock().unwrap().push(call);
    }
}

impl Prikk for Recording {
    fn handshake(&self) -> Result<stikk_prikk::Handshake> {
        self.record("handshake");
        self.inner.handshake()
    }
    fn readiness(&self, repo: &Path) -> Result<stikk_prikk::ReadinessReport> {
        self.record("readiness");
        self.inner.readiness(repo)
    }
    fn orientation(&self, repo: &Path) -> Result<stikk_prikk::Orientation> {
        self.record("orientation");
        self.inner.orientation(repo)
    }
    fn history(&self, repo: &Path, reff: &str, limit: usize) -> Result<stikk_prikk::History> {
        self.record("history");
        self.inner.history(repo, reff, limit)
    }
    fn block_state(
        &self,
        repo: &Path,
        reff: &str,
        block_id: &str,
        is_tip: bool,
    ) -> Result<stikk_prikk::StateReport> {
        self.record("block_state");
        self.inner.block_state(repo, reff, block_id, is_tip)
    }
    fn verify(&self, repo: &Path) -> Result<stikk_prikk::VerifyReport> {
        self.record("verify");
        self.inner.verify(repo)
    }

    fn create_branch(
        &self,
        repo: &Path,
        name: &str,
        from_ref: &str,
    ) -> Result<stikk_prikk::RefWrite> {
        self.record("create_branch");
        self.inner.create_branch(repo, name, from_ref)
    }

    fn create_tag(
        &self,
        repo: &Path,
        name: &str,
        target: &str,
        message: &str,
    ) -> Result<stikk_prikk::RefWrite> {
        self.record("create_tag");
        self.inner.create_tag(repo, name, target, message)
    }

    fn close_branch(&self, repo: &Path, name: &str) -> Result<stikk_prikk::RefWrite> {
        self.record("close_branch");
        self.inner.close_branch(repo, name)
    }

    fn switch_branch(&self, repo: &Path, name: &str) -> Result<stikk_prikk::SwitchResult> {
        self.record("switch_branch");
        self.inner.switch_branch(repo, name)
    }

    fn merge_plan(
        &self,
        repo: &Path,
        baseline: &str,
        left: &str,
        right: &str,
    ) -> Result<stikk_prikk::MergePlan> {
        self.record("merge_plan");
        self.inner.merge_plan(repo, baseline, left, right)
    }

    fn bundle_verify(&self, path: &Path) -> Result<stikk_prikk::BundleVerify> {
        self.record("bundle_verify");
        self.inner.bundle_verify(path)
    }

    fn bundle_preview(
        &self,
        repo: &Path,
        path: &Path,
        local_ref: Option<&str>,
    ) -> Result<stikk_prikk::BundlePreview> {
        self.record("bundle_preview");
        self.inner.bundle_preview(repo, path, local_ref)
    }

    fn bundle_import(&self, repo: &Path, path: &Path) -> Result<stikk_prikk::BundleImport> {
        self.record("bundle_import");
        self.inner.bundle_import(repo, path)
    }

    fn file_meta(&self, repo: &Path, block: &str, path: &str) -> Result<stikk_prikk::FileMetaRead> {
        self.record("file_meta");
        self.inner.file_meta(repo, block, path)
    }

    fn save_file(
        &self,
        repo: &Path,
        block: &str,
        path: &str,
        output: &Path,
    ) -> Result<stikk_prikk::FileSaved> {
        self.record("save_file");
        self.inner.save_file(repo, block, path, output)
    }

    fn extract_plan(&self, repo: &Path, block: &str, dir: &Path) -> Result<stikk_prikk::Extracted> {
        self.record("extract_plan");
        self.inner.extract_plan(repo, block, dir)
    }

    fn extract(&self, repo: &Path, block: &str, dir: &Path) -> Result<stikk_prikk::Extracted> {
        self.record("extract");
        self.inner.extract(repo, block, dir)
    }

    fn bundle_export(
        &self,
        repo: &Path,
        ref_name: &str,
        output: &Path,
    ) -> Result<stikk_prikk::BundleExport> {
        self.record("bundle_export");
        self.inner.bundle_export(repo, ref_name, output)
    }

    fn merge(
        &self,
        repo: &Path,
        baseline: &str,
        into: &str,
        from: &str,
    ) -> Result<stikk_prikk::MergeResult> {
        self.record("merge");
        self.inner.merge(repo, baseline, into, from)
    }

    fn diff(
        &self,
        repo: &Path,
        comparison: &stikk_prikk::Comparison,
    ) -> Result<stikk_prikk::DiffReport> {
        self.record("diff");
        self.inner.diff(repo, comparison)
    }

    fn refs(&self, repo: &Path) -> Result<stikk_prikk::BranchList> {
        self.record("refs");
        self.inner.refs(repo)
    }
    fn tags(&self, repo: &Path) -> Result<Vec<stikk_prikk::RefEntry>> {
        self.record("tags");
        self.inner.tags(repo)
    }
    fn worktree_status(&self, repo: &Path, reff: &str) -> Result<stikk_prikk::WorktreeStatus> {
        self.record("worktree_status");
        self.inner.worktree_status(repo, reff)
    }
    fn queue(&self, repo: &Path) -> Result<stikk_prikk::QueueReport> {
        self.record("queue");
        self.inner.queue(repo)
    }
    fn show(&self, repo: &Path, id: &str) -> Result<stikk_prikk::ShowReport> {
        self.record("show");
        self.inner.show(repo, id)
    }
    fn change_token(&self, repo: &Path) -> Result<ChangeToken> {
        self.record("change_token");
        self.inner.change_token(repo)
    }
    fn commit(&self, repo: &Path, reff: &str, message: &str) -> Result<CommitResult> {
        self.record("commit");
        self.inner.commit(repo, reff, message)
    }
    fn seal(&self, repo: &Path, reff: &str) -> Result<SealResult> {
        self.record("seal");
        self.inner.seal(repo, reff)
    }
}

/// RFC 031 §3: the Orient arm reads the change token **before** Orientation, and answers both at once.
/// Read after, a change landing between the two would be absorbed into the stamp and never noticed.
#[test]
fn the_orient_arm_reads_the_change_token_before_orientation() {
    let backend = Recording {
        inner: NullBackend::supported(),
        calls: std::sync::Mutex::new(Vec::new()),
    };
    let (req_tx, req_rx) = mpsc::channel();
    let (res_tx, res_rx) = mpsc::channel();
    req_tx
        .send(Request {
            seq: 3,
            kind: RequestKind::Orient,
        })
        .unwrap();
    drop(req_tx);

    run(&backend, Path::new("/repo"), req_rx, res_tx);

    let calls = backend.calls.lock().unwrap().clone();
    let token_at = calls.iter().position(|c| *c == "change_token");
    let orientation_at = calls.iter().position(|c| *c == "orientation");
    assert!(
        matches!((token_at, orientation_at), (Some(t), Some(o)) if t < o),
        "the token must be read before Orientation: {calls:?}"
    );
    // The token read is complete before any of Orientation's own calls begin.
    let token_calls_end = calls.iter().rposition(|c| *c == "change_token").unwrap();
    assert!(
        calls
            .get(..token_calls_end)
            .unwrap()
            .iter()
            .all(|c| *c == "change_token")
    );
    let response = res_rx.try_recv().expect("one response");
    assert_eq!(response.seq, 3);
    assert!(matches!(
        response.kind,
        ResponseKind::Orient(OrientRead {
            token: Ok(_),
            view: Ok(_)
        })
    ));
    assert!(res_rx.try_recv().is_err(), "both answered in one response");
}

#[test]
fn a_change_check_reads_only_the_change_token() {
    let backend = Recording {
        inner: NullBackend::supported(),
        calls: std::sync::Mutex::new(Vec::new()),
    };
    let (req_tx, req_rx) = mpsc::channel();
    let (res_tx, res_rx) = mpsc::channel();
    req_tx
        .send(Request {
            seq: 4,
            kind: RequestKind::ChangeCheck,
        })
        .unwrap();
    drop(req_tx);

    run(&backend, Path::new("/repo"), req_rx, res_tx);

    assert_eq!(*backend.calls.lock().unwrap(), vec!["change_token"]);
    let response = res_rx.try_recv().expect("one response");
    assert!(matches!(response.kind, ResponseKind::ChangeCheck(Ok(_))));
}
