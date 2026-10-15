use super::{read, wire, Inputs};
use crate::journal::context::{stage_context_object, StagedContextObject};
use crate::public_identity_tests::support::{subject, TestResult};
use crate::{Store, StoreError, StoreResult, Table};
use census_domain::model::{
    AthleteIdentityIndex, CanonicalAthlete, Grade, ObservedGrade, ReviewCase, ReviewVerdictRecord,
    SchoolYear, SourceAthleteObservation, SourceNamespace, SourceObservation, SourceRef,
    StoreIdentity, ATHLETE_IDENTITY_FAMILY,
};
use serde_json::Value;

pub(super) fn full_inputs() -> TestResult<Inputs> {
    let first = subject("context-first", "9201")?;
    let second = subject("context-second", "9202")?;
    let reviewed = subject("context-reviewed", "9203")?;
    let case = blocking_case(&reviewed)?;
    let verdict = ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        kind: "rejected".into(),
        field: "identity".into(),
        value: "insufficient_evidence".into(),
        accepted: false,
        confidence: 0,
        rationale: "Retained synthetic review requires corroboration".into(),
        reviewer: "context-fixture".into(),
        observed_at: "2026-10-14T00:00:00Z".into(),
        member_ids: case.member_ids.clone(),
    };
    let observation = SourceAthleteObservation::new(
        SourceNamespace::MilesplitAthlete,
        "9201",
        "original-source-row",
        "Synthetic Runner context-first",
        "2026-10-14",
    )
    .with_grade(Some(grade()?));
    Ok(Inputs {
        athletes: vec![first, second, reviewed],
        observations: vec![SourceObservation::Athlete(observation)],
        cases: vec![case],
        verdicts: vec![verdict],
    })
}

pub(super) fn grade() -> TestResult<ObservedGrade> {
    Ok(ObservedGrade {
        grade: Grade::new(11).ok_or("invalid fixture grade")?,
        school_year: SchoolYear::new(2025).ok_or("invalid fixture school year")?,
        source: SourceRef::new(
            "retained context grade",
            Some("https://mi.milesplit.com/athletes/9201".into()),
        ),
    })
}

pub(super) fn blocking_case(athlete: &CanonicalAthlete) -> TestResult<ReviewCase> {
    let mut index = AthleteIdentityIndex::default();
    index.observe(athlete)?;
    let members = vec![athlete.id.cast()];
    let subject = "Retained identity ambiguity";
    let detail = "Synthetic evidence requires adjudication";
    let evidence = index.case_evidence(subject, detail, &members)?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        athlete.id.as_str(),
        subject,
        detail,
        evidence,
    );
    case.member_ids = members;
    Ok(case)
}

pub(super) fn install_inputs(store: &Store, inputs: &Inputs) -> TestResult {
    store.append_many(Table::Athletes, &inputs.athletes)?;
    store.append_many(Table::SourceObservations, &inputs.observations)?;
    store.replace_many(Table::ReviewCases, &inputs.cases)?;
    store.replace_many(Table::IdentityVerdicts, &inputs.verdicts)?;
    Ok(())
}

pub(super) fn exact_inputs(inputs: &Inputs) -> TestResult<Value> {
    Ok(serde_json::to_value((
        &inputs.athletes,
        &inputs.observations,
        &inputs.cases,
        &inputs.verdicts,
    ))?)
}

pub(super) fn read_inputs(store: &Store, root: &str) -> StoreResult<Inputs> {
    read::read(
        &store.db.snapshot(),
        &store.journal,
        root,
        &store.store_identity()?,
    )
}

pub(super) fn records(store: &Store, root: &str) -> TestResult<Vec<Value>> {
    let object = crate::read::read_context_object(&store.db.snapshot(), &store.journal, root)?;
    object
        .records()
        .map(|record| {
            let record = record?;
            Ok(serde_json::json!({
                "key": record.entry().key,
                "at": record.entry().at,
                "payload": record.payload(),
            }))
        })
        .collect()
}

pub(super) fn header_mut(records: &mut [Value]) -> TestResult<&mut Value> {
    records
        .first_mut()
        .and_then(|record| record.pointer_mut("/payload/value"))
        .ok_or_else(|| "missing fixture context header".into())
}

pub(super) fn stage_records(store: &Store, records: &[Value]) -> StoreResult<StagedContextObject> {
    stage_context_object(store, |sink| {
        records.iter().try_for_each(|record| sink.record(record))
    })
}

pub(super) fn refusal<T>(result: StoreResult<T>, expected: &str) -> TestResult {
    match result {
        Err(StoreError::Refused { detail }) => check!(eq; detail, expected),
        Err(error) => return Err(format!("expected refusal {expected:?}, got {error:?}").into()),
        Ok(_) => return Err(format!("expected refusal {expected:?}, accepted instead").into()),
    }
    Ok(())
}

pub(super) fn witness_root(store: &Store, sequence: u64) -> TestResult<String> {
    let key = crate::public_identity::witness::key(sequence);
    let raw = store.journal.get(&key)?.ok_or("missing graph witness")?;
    let witness = crate::public_identity::graph_witness::HeldGraphWitness::decode(&key, &raw)?;
    Ok(witness.graph.seal().root().as_str().to_owned())
}

pub(super) fn write_journal(store: &Store, key: &[u8], bytes: &[u8]) -> TestResult {
    let mut batch = store.db.batch();
    batch.insert(&store.journal, key, bytes);
    batch.commit()?;
    Ok(())
}

pub(super) fn empty_header(lineage: &StoreIdentity) -> wire::Header {
    wire::Header {
        version: 1,
        authority_policy: census_domain::model::ATHLETE_IDENTITY_POLICY,
        enrichment_policy: 1,
        lineage: lineage.clone(),
        counts: [0; 4],
    }
}
