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
    fn block_state(&self, repo: &Path, reff: &str) -> Result<stikk_prikk::StateFiles> {
        self.record("block_state");
        self.inner.block_state(repo, reff)
    }
    fn refs(&self, repo: &Path) -> Result<Vec<RefEntry>> {
        self.record("refs");
        self.inner.refs(repo)
    }
    fn tags(&self, repo: &Path) -> Result<Vec<RefEntry>> {
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
