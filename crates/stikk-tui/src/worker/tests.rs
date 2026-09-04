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
    assert!(matches!(response.kind, ResponseKind::Orient(Ok(_))));
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
