
use std::collections::VecDeque;

use tokio::sync::oneshot;
use url::Url;

use super::{reject_pending, Pending};
use crate::request::{RequestAction, RequestSpec};
use crate::{BrowserError, BrowserOutcome, Verdict};

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
