use census_domain::model::{CanonicalSchool, ReviewCase, ReviewState, ReviewVerdictRecord};
use census_domain::UsJurisdiction;
use census_store::{Application, Store, StoreBatch, StoreError, Table};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn checkpoint(
    store: &Store,
) -> Result<(StoreBatch<'_>, ReviewCase, ReviewVerdictRecord), StoreError> {
    let mut case = ReviewCase::pending(
        "school-jurisdiction",
        "school-fixture",
        "Source School",
        "Jurisdiction unresolved",
    );
    case.state = ReviewState::Resolved;
    let verdict = ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        kind: "value_proposed".into(),
        field: "state".into(),
        value: "NC".into(),
        accepted: true,
        confidence: 100,
        rationale: "bounded checkpoint fixture".into(),
        reviewer: "checkpoint fixture".into(),
        observed_at: "2026-09-27".into(),
        member_ids: vec![],
    };
    let mut batch = store.write_batch();
    batch.replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
    batch.replace_many(Table::IdentityVerdicts, std::slice::from_ref(&verdict))?;
    batch.journal_done(
        "review-checkpoint",
        "one",
        &serde_json::json!({"case": case.id}),
    )?;
    Ok((batch, case, verdict))
}

#[test]
fn source_change_rejects_checkpoint_without_partial_state_verdict_journal_or_receipt() -> TestResult
{
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let observed = store.snapshot().sequence();
    let (batch, _, _) = checkpoint(&store)?;
    let school = CanonicalSchool::new(
        UsJurisdiction::NorthCarolina,
        "New Source School",
        "new source school",
    )
    .0;
    store.append(Table::Schools, &school)?;
    let current = store.snapshot().sequence();
    assert!(matches!(
        batch.commit_once_at_sequence("review:one", "payload-one", observed),
        Err(StoreError::Invariant { .. })
    ));
    assert_eq!(store.snapshot().sequence(), current);
    assert_eq!(store.scan::<ReviewCase>(Table::ReviewCases)?, Vec::new());
    assert_eq!(
        store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?,
        Vec::new()
    );
    assert_eq!(
        store.journal_payloads("review-checkpoint")?,
        Vec::<serde_json::Value>::new()
    );
    assert_eq!(store.receipt("review:one")?, None);
    assert_eq!(store.scan::<CanonicalSchool>(Table::Schools)?, vec![school]);
    Ok(())
}

#[test]
fn fresh_checkpoint_is_atomic_durable_and_exact_replay_does_not_require_the_old_sequence(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let (case, verdict, receipt, original_sequence);
    {
        let store = Store::open(root.path())?;
        original_sequence = store.snapshot().sequence();
        let (batch, expected_case, expected_verdict) = checkpoint(&store)?;
        let applied =
            batch.commit_once_at_sequence("review:one", "payload-one", original_sequence)?;
        assert!(matches!(applied, Application::Written(_)));
        receipt = applied.receipt().clone();
        case = expected_case;
        verdict = expected_verdict;
        assert_eq!(
            store.scan::<ReviewCase>(Table::ReviewCases)?,
            vec![case.clone()]
        );
        assert_eq!(
            store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?,
            vec![verdict.clone()]
        );
    }
    let store = Store::open(root.path())?;
    let current = store.snapshot().sequence();
    let before_journal = store.journal_payloads("review-checkpoint")?;
    let (batch, _, _) = checkpoint(&store)?;
    assert_eq!(
        batch.commit_once_at_sequence("review:one", "payload-one", original_sequence)?,
        Application::Repeated(receipt.clone())
    );
    assert_eq!(store.snapshot().sequence(), current);
    assert_eq!(store.scan::<ReviewCase>(Table::ReviewCases)?, vec![case]);
    assert_eq!(
        store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?,
        vec![verdict]
    );
    assert_eq!(store.journal_payloads("review-checkpoint")?, before_journal);
    assert_eq!(store.receipt("review:one")?, Some(receipt));
    assert_eq!(store.receipt_count()?, 1);
    Ok(())
}
