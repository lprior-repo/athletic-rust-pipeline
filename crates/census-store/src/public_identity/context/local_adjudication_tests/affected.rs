use super::*;
use crate::public_identity_tests::support::retained_state;
use crate::Table;
use census_domain::model::{CanonicalPerformance, RelayResult};

#[test]
fn affected_inventory_is_exact_sorted_unique_and_includes_all_raw_result_variants() -> TestResult {
    let fixture = merged()?;
    let canonical = fixture.births.first().ok_or("birth")?.public_id();
    let inventory = fixture
        .writer
        .inspect_local_adjudication(&fixture.store, canonical)?;
    assert_eq!(
        inventory.affected_record_keys(),
        results::expected_keys(&fixture)?
    );
    assert!(!inventory.affected_context().is_empty());
    assert_eq!(
        results::raw::<CanonicalPerformance>(&fixture.store, Table::Performances)?.len(),
        5
    );
    assert_eq!(
        results::raw::<RelayResult>(&fixture.store, Table::RelayResults)?.len(),
        5
    );
    let [_, _, unrelated] = &fixture.candidates;
    let merged_performances: Vec<CanonicalPerformance> = fixture.store.scan(Table::Performances)?;
    let merged_relays: Vec<RelayResult> = fixture.store.scan(Table::RelayResults)?;
    let performance = merged_performances
        .iter()
        .find(|row| row.source_key == "raw-variant")
        .ok_or("raw performance variant")?;
    let relay = merged_relays
        .iter()
        .find(|row| row.source_key == "raw-variant")
        .ok_or("raw relay variant")?;
    assert_eq!(performance.athlete, unrelated.cast());
    assert_eq!(
        relay.members.first().ok_or("relay member")?.athlete,
        Some(unrelated.cast())
    );
    for unrelated in results::unrelated_keys(&fixture)? {
        assert!(!inventory.affected_record_keys().contains(&unrelated));
    }
    let inspected = fixture
        .writer
        .inspect_local_adjudication(&fixture.store, canonical)?;
    assert_eq!(inspected.affected_context(), inventory.affected_context());
    assert_eq!(
        inspected.affected_record_keys(),
        inventory.affected_record_keys()
    );
    Ok(())
}

#[test]
fn unrelated_existing_affected_keys_are_refused_before_any_commit() -> TestResult {
    let fixture = merged()?;
    for key in results::unrelated_keys(&fixture)? {
        let mut request = request(&fixture)?;
        request.affected_record_keys.push(key);
        request.affected_record_keys.sort();
        refused_without_commit(&fixture, request)?;
    }
    Ok(())
}

#[test]
fn nonexistent_affected_keys_are_refused_before_any_commit() -> TestResult {
    let fixture = merged()?;
    for table in [
        "athletes",
        "source_observations",
        "performances",
        "relay_results",
    ] {
        let mut request = request(&fixture)?;
        request
            .affected_record_keys
            .push(format!("{table}:synthetic-does-not-exist"));
        request.affected_record_keys.sort();
        refused_without_commit(&fixture, request)?;
    }
    Ok(())
}

#[test]
fn omitted_athlete_source_performance_or_relay_is_refused_before_any_commit() -> TestResult {
    let fixture = merged()?;
    let expected = results::expected_keys(&fixture)?;
    for key in expected {
        let mut request = request(&fixture)?;
        request.affected_record_keys.retain(|held| held != &key);
        refused_without_commit(&fixture, request)?;
    }
    Ok(())
}

fn refused_without_commit(fixture: &ReviewedFixture, request: LocalIdentitySplitV1) -> TestResult {
    let before = retained_state(&fixture.store)?;
    let head = fixture.writer.head(&fixture.store)?;
    let sequence = fixture.store.snapshot().sequence();
    let result = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-invalid-affected-inventory",
        request,
        fixture.store.snapshot().revision(),
        &head,
    );
    assert!(matches!(
        result,
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "manual affected record inventory"
        )))
    ));
    assert_eq!(fixture.writer.head(&fixture.store)?, head);
    assert_eq!(fixture.store.snapshot().sequence(), sequence);
    assert_eq!(retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn merge_and_manual_split_preserve_every_original_raw_fact() -> TestResult {
    let fixture = ready_for_merge()?;
    results::install(&fixture)?;
    let before = results::retained_facts(&fixture.store)?;
    fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "synthetic-retained-merge",
        &fixture.request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    assert_eq!(results::retained_facts(&fixture.store)?, before);
    let request = request(&fixture)?;
    assert_eq!(
        request.affected_record_keys,
        results::expected_keys(&fixture)?
    );
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-retained-split",
        request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let (bytes, digest) = approval(&prepared)?;
    fixture
        .writer
        .apply_local_adjudication(&fixture.store, prepared, bytes, digest)?;
    assert_eq!(results::retained_facts(&fixture.store)?, before);
    Ok(())
}
