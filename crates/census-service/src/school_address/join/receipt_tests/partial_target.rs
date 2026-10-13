use super::*;
use census_store::{
    ObservationBudget, StoreError, MAX_OBSERVATION_BATCH_BYTES, MAX_OBSERVATION_RECORD_BYTES,
};

#[test]
fn earlier_target_survives_later_budget_refusal_and_retries_recompute_without_duplicates(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let (earlier, later, entries) = ordered_targets()?;
    assert_aggregate_fixture(&later)?;
    store.append(Table::Schools, &later)?;
    store.append(Table::Schools, &earlier)?;
    let initial_generation = store.snapshot().evidence_generation();
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?.into_iter()
        .map(|school| school.id).collect::<Vec<_>>(),
        vec![earlier.id.clone(), later.id.clone()]);
    let index = DirectoryIndex::build(&entries);
    let lanes = lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01"));
    let (store, before) = physical::checkpoint(store)?;
    check!(eq; before.schools()?.len(), 2);
    check!(eq; before.applications()?.len(), 0);

    assert_aggregate_refusal(process(&store, &index, &lanes, Mode::Apply))?;
    let first_witness = witness::single(&store, &earlier.id, "schools")?;
    check!(eq; store.receipt_count()?, 1);
    check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?.len(), 0);
    check!(eq; store.snapshot().evidence_generation(),
        initial_generation.checked_add(1).ok_or("generation overflow")?);
    let (store, partial) = physical::checkpoint(store)?;
    assert_partial_state(&store, &before, &partial, &earlier, &later, &first_witness)?;

    assert_aggregate_refusal(process(&store, &index, &lanes, Mode::Apply))?;
    let (store, refused_retry) = physical::checkpoint(store)?;
    check!(eq; refused_retry, partial);
    check!(eq; witness::all(&store)?, vec![first_witness.clone()]);
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?.len(), 2);

    let reduced_entries = entries.get(..2).ok_or("fixture lacks two main entries")?;
    let reduced_index = DirectoryIndex::build(reduced_entries);
    let (recomputed, outcomes) = process(&store, &reduced_index, &lanes, Mode::Apply)?;
    assert_recomputed_report(&recomputed, &outcomes, &later)?;
    let (store, completed) = physical::checkpoint(store)?;
    check!(eq; target_rows(&completed, &earlier.id)?, target_rows(&partial, &earlier.id)?);
    check!(eq; completed.schools()?.len(), 4);
    check!(eq; completed.applications()?.len(), 2);
    check!(eq; completed.rows("entities")?.len(), 4);
    check!(eq; completed.rows("journal")?.len(), 2);
    check!(eq; completed.rows("receipts")?.len(), 2);
    assert_completed_state(&store, &completed, &partial, &later, &first_witness)?;

    let (repeated, repeated_outcomes) = process(&store, &reduced_index, &lanes, Mode::Apply)?;
    check!(eq; repeated.scanned, 2);
    check!(eq; repeated.linked, 0);
    check!(eq; repeated.already_linked, 2);
    check!(eq; repeated.backfilled, 0);
    check!(eq; repeated.postal_filled, 0);
    check!(eq; repeated.review_filed, 0);
    check!(eq; repeated_outcomes.len(), 8);
    let (_store, after_repeat) = physical::checkpoint(store)?;
    check!(eq; after_repeat, completed);
    Ok(())
}

fn ordered_targets() -> TestResult<(CanonicalSchool, CanonicalSchool, Vec<SchoolDirectoryEntry>)> {
    let mut candidates = [
        co_op_school("Atomic Candidate One High School", "Riverside", &[])?,
        co_op_school("Atomic Candidate Two High School", "Riverside", &[])?,
    ];
    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    let [mut earlier, mut later] = candidates;
    earlier.co_op = false;
    later.aliases = (1..9)
        .map(|index| format!("Budget Member {index} High School"))
        .collect();
    later.classification = Some("x".repeat(950_000));
    check!(earlier.id < later.id);
    let mut entries = vec![
        entry(
            "470000000001",
            &earlier.name,
            "Riverside",
            UsJurisdiction::Tennessee,
            "1 Durable Rd",
        )?,
        entry(
            "470000000010",
            &later.name,
            "Riverside",
            UsJurisdiction::Tennessee,
            "10 Pending Rd",
        )?,
    ];
    for (offset, name) in later.aliases.iter().enumerate() {
        let number = offset.checked_add(11).ok_or("member number overflow")?;
        entries.push(entry(
            &format!("4700000000{number:02}"),
            name,
            "Riverside",
            UsJurisdiction::Tennessee,
            &format!("{number} Member Rd"),
        )?);
    }
    Ok((earlier, later, entries))
}

fn assert_aggregate_fixture(later: &CanonicalSchool) -> TestResult {
    check!(eq; later.aliases.len(), 8);
    let record_bytes = ObservationBudget::record_bytes(later)?;
    check!(record_bytes < MAX_OBSERVATION_RECORD_BYTES);
    let effects = later
        .aliases
        .len()
        .checked_add(1)
        .ok_or("effect count overflow")?;
    let minimum_bytes = record_bytes
        .checked_mul(effects)
        .ok_or("fixture byte overflow")?;
    check!(minimum_bytes > MAX_OBSERVATION_BATCH_BYTES);
    Ok(())
}

fn assert_aggregate_refusal(result: Result<(Counters, Vec<OutcomeRow>), JoinError>) -> TestResult {
    match result {
        Err(JoinError::Store(StoreError::BudgetExceeded {
            what,
            needed,
            available,
        })) => {
            check!(eq; what, "school-address target staged bytes");
            check!(eq; available, u64::try_from(MAX_OBSERVATION_BATCH_BYTES)?);
            check!(needed > available);
        }
        Err(error) => {
            return Err(format!("later-target aggregate refusal had wrong reason: {error}").into())
        }
        Ok(_) => return Err("oversized later target unexpectedly applied".into()),
    }
    Ok(())
}

fn target_rows(physical: &physical::Physical, target: &SchoolId) -> TestResult<physical::Rows> {
    let prefix = format!("{}\0{}\0", Table::Schools.file(), target.as_str());
    Ok(physical
        .schools()?
        .into_iter()
        .filter(|(key, _)| key.starts_with(prefix.as_bytes()))
        .collect())
}

fn school_at(store: &Store, target: &SchoolId) -> TestResult<CanonicalSchool> {
    store
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .find(|school| &school.id == target)
        .ok_or_else(|| "target school is missing".into())
}

fn assert_partial_state(
    store: &Store,
    before: &physical::Physical,
    partial: &physical::Physical,
    earlier: &CanonicalSchool,
    later: &CanonicalSchool,
    application: &Value,
) -> TestResult {
    check!(eq; partial.schools()?.len(), 3);
    check!(eq; partial.applications()?.len(), 1);
    check!(eq; partial.rows("entities")?.len(), 3);
    check!(eq; partial.rows("journal")?.len(), 1);
    check!(eq; partial.rows("receipts")?.len(), 1);
    let retained = partial.schools()?;
    check!(before.schools()?.iter().all(|row| retained.contains(row)));
    check!(eq; target_rows(partial, &later.id)?, target_rows(before, &later.id)?);
    check!(eq; target_rows(partial, &earlier.id)?.len(), 2);
    check!(eq; school_at(store, &later.id)?, later.clone());
    let merged = school_at(store, &earlier.id)?;
    assert_owned_claim(&merged, "470000000001", "1 Durable Rd")?;
    let previous = target_rows(before, &earlier.id)?;
    let added: physical::Rows = target_rows(partial, &earlier.id)?
        .into_iter()
        .filter(|row| !previous.contains(row))
        .collect();
    check!(eq; added.len(), 1);
    assert_application_binding(store, partial, application, &added)?;
    Ok(())
}

fn assert_owned_claim(school: &CanonicalSchool, owner: &str, street: &str) -> TestResult {
    check!(eq; school.source_identities.len(), 1);
    check!(eq; school.source_identities.first().map(|identity| identity.id.as_str()), Some(owner));
    check!(eq; school.postal_addresses.len(), 1);
    let claim = school
        .postal_addresses
        .first()
        .ok_or("committed school lacks postal claim")?;
    check!(eq; claim.owner().id.as_str(), owner);
    check!(eq; claim.address().line1().map(StreetLine::as_str), Some(street));
    check!(eq; claim.capture_sha256(), CAPTURE_SHA);
    check!(eq; claim.evidence().source.url.as_deref(), Some("https://nces.ed.gov/ccd.csv"));
    check!(eq; claim.evidence().observed_on.as_str(), "2026-08-01");
    Ok(())
}

fn assert_recomputed_report(
    counters: &Counters,
    outcomes: &[OutcomeRow],
    later: &CanonicalSchool,
) -> TestResult {
    check!(eq; counters.scanned, 2);
    check!(eq; counters.linked, 1);
    check!(eq; counters.already_linked, 1);
    check!(eq; counters.backfilled, 0);
    check!(eq; counters.ccd_filled, 1);
    check!(eq; counters.co_op_members, 8);
    check!(eq; counters.co_op_declined, 8);
    check!(eq; counters.review_filed, 0);
    check!(eq; outcomes.len(), 8);
    check!(outcomes
        .iter()
        .all(|row| row.school_id == later.id.as_str() && row.outcome == "co_op_declined"));
    Ok(())
}

fn assert_completed_state(
    store: &Store,
    completed: &physical::Physical,
    partial: &physical::Physical,
    later: &CanonicalSchool,
    first_witness: &Value,
) -> TestResult {
    let merged = school_at(store, &later.id)?;
    assert_owned_claim(&merged, "470000000010", "10 Pending Rd")?;
    check!(eq; merged.classification, later.classification);
    check!(eq; merged.aliases, later.aliases);
    check!(eq; store.receipt_count()?, 2);
    check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?.len(), 0);
    let witnesses = witness::all(store)?;
    check!(eq; witnesses.len(), 2);
    check!(witnesses.contains(first_witness));
    let second = witnesses
        .iter()
        .find(|value| value.get("target").and_then(Value::as_str) == Some(later.id.as_str()))
        .ok_or("recomputed later target lacks application witness")?;
    check!(eq; second.get("application_version").and_then(Value::as_str), Some(APPLICATION_VERSION));
    check!(eq; second.get("mutation").and_then(Value::as_str), Some("schools"));
    check!(eq; second.get("scope_digest"), first_witness.get("scope_digest"));
    let previous = target_rows(partial, &later.id)?;
    let committed = target_rows(completed, &later.id)?;
    check!(eq; committed.len(), 2);
    check!(previous.iter().all(|row| committed.contains(row)));
    let added: physical::Rows = committed
        .into_iter()
        .filter(|row| !previous.contains(row))
        .collect();
    check!(eq; added.len(), 1);
    assert_application_binding(store, completed, second, &added)?;
    let applications = completed.applications()?;
    check!(partial
        .applications()?
        .iter()
        .all(|row| applications.contains(row)));
    let journal = completed.rows("journal")?;
    check!(partial
        .rows("journal")?
        .iter()
        .all(|row| journal.contains(row)));
    Ok(())
}

fn assert_application_binding(
    store: &Store,
    physical: &physical::Physical,
    application: &Value,
    added: &physical::Rows,
) -> TestResult {
    let payloads: Vec<CanonicalSchool> = added
        .iter()
        .map(|(_, value)| serde_json::from_slice(value))
        .collect::<Result<_, _>>()?;
    let effect = witness::effect(application, Table::Schools)?;
    check!(eq; effect.get("payload_digest").and_then(Value::as_str), Some(serialized_digest(&payloads)?.as_str()));
    check!(eq; effect.get("placement"), Some(&json!("unpartitioned")));
    check!(eq; effect.get("target"), application.get("target"));
    let digest = serialized_digest(application)?;
    let operation = format!("{OPERATION_PREFIX}{digest}");
    let receipt = store
        .receipt(&operation)?
        .ok_or("durable application lacks receipt")?;
    check!(eq; receipt.digest, digest);
    check!(eq; receipt.appended, 1);
    check!(eq; receipt.credited, false);
    let receipt_row = physical
        .rows("receipts")?
        .iter()
        .find(|(key, _)| key.as_slice() == operation.as_bytes())
        .ok_or("physical receipt key missing")?;
    check!(eq; serde_json::from_slice::<census_store::Receipt>(&receipt_row.1)?, receipt);
    let journal_key = format!("{WITNESS_PHASE}\0{operation}");
    let journal_row = physical
        .rows("journal")?
        .iter()
        .find(|(key, _)| key.as_slice() == journal_key.as_bytes())
        .ok_or("physical witness key missing")?;
    let envelope: Value = serde_json::from_slice(&journal_row.1)?;
    check!(eq; envelope.get("payload"), Some(application));
    Ok(())
}
