use census_domain::model::{CanonicalSchool, ReviewCase, ReviewState};
use census_domain::UsJurisdiction;
use census_store::{Application, DerivedStage, Store, StoreError, Table, MAX_JOURNAL_VALUE_BYTES};

const PHASE: &str = "identity_retained_supersession_v1";
const AT: &str = "2026-10-13";
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn retained() -> ReviewCase {
    let mut case = ReviewCase::pending("Athlete identity", "fixture", "Runner", "old finding");
    case.state = ReviewState::Retained;
    case
}

fn stage<'s>(store: &'s Store, old: &ReviewCase) -> TestResult<DerivedStage<'s>> {
    let mut stage = store.stage_derived()?;
    for table in Table::ALL
        .into_iter()
        .filter(|table| table.generation_partitioned())
    {
        stage.carry_forward(table)?;
    }
    let mut closed = old.clone();
    closed.state = ReviewState::Superseded;
    stage.replace_many(Table::ReviewCases, &[closed])?;
    stage.journal_done_once(PHASE, &old.id, &("successor", "reason"))?;
    Ok(stage)
}

fn expect(store: &Store, old: &ReviewCase, state: ReviewState, journal: bool) -> TestResult {
    let held = store
        .scan::<ReviewCase>(Table::ReviewCases)?
        .into_iter()
        .find(|case| case.id == old.id)
        .ok_or("old case disappeared")?;
    let mut expected = old.clone();
    expected.state = state;
    if held != expected {
        return Err(format!("old evidence changed: {held:?}").into());
    }
    let payload = store.journal_payload(PHASE, &old.id)?;
    if journal {
        if payload != Some(serde_json::json!(["successor", "reason"])) {
            return Err(format!("wrong provenance: {payload:?}").into());
        }
    } else if payload.is_some() {
        return Err("an unpublished stage exposed provenance".into());
    }
    Ok(())
}

#[test]
fn dropped_stage_exposes_neither_supersession_nor_provenance() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let old = retained();
    store.replace(Table::ReviewCases, &old)?;
    let pointer = store.derived_generation();
    drop(stage(&store, &old)?);
    expect(&store, &old, ReviewState::Retained, false)?;
    if store.derived_generation() != pointer {
        return Err("dropping a stage changed the pointer".into());
    }
    drop(store);
    expect(
        &Store::open(directory.path())?,
        &old,
        ReviewState::Retained,
        false,
    )
}

#[test]
fn moved_evidence_refuses_supersession_and_provenance_together() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let old = retained();
    store.replace(Table::ReviewCases, &old)?;
    let pointer = store.derived_generation();
    let pending = stage(&store, &old)?;
    let school = CanonicalSchool::new(UsJurisdiction::Wisconsin, "School", "school", None).0;
    store.append(Table::Schools, &school)?;
    if !matches!(
        pending.publish("supersession:moved", "moved"),
        Err(StoreError::PublicationRefused { .. })
    ) {
        return Err("moved inputs were published".into());
    }
    if store.derived_generation() != pointer || store.receipt("supersession:moved")?.is_some() {
        return Err("refusal changed publication state".into());
    }
    expect(&store, &old, ReviewState::Retained, false)?;
    drop(store);
    expect(
        &Store::open(directory.path())?,
        &old,
        ReviewState::Retained,
        false,
    )
}

#[test]
fn published_supersession_and_provenance_survive_reopen_and_repeat() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let old = retained();
    store.replace(Table::ReviewCases, &old)?;
    let publication = stage(&store, &old)?.publish("supersession:published", "published")?;
    if !matches!(publication.application, Application::Written(_)) {
        return Err("first publication did not write".into());
    }
    expect(&store, &old, ReviewState::Superseded, true)?;
    let pointer = store.derived_generation();
    let receipts = store.receipt_count()?;
    let mut repeated = stage(&store, &old)?;
    repeated.journal_done_once(PHASE, "discard-on-repeat", &("unpublished", AT))?;
    if !matches!(
        repeated
            .publish("supersession:published", "published")?
            .application,
        Application::Repeated(_)
    ) {
        return Err("published operation was not a repeat".into());
    }
    if store.derived_generation() != pointer
        || store.receipt_count()? != receipts
        || store.journal_payload(PHASE, "discard-on-repeat")?.is_some()
    {
        return Err("repeat committed its pending provenance".into());
    }
    drop(store);
    let store = Store::open(directory.path())?;
    expect(&store, &old, ReviewState::Superseded, true)?;
    if store.receipt("supersession:published")?.is_none() {
        return Err("publication lost its receipt".into());
    }
    Ok(())
}

#[test]
fn conflicting_historical_provenance_is_never_overwritten() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let old = retained();
    store.replace(Table::ReviewCases, &old)?;
    let mut written = store.write_batch();
    written.journal_done(
        PHASE,
        &old.id,
        &("historical-successor", "historical-reason"),
    )?;
    written.commit()?;
    let historical = store.journal_payload(PHASE, &old.id)?;
    let pointer = store.derived_generation();
    if !matches!(
        stage(&store, &old)?.publish("supersession:conflict", "conflict"),
        Err(StoreError::PublicationRefused { .. })
    ) {
        return Err("contradictory historical provenance was accepted".into());
    }
    if store.journal_payload(PHASE, &old.id)? != historical
        || store.derived_generation() != pointer
        || store.receipt("supersession:conflict")?.is_some()
    {
        return Err("contradictory publication changed historical state".into());
    }
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    if !cases.contains(&old) {
        return Err("historical provenance conflict closed the old case".into());
    }
    Ok(())
}

#[test]
fn a_journal_race_that_does_not_move_evidence_is_rechecked_under_the_fence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let old = retained();
    store.replace(Table::ReviewCases, &old)?;
    let pending = stage(&store, &old)?;
    let evidence = store.evidence_generation();
    let mut concurrent = store.write_batch();
    concurrent.journal_done(PHASE, &old.id, &("concurrent-successor", "reason"))?;
    concurrent.commit()?;
    if store.evidence_generation() != evidence {
        return Err("fixture journal unexpectedly moved evidence".into());
    }
    if !matches!(
        pending.publish("supersession:race", "race"),
        Err(StoreError::PublicationRefused { .. })
    ) {
        return Err("concurrent contradictory provenance was overwritten".into());
    }
    if store.journal_payload(PHASE, &old.id)?
        != Some(serde_json::json!(["concurrent-successor", "reason"]))
    {
        return Err("publication changed the concurrent journal".into());
    }
    Ok(())
}

#[test]
fn pending_journal_keys_are_unique_and_conflicts_leave_the_first_payload() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let old = retained();
    store.replace(Table::ReviewCases, &old)?;
    let mut pending = stage(&store, &old)?;
    pending.journal_done_once(PHASE, &old.id, &("successor", "reason"))?;
    if !matches!(
        pending.journal_done_once(PHASE, &old.id, &("different", "reason")),
        Err(StoreError::PublicationRefused { .. })
    ) {
        return Err("one pending key accepted contradictory payloads".into());
    }
    pending.publish("supersession:unique", "unique")?;
    expect(&store, &old, ReviewState::Superseded, true)
}

#[test]
fn oversized_or_ambiguous_journals_are_refused_before_publication() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut pending = store.stage_derived()?;
    if !matches!(
        pending.journal_done_once(PHASE, "oversized", &"x".repeat(MAX_JOURNAL_VALUE_BYTES)),
        Err(StoreError::JournalTooLarge { .. })
    ) {
        return Err("publication journal exceeded its per-entry bound".into());
    }
    for (phase, key) in [
        ("", "key"),
        (PHASE, ""),
        ("phase\0part", "key"),
        (PHASE, "key\0part"),
    ] {
        if !matches!(
            pending.journal_done_once(phase, key, &"payload"),
            Err(StoreError::Refused { .. })
        ) {
            return Err("ambiguous publication journal key was accepted".into());
        }
    }
    drop(pending);
    if store.journal_payload(PHASE, "oversized")?.is_some() {
        return Err("refused journal became durable".into());
    }
    Ok(())
}

struct FailingSerialize;

impl serde::Serialize for FailingSerialize {
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(
            "intentional fixture serialization failure",
        ))
    }
}

#[test]
fn serializer_failure_is_typed_for_both_publication_and_generic_journals() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut stage = store.stage_derived()?;
    if !matches!(
        stage.journal_done_once(PHASE, "stage-failure", &FailingSerialize),
        Err(StoreError::Json { .. })
    ) {
        return Err("stage serialization failure was not returned".into());
    }
    drop(stage);
    let mut batch = store.write_batch();
    if !matches!(
        batch.journal_done(PHASE, "generic-failure", &FailingSerialize),
        Err(StoreError::Json { .. })
    ) {
        return Err("generic serialization failure was not returned".into());
    }
    if !batch.is_empty() {
        return Err("failed serialization staged journal rows".into());
    }
    Ok(())
}

#[test]
fn aggregate_publication_budget_fails_closed_without_exposing_a_journal() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut stage = store.stage_derived()?;
    let payload = "x".repeat(512 * 1024);
    let mut refused = false;
    for position in 0..520 {
        match stage.journal_done_once(PHASE, &format!("aggregate-{position}"), &payload) {
            Ok(()) => {}
            Err(StoreError::Refused { .. }) => {
                refused = true;
                break;
            }
            Err(error) => return Err(error.into()),
        }
    }
    if !refused {
        return Err("aggregate publication journal byte ceiling was not enforced".into());
    }
    drop(stage);
    if !store.journal_keys(PHASE)?.is_empty() {
        return Err("budget refusal exposed queued journals".into());
    }
    Ok(())
}

#[test]
fn generic_chunked_journal_compatibility_is_preserved() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let payload = "x".repeat(2 * MAX_JOURNAL_VALUE_BYTES);
    let mut batch = store.write_batch();
    batch.journal_done("generic-chunk-fixture", "key", &payload)?;
    batch.commit()?;
    if store.journal_payload("generic-chunk-fixture", "key")?
        != Some(serde_json::Value::String(payload))
    {
        return Err("generic chunked journal format changed".into());
    }
    Ok(())
}
