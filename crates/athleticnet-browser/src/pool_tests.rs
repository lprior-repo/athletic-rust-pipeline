//! What a caller still waiting in the queue is told when the browser dies under it.
//!
//! §59 asks for the kill-browser leg explicitly: kill the browser mid-flight and the in-flight
//! requests must come back *explicitly unavailable*, never as an answer. The actor does that by
//! rejecting its whole queue when the handler it was talking to goes away, so this file pins the
//! one thing that decision has to guarantee — every waiter leaves with a failure it can classify,
//! and none of them is left waiting for a reply that will never come.

use std::collections::VecDeque;

use tokio::sync::oneshot;
use url::Url;

use super::{reject_pending, Pending};
use crate::request::{RequestAction, RequestSpec};
use crate::{BrowserError, BrowserOutcome, Verdict};

/// One queued request, with the receiver its caller would be waiting on.
fn waiting() -> (Pending, oneshot::Receiver<BrowserOutcome>) {
    let (reply, answer) = oneshot::channel();
    let request = RequestSpec {
        url: Url::parse("https://example.test/capture").expect("a parsed url"),
        semantic_url: "https://example.test/capture".to_string(),
        action: RequestAction::Fetch { body: None },
    };
    (Pending { request, reply }, answer)
}

#[tokio::test]
async fn a_dead_browser_leaves_every_queued_request_with_a_terminal_failure() {
    let (first, first_answer) = waiting();
    let (second, second_answer) = waiting();
    let mut queue = VecDeque::from([first, second]);

    reject_pending(&mut queue, BrowserError::Unavailable);

    assert!(
        queue.is_empty(),
        "the queue is drained, not partly answered"
    );
    for answer in [first_answer, second_answer] {
        let outcome = answer.await.expect("every waiter is answered");
        let BrowserOutcome::Failed(failure) = outcome else {
            panic!("a browser that died answers with a failure, never with a capture");
        };
        assert_eq!(failure.error, BrowserError::Unavailable);
        assert_eq!(
            failure.verdict,
            Verdict::Terminal,
            "the lane is gone for this caller: terminal, not a retry it should keep spinning on"
        );
        assert!(
            outcome.response().is_none(),
            "there is no response, so there is no match and no non-match to mistake it for"
        );
    }
}
