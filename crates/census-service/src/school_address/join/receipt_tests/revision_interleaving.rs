use super::*;
use census_store::{Receipt, StoreError, StoreRevision};

const AMBIGUOUS_MEMBER: &str = "Revision Ambiguous Member High School";
const EXTERNAL_WEBSITE: &str = "https://independent.example/revision";

#[derive(Clone, Copy)]
enum CompetingWrite {
    Entity,
    DerivedPublication,
}

#[derive(Debug, PartialEq, Eq)]
struct LiveState {
    sequence: u64,
    tables_digest: String,
    witnesses: Vec<Value>,
    receipts: u64,
}

#[test]
fn job_refuses_target_b_after_competing_entity_write_and_fresh_process_recomputes() -> TestResult {
    competing_writer_interleaving(CompetingWrite::Entity)
}

#[test]
fn job_refuses_target_b_after_derived_publication_and_fresh_process_retains_decision() -> TestResult
{
    competing_writer_interleaving(CompetingWrite::DerivedPublication)
}

fn competing_writer_interleaving(write: CompetingWrite) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let (earlier, later, index) = ordered_targets()?;
    store.append(Table::Schools, &later)?;
    store.append(Table::Schools, &earlier)?;
    let lanes = lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01"));
    let (store, before) = physical::checkpoint(store)?;
    let initial = store.snapshot();
    let initial_pair = pair(initial.revision());
    let targets = initial.scan::<CanonicalSchool>(Table::Schools)?;
    check!(eq; targets, vec![earlier.clone(), later.clone()]);
    check!(eq; initial.scan::<ReviewCase>(Table::ReviewCases)?.len(), 0);
    let mut job = Job::new(&index, &lanes, Mode::Apply, &store, &initial)?;
    job.visit(targets.first().ok_or("snapshot lacks target A")?)?;
    check!(eq; job.pending.len(), 1);
    let effects_a = job.pending.clone();
    job.apply_target(&earlier.id)?;
    check!(eq; job.pending.len(), 0);
    check!(eq; job.pending_cases.len(), 0);
    let application_a = witness::single(&store, &earlier.id, "schools")?;
    let operation_a = operation(&application_a)?;
    let receipt_a = store.receipt(&operation_a)?.ok_or("A receipt missing")?;
    check!(eq; receipt_a.appended, 1);
    let observations_a = observations(&store, &earlier.id)?;
    check!(eq; observations_a, vec![earlier.clone(), effects_a.first().ok_or("A effect missing")?.clone()]);
    let own_pair = pair(store.snapshot().revision());
    check!(eq; own_pair, (initial_pair.0.checked_add(1).ok_or("evidence overflow")?, initial_pair.1));
    check!(eq; job_pair(&job)?, own_pair);
    check!(eq; pair(initial.revision()), initial_pair);
    job.visit(targets.last().ok_or("snapshot lacks target B")?)?;
    check!(eq; job.pending.len(), 1);
    check!(eq; job.pending_cases.len(), 1);
    let schools_b = job.pending.clone();
    let cases_b = job.pending_cases.clone();
    let planned_b =
        planned_application(&application_a, &later.id, &schools_b, &cases_b, own_pair.1)?;
    let operation_b = operation(&planned_b)?;
    let writer = store.clone();
    let decided = compete(write, &writer, &later, &cases_b)?;
    let actual_pair = pair(store.snapshot().revision());
    check!(eq; actual_pair.0, own_pair.0.checked_add(1).ok_or("evidence overflow")?);
    match write {
        CompetingWrite::Entity => check!(eq; actual_pair.1, own_pair.1),
        CompetingWrite::DerivedPublication => check!(ne; actual_pair.1, own_pair.1),
    }
    let retained = LiveState::capture(&store)?;
    assert_revision_moved(
        job.apply_target(&later.id),
        &operation_b,
        own_pair,
        actual_pair,
    )?;
    check!(eq; LiveState::capture(&store)?, retained);
    check!(eq; job_pair(&job)?, own_pair);
    check!(eq; job.pending, schools_b);
    check!(eq; job.pending_cases, cases_b);
    check!(eq; observations(&store, &earlier.id)?, observations_a);
    check!(eq; witness::all(&store)?, vec![application_a.clone()]);
    check!(eq; store.receipt(&operation_a)?, Some(receipt_a.clone()));
    check!(eq; store.receipt(&operation_b)?, None);
    check!(eq; store.journal_payload(WITNESS_PHASE, &operation_b)?, None);
    match job.finish() {
        Err(StoreError::Invariant { detail }) => {
            check!(eq; detail, "school-address join finished with an unapplied target group")
        }
        Err(error) => return Err(format!("wrong unfinished Job error: {error}").into()),
        Ok(_) => return Err("stale Job returned a completed report".into()),
    }
    drop(initial);
    drop(writer);
    let (store, refused) = physical::checkpoint(store)?;
    check!(eq; LiveState::capture(&store)?, retained);
    check!(eq; target_rows(&refused, &earlier.id)?.len(), 2);
    check!(eq; refused.applications()?.len(), 1);
    let refused_schools = refused.schools()?;
    check!(before
        .schools()?
        .iter()
        .all(|row| refused_schools.contains(row)));
    let refused_b = target_rows(&refused, &later.id)?;
    match write {
        CompetingWrite::Entity => check!(eq; refused_b.len(), 2),
        CompetingWrite::DerivedPublication => check!(eq; refused_b.len(), 1),
    }
    assert_binding(&store, &refused, &application_a, &receipt_a, &effects_a)?;
    let (counters, outcomes) = process(&store, &index, &lanes, Mode::Apply)?;
    assert_fresh_report(write, &counters, &outcomes, &later)?;
    let merged = store.scan::<CanonicalSchool>(Table::Schools)?;
    let recomputed_b = merged
        .iter()
        .find(|school| school.id == later.id)
        .ok_or("fresh B missing")?;
    check!(eq; recomputed_b.source_identities.len(), 1);
    check!(eq; recomputed_b.postal_addresses.len(), 1);
    check!(eq; recomputed_b.postal_addresses.first().map(|claim| claim.owner().id.as_str()), Some("390000000010"));
    let applications = witness::all(&store)?;
    check!(eq; applications.len(), 2);
    check!(applications.contains(&application_a));
    let application_b = applications
        .iter()
        .find(|value| value.get("target").and_then(Value::as_str) == Some(later.id.as_str()))
        .ok_or("fresh B witness missing")?;
    check!(ne; operation(application_b)?, operation_b);
    let receipt_b = store
        .receipt(&operation(application_b)?)?
        .ok_or("fresh B receipt missing")?;
    let observations_b = observations(&store, &later.id)?;
    let effects_b = [observations_b
        .last()
        .ok_or("fresh B effect missing")?
        .clone()];
    match write {
        CompetingWrite::Entity => {
            check!(eq; recomputed_b.school_website.as_deref(), Some(EXTERNAL_WEBSITE));
            check!(eq; recomputed_b.classification.as_deref(), Some("independent writer retained"));
            check!(eq; application_b.get("mutation"), Some(&json!("schools-and-review-cases")));
            check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?, cases_b);
        }
        CompetingWrite::DerivedPublication => {
            check!(eq; application_b.get("mutation"), Some(&json!("schools")));
            check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?, decided);
        }
    }
    let (store, completed) = physical::checkpoint(store)?;
    check!(eq; target_rows(&completed, &earlier.id)?, target_rows(&refused, &earlier.id)?);
    let completed_b = target_rows(&completed, &later.id)?;
    check!(eq; completed_b.len(), refused_b.len().checked_add(1).ok_or("row count overflow")?);
    check!(refused_b.iter().all(|row| completed_b.contains(row)));
    check!(eq; completed.applications()?.len(), 2);
    assert_binding(&store, &completed, &application_a, &receipt_a, &effects_a)?;
    assert_binding(&store, &completed, application_b, &receipt_b, &effects_b)?;
    for space in ["journal", "receipts"] {
        let rows = completed.rows(space)?;
        check!(refused.rows(space)?.iter().all(|row| rows.contains(row)));
    }
    let (repeated, _) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; repeated.scanned, 2);
    check!(eq; repeated.linked, 0);
    check!(eq; repeated.already_linked, 2);
    check!(eq; repeated.backfilled, 0);
    check!(eq; repeated.review_filed, 0);
    check!(eq; repeated.review_present, 1);
    let (_store, repeated) = physical::checkpoint(store)?;
    check!(eq; repeated, completed);
    check!(eq; before.schools()?.len(), 2);
    Ok(())
}

fn ordered_targets() -> TestResult<(CanonicalSchool, CanonicalSchool, DirectoryIndex)> {
    let mut schools = [
        "Revision Candidate One High School",
        "Revision Candidate Two High School",
    ]
    .map(|name| {
        CanonicalSchool::new(
            UsJurisdiction::Ohio,
            name,
            normalize_name(name),
            Some("Springfield"),
        )
        .0
    });
    schools.sort_by(|left, right| left.id.cmp(&right.id));
    let [earlier, mut later] = schools;
    later.co_op = true;
    later.aliases.push(AMBIGUOUS_MEMBER.to_string());
    check!(earlier.id < later.id);
    let index = DirectoryIndex::build(&[
        nces_entry_with_website(
            "390000000001",
            &earlier.name,
            "Springfield",
            UsJurisdiction::Ohio,
            "1 Durable Rd",
            "https://earlier.example/high",
            (9, 12),
        )?,
        nces_entry_with_website(
            "390000000010",
            &later.name,
            "Springfield",
            UsJurisdiction::Ohio,
            "10 Recomputed Rd",
            "https://later.example/high",
            (9, 12),
        )?,
        nces_entry(
            "390000000002",
            AMBIGUOUS_MEMBER,
            "Springfield",
            UsJurisdiction::Ohio,
            "2 Member Rd",
            (9, 12),
        )?,
        nces_entry(
            "390000000003",
            AMBIGUOUS_MEMBER,
            "Springfield",
            UsJurisdiction::Ohio,
            "3 Member Rd",
            (9, 12),
        )?,
    ]);
    Ok((earlier, later, index))
}

fn compete(
    write: CompetingWrite,
    store: &Store,
    later: &CanonicalSchool,
    cases: &[ReviewCase],
) -> TestResult<Vec<ReviewCase>> {
    match write {
        CompetingWrite::Entity => {
            let mut changed = later.clone();
            changed.school_website = Some(EXTERNAL_WEBSITE.to_string());
            changed.classification = Some("independent writer retained".to_string());
            store.append(Table::Schools, &changed)?;
            Ok(Vec::new())
        }
        CompetingWrite::DerivedPublication => {
            let mut decided = cases.to_vec();
            let case = decided.first_mut().ok_or("stale B lacks review case")?;
            case.state = ReviewState::Resolved;
            case.detail = "independent published decision retained".to_string();
            let mut stage = store.stage_derived()?;
            for table in Table::ALL {
                if table.generation_partitioned() {
                    stage.replace_many::<Value>(table, &[])?;
                }
            }
            stage.replace_many(Table::ReviewCases, &decided)?;
            let publication = stage.publish(
                "revision-interleaving:derived",
                "revision-interleaving:derived-digest",
            )?;
            check!(publication.application.written());
            check!(eq; store.derived_generation(), publication.generation);
            Ok(decided)
        }
    }
}

fn planned_application(
    a: &Value,
    target: &SchoolId,
    schools: &[CanonicalSchool],
    cases: &[ReviewCase],
    derived: u64,
) -> TestResult<Value> {
    Ok(json!({
        "application_version": APPLICATION_VERSION,
        "effects": [
            {"payload_digest": serialized_digest(schools)?, "placement": "unpartitioned", "table": Table::Schools.file(), "target": target},
            {"payload_digest": serialized_digest(cases)?, "placement": {"derived-generation": derived}, "table": Table::ReviewCases.file(), "target": target},
        ],
        "mutation": "schools-and-review-cases",
        "provenance": a.get("provenance").ok_or("A provenance missing")?,
        "scope_digest": a.get("scope_digest").ok_or("A scope digest missing")?,
        "target": target,
    }))
}

fn operation(application: &Value) -> TestResult<String> {
    Ok(format!(
        "{OPERATION_PREFIX}{}",
        serialized_digest(application)?
    ))
}

fn pair(revision: StoreRevision<'_>) -> (u64, u64) {
    (
        revision.evidence_generation(),
        revision.derived_generation(),
    )
}

fn job_pair(job: &Job<'_>) -> TestResult<(u64, u64)> {
    match &job.application {
        apply::context::ApplicationContext::Apply { revision, .. } => Ok(pair(*revision)),
        apply::context::ApplicationContext::DryRun => Err("Job lost Apply context".into()),
    }
}

fn assert_revision_moved(
    result: census_store::StoreResult<()>,
    operation: &str,
    expected: (u64, u64),
    actual: (u64, u64),
) -> TestResult {
    match result {
        Err(StoreError::RevisionMoved {
            operation: found,
            expected_evidence,
            actual_evidence,
            expected_derived,
            actual_derived,
        }) => {
            check!(eq; found, operation);
            check!(eq; (expected_evidence, expected_derived), expected);
            check!(eq; (actual_evidence, actual_derived), actual);
        }
        Err(error) => return Err(format!("stale B had wrong refusal: {error}").into()),
        Ok(()) => return Err("stale B unexpectedly committed".into()),
    }
    Ok(())
}

impl LiveState {
    fn capture(store: &Store) -> TestResult<Self> {
        let snapshot = store.snapshot();
        Ok(Self {
            sequence: snapshot.sequence(),
            tables_digest: snapshot.tables_digest(&Table::ALL)?,
            witnesses: witness::all(store)?,
            receipts: store.receipt_count()?,
        })
    }
}

fn observations(store: &Store, target: &SchoolId) -> TestResult<Vec<CanonicalSchool>> {
    let mut rows = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Schools, |school: CanonicalSchool| {
            if &school.id == target {
                rows.push(school);
            }
            Ok(())
        })?;
    Ok(rows)
}

fn target_rows(state: &physical::Physical, target: &SchoolId) -> TestResult<physical::Rows> {
    let prefix = format!("{}\0{}\0", Table::Schools.file(), target.as_str());
    Ok(state
        .schools()?
        .into_iter()
        .filter(|(key, _)| key.starts_with(prefix.as_bytes()))
        .collect())
}

fn assert_binding(
    store: &Store,
    state: &physical::Physical,
    application: &Value,
    receipt: &Receipt,
    effects: &[CanonicalSchool],
) -> TestResult {
    check!(eq; witness::effect(application, Table::Schools)?.get("payload_digest").and_then(Value::as_str), Some(serialized_digest(effects)?.as_str()));
    check!(eq; receipt.digest.as_str(), serialized_digest(application)?.as_str());
    check!(eq; receipt.operation, operation(application)?);
    check!(eq; receipt.appended, 1);
    check!(eq; receipt.credited, false);
    check!(eq; store.receipt(&receipt.operation)?, Some(receipt.clone()));
    let receipt_row = state
        .rows("receipts")?
        .iter()
        .find(|(key, _)| key.as_slice() == receipt.operation.as_bytes())
        .ok_or("A physical receipt missing")?;
    check!(eq; serde_json::from_slice::<Receipt>(&receipt_row.1)?, receipt.clone());
    let key = format!("{WITNESS_PHASE}\0{}", receipt.operation);
    let journal = state
        .rows("journal")?
        .iter()
        .find(|(held, _)| held.as_slice() == key.as_bytes())
        .ok_or("A physical witness missing")?;
    check!(eq; serde_json::from_slice::<Value>(&journal.1)?.get("payload"), Some(application));
    Ok(())
}

fn assert_fresh_report(
    write: CompetingWrite,
    counters: &Counters,
    outcomes: &[OutcomeRow],
    later: &CanonicalSchool,
) -> TestResult {
    check!(eq; (counters.scanned, counters.linked, counters.already_linked, counters.backfilled), (2, 1, 1, 0));
    check!(eq; (counters.postal_filled, counters.ccd_filled, counters.co_op_members, counters.review), (1, 1, 1, 1));
    match write {
        CompetingWrite::Entity => {
            check!(eq; (counters.review_filed, counters.review_present, counters.websites), (1, 0, 0))
        }
        CompetingWrite::DerivedPublication => {
            check!(eq; (counters.review_filed, counters.review_present, counters.websites), (0, 1, 1))
        }
    }
    check!(eq; outcomes.len(), 1);
    let row = outcomes.first().ok_or("fresh review outcome missing")?;
    check!(eq; (row.school_id.as_str(), row.outcome.as_str(), row.reason.as_deref()), (later.id.as_str(), "review", Some("ambiguous")));
    Ok(())
}
