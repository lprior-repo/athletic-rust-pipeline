use std::collections::VecDeque;

use tokio::sync::oneshot;
use url::Url;

use super::{reject_pending, Pending};
use crate::request::{RequestAction, RequestSpec};
use crate::{BrowserError, BrowserOutcome, Verdict};

fn waiting() -> Result<(Pending, oneshot::Receiver<BrowserOutcome>), url::ParseError> {
    let (reply, answer) = oneshot::channel();
    let request = RequestSpec {
        url: Url::parse("https://example.test/capture")?,
        semantic_url: "https://example.test/capture".to_string(),
        action: RequestAction::Fetch { body: None },
    };
    Ok((Pending { request, reply }, answer))
}

#[test]
fn a_dead_browser_leaves_every_queued_request_with_a_terminal_failure(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (first, first_answer) = waiting()?;
            let (second, second_answer) = waiting()?;
            let mut queue = VecDeque::from([first, second]);

            reject_pending(&mut queue, BrowserError::Unavailable);

            check!(
                queue.is_empty(),
                "the queue is drained, not partly answered"
            );
            for answer in [first_answer, second_answer] {
                let outcome = answer.await?;
                let BrowserOutcome::Failed(failure) = outcome else {
                    return Err(
                        "a dead browser returned a capture instead of a terminal failure".into(),
                    );
                };
                check!(eq; failure.error, BrowserError::Unavailable);
                check!(eq; failure.verdict,
Verdict::Terminal,
"the lane is gone for this caller: terminal, not a retry it should keep spinning on");
                check!(
                    outcome.response().is_none(),
                    "there is no response, so there is no match and no non-match to mistake it for"
                );
            }
            Ok(())
        })
}
