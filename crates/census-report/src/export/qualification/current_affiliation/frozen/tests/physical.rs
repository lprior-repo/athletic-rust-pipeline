use super::*;
use census_domain::model::{CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam};
use census_store::{Entity, ReadBudget, StoreSnapshot};

mod fixture_rows;

#[test]
fn shared_physical_inventory_replays_all_five_tables_with_original_duplicates() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    fixture_rows::append(&fixture)?;
    let directory = tempfile::tempdir()?;
    let objects = objects(&directory)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    verify::<CanonicalAthlete>(&dataset, &snapshot, Table::Athletes, &objects)?;
    verify::<CanonicalTeam>(&dataset, &snapshot, Table::Teams, &objects)?;
    verify::<CanonicalMeet>(&dataset, &snapshot, Table::Meets, &objects)?;
    verify::<CanonicalEvent>(&dataset, &snapshot, Table::Events, &objects)?;
    verify::<CanonicalPerformance>(&dataset, &snapshot, Table::Performances, &objects)?;
    Ok(())
}

fn verify<T: Entity>(
    dataset: &ExportDataset,
    snapshot: &StoreSnapshot<'_>,
    table: Table,
    objects: &std::path::Path,
) -> TestResult {
    let expected = snapshot.tables_digest(&[table])?;
    let mut original = Vec::new();
    let counts = snapshot.for_each_physical_observation_bounded(
        table,
        observation_budget(),
        |row: T, _, _, _| {
            original.push(serde_json::to_value(row).map_err(|source| {
                census_store::StoreError::Decode {
                    key: "fixture".into(),
                    source,
                }
            })?);
            Ok(())
        },
    )?;
    let descriptor = physical_observations::capture::<T>(
        dataset,
        snapshot,
        table,
        objects,
        observation_budget(),
    )?;
    check!(eq; descriptor.table(), table);
    check!(eq; descriptor.fingerprint(), expected);
    check!(eq; descriptor.counts(), counts);
    check!(eq; counts.decoded_rows, 2);
    let mut replayed = Vec::new();
    let replay_counts = physical_observations::visit(
        dataset,
        objects,
        &descriptor,
        &expected,
        observation_budget(),
        |row: T| {
            replayed.push(serde_json::to_value(row).map_err(|source| {
                census_store::StoreError::Decode {
                    key: "fixture".into(),
                    source,
                }
            })?);
            Ok(())
        },
    )?;
    check!(eq; replay_counts, counts);
    check!(eq; replayed, original);
    Ok(())
}

#[test]
fn shared_physical_budgets_refuse_before_callbacks_and_accept_exact_limits() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let directory = tempfile::tempdir()?;
    let objects = objects(&directory)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let expected = snapshot.tables_digest(&[Table::Athletes])?;
    let descriptor = physical_observations::capture::<CanonicalAthlete>(
        &dataset,
        &snapshot,
        Table::Athletes,
        &objects,
        observation_budget(),
    )?;
    let mut record_bytes = 0;
    snapshot.for_each_physical_observation_bounded(
        Table::Athletes,
        observation_budget(),
        |_: CanonicalAthlete, _, raw, _| {
            record_bytes = raw.len();
            Ok(())
        },
    )?;
    let exact = ReadBudget {
        rows: 1,
        bytes: descriptor.counts().encoded_bytes,
        record_bytes,
    };
    for budget in [
        ReadBudget { rows: 0, ..exact },
        ReadBudget {
            bytes: exact.bytes - 1,
            ..exact
        },
        ReadBudget {
            record_bytes: record_bytes - 1,
            ..exact
        },
    ] {
        let mut callbacks = 0;
        let replayed = physical_observations::visit(
            &dataset,
            &objects,
            &descriptor,
            &expected,
            budget,
            |_: CanonicalAthlete| {
                callbacks += 1;
                Ok(())
            },
        );
        check!(replayed.is_err());
        check!(eq; callbacks, 0);
        check!(physical_observations::capture::<CanonicalAthlete>(
            &dataset,
            &snapshot,
            Table::Athletes,
            &objects,
            budget
        )
        .is_err());
    }
    let mut callbacks = 0;
    let counts = physical_observations::visit(
        &dataset,
        &objects,
        &descriptor,
        &expected,
        exact,
        |_: CanonicalAthlete| {
            callbacks += 1;
            Ok(())
        },
    )?;
    check!(eq; counts, descriptor.counts());
    check!(eq; callbacks, 1);
    check!(eq; physical_observations::capture::<CanonicalAthlete>(&dataset, &snapshot, Table::Athletes, &objects, exact)?, descriptor);
    Ok(())
}

#[test]
fn physical_descriptor_cannot_cross_tables_stores_generations_or_selection() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let directory = tempfile::tempdir()?;
    let objects = objects(&directory)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let expected = snapshot.tables_digest(&[Table::Athletes])?;
    let descriptor = physical_observations::capture::<CanonicalAthlete>(
        &dataset,
        &snapshot,
        Table::Athletes,
        &objects,
        observation_budget(),
    )?;
    for field in [
        "store_identity",
        "snapshot_sequence",
        "input_generation",
        "input_digest",
        "source_digest",
        "profile_id",
        "as_of",
        "table",
        "fingerprint",
    ] {
        let mut wire = serde_json::to_value(&descriptor)?;
        *wire.get_mut(field).ok_or("physical descriptor field")? = match field {
            "snapshot_sequence" => serde_json::json!(snapshot.sequence() + 1),
            "as_of" => serde_json::json!("2026-10-08"),
            "table" => serde_json::json!("performances"),
            _ => serde_json::json!("a".repeat(64)),
        };
        let changed: physical_observations::PhysicalObservationDescriptor =
            serde_json::from_value(wire)?;
        check!(physical_observations::visit(
            &dataset,
            &objects,
            &changed,
            &expected,
            observation_budget(),
            |_: CanonicalAthlete| Ok(())
        )
        .is_err());
    }
    check!(physical_observations::visit(
        &dataset,
        &objects,
        &descriptor,
        &snapshot.tables_digest(&[Table::Teams])?,
        observation_budget(),
        |_: CanonicalAthlete| Ok(())
    )
    .is_err());
    check!(physical_observations::capture::<CanonicalAthlete>(
        &dataset,
        &snapshot,
        Table::Schools,
        &objects,
        observation_budget()
    )
    .is_err());
    let mut changed = fixture.dataset(&snapshot)?;
    changed.athletes.clear();
    check!(physical_observations::visit(
        &changed,
        &objects,
        &descriptor,
        &expected,
        observation_budget(),
        |_: CanonicalAthlete| Ok(())
    )
    .is_err());
    fixture
        .store
        .journal_done("after-physical-freeze", "changed", &true)?;
    check!(physical_observations::capture::<CanonicalAthlete>(
        &dataset,
        &fixture.store.snapshot(),
        Table::Athletes,
        &objects,
        observation_budget()
    )
    .is_err());
    check!(physical_observations::visit(
        &dataset,
        &objects,
        &descriptor,
        &expected,
        observation_budget(),
        |_: CanonicalAthlete| Ok(())
    )
    .is_ok());
    Ok(())
}
