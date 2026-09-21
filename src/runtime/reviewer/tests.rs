use super::model::Attempt;
use super::*;

fn digest(character: char) -> crate::domain::identity::EvidenceDigest {
    crate::domain::identity::EvidenceDigest::parse(&character.to_string().repeat(64))
        .expect("synthetic digest")
}

fn failed_record(character: char, cooldown_ms: u64) -> crate::store::AttemptEvidence<Attempt> {
    let response = crate::runtime::protocol::DocumentReceipt {
        digest: digest(character),
        source_url: "http://127.0.0.1:9000/v1/chat/completions".to_owned(),
        http_status: 503,
        media_type: "application/json".to_owned(),
        bytes: 1,
        fetched_at_unix_ms: 1,
        elapsed_ms: 1,
        rankings: None,
    };
    crate::store::AttemptEvidence {
        digest: digest(character),
        value: Attempt::Failure {
            code: FailureCode::HttpFailure,
            message: "synthetic transient failure".to_owned(),
            status: Some(503),
            receipt: Some(response),
            retryable: true,
            retry_after_ms: cooldown_ms,
        },
    }
}

#[test]
fn sdk_exhaustion_retains_every_receipt_and_final_cooldown() {
    let records = vec![
        failed_record('a', 0),
        failed_record('b', 1_000),
        failed_record('c', 2_000),
    ];
    let finalized = finalize(
        digest('d'),
        records,
        Err(TerminalError::new("SDK exhausted")),
        digest('e'),
        ModelLane::Q5_5090,
    )
    .expect("synthetic finalization");
    assert_eq!(finalized.cooldown_ms, 2_000);
    let ReviewOutcome::Failed { failure, .. } = finalized.outcome else {
        panic!("SDK exhaustion must fail");
    };
    assert_eq!(failure.evidence.len(), 3);
    assert!(matches!(
        failure.retries,
        crate::runtime::protocol::RetryEvidence::SdkControlled {
            observed_attempts: 3,
            ..
        }
    ));
}
