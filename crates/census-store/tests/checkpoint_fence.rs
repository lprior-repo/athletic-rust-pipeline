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
    let outcome = batch.commit_once_at_sequence("review:one", "payload-one", observed);
    if !matches!(outcome, Err(StoreError::Invariant { .. })) {
        return Err(format!("expected invariant refusal: {outcome:?}").into());
    }
    {
        let (left, right) = (&store.snapshot().sequence(), &current);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&store.scan::<ReviewCase>(Table::ReviewCases)?, &Vec::new());
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?,
            &Vec::new(),
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &store.journal_payloads("review-checkpoint")?,
            &Vec::<serde_json::Value>::new(),
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&store.receipt("review:one")?, &None);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &store.scan::<CanonicalSchool>(Table::Schools)?,
            &vec![school],
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
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
        if !matches!(applied, Application::Written(_)) {
            return Err(format!("expected written checkpoint: {applied:?}").into());
        }
        receipt = applied.receipt().clone();
        case = expected_case;
        verdict = expected_verdict;
        {
            let (left, right) = (
                &store.scan::<ReviewCase>(Table::ReviewCases)?,
                &vec![case.clone()],
            );
            if left != right {
                return Err(format!("left={left:?} right={right:?}").into());
            }
        }
        {
            let (left, right) = (
                &store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?,
                &vec![verdict.clone()],
            );
            if left != right {
                return Err(format!("left={left:?} right={right:?}").into());
            }
        }
    }
    let store = Store::open(root.path())?;
    let current = store.snapshot().sequence();
    let before_journal = store.journal_payloads("review-checkpoint")?;
    let (batch, _, _) = checkpoint(&store)?;
    {
        let (left, right) = (
            &batch.commit_once_at_sequence("review:one", "payload-one", original_sequence)?,
            &Application::Repeated(receipt.clone()),
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&store.snapshot().sequence(), &current);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&store.scan::<ReviewCase>(Table::ReviewCases)?, &vec![case]);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?,
            &vec![verdict],
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &store.journal_payloads("review-checkpoint")?,
            &before_journal,
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&store.receipt("review:one")?, &Some(receipt));
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&store.receipt_count()?, &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}
