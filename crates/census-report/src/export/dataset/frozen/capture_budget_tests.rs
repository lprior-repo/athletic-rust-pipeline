use super::super::fixtures::{seeded_store, TestResult};
use super::*;

fn physical_bytes<T: Entity>(snapshot: &StoreSnapshot<'_>, table: Table) -> TestResult<u64> {
    let mut total = 0_u64;
    snapshot.for_each_observation(table, |row: T| {
        let raw = serde_json::to_vec(&row).map_err(|source| StoreError::Json {
            detail: "synthetic observation bytes".to_owned(),
            source,
        })?;
        let bytes = table
            .file()
            .len()
            .checked_add(1)
            .and_then(|n| n.checked_add(row.entity_id().len()))
            .and_then(|n| n.checked_add(1 + 8))
            .and_then(|n| n.checked_add(raw.len()))
            .ok_or(StoreError::CounterOverflow)?;
        total = total
            .checked_add(u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)?)
            .ok_or(StoreError::CounterOverflow)?;
        Ok(())
    })?;
    Ok(total)
}

#[test]
fn capture_second_stream_cannot_reset_completed_stream_byte_debit() -> TestResult {
    let seeded = seeded_store()?;
    let snapshot = seeded.store.snapshot();
    let school = physical_bytes::<CanonicalSchool>(&snapshot, Table::Schools)?;
    let team = physical_bytes::<CanonicalTeam>(&snapshot, Table::Teams)?;
    let available = team.checked_sub(1).ok_or("synthetic team size")?;
    let directory = tempfile::tempdir()?;
    let mut sink = StreamSink::new(directory.path());
    sink.admission = admission::CaptureAdmission::fixture(
        school.checked_add(available).ok_or("synthetic grant")?,
    );
    sink.athletes(&snapshot)?;
    sink.merged::<CanonicalSchool>(StreamKind::Schools, Table::Schools, &snapshot)?;
    let error = sink.merged::<CanonicalTeam>(StreamKind::Teams, Table::Teams, &snapshot);
    check!(
        matches!(error, Err(ReportError::Store(StoreError::BudgetExceeded { what: "capture encoded bytes", needed, available: actual })) if needed == team && actual == available)
    );
    check!(eq; sink.streams.len(), 2);
    check!(eq; sink.admission.budget().bytes, available);
    Ok(())
}

#[test]
fn capture_observed_coaches_pay_again_for_every_physical_version() -> TestResult {
    let seeded = seeded_store()?;
    let snapshot = seeded.store.snapshot();
    let school = physical_bytes::<CanonicalSchool>(&snapshot, Table::Schools)?;
    let team = physical_bytes::<CanonicalTeam>(&snapshot, Table::Teams)?;
    let coach = physical_bytes::<CanonicalCoach>(&snapshot, Table::Coaches)?;
    check!(eq; coach % 2, 0);
    let base = school
        .checked_add(team)
        .and_then(|n| n.checked_add(coach))
        .ok_or("synthetic grant")?;
    for extra in [0, coach] {
        let directory = tempfile::tempdir()?;
        let mut sink = StreamSink::new(directory.path());
        sink.admission =
            admission::CaptureAdmission::fixture(base.checked_add(extra).ok_or("synthetic grant")?);
        sink.athletes(&snapshot)?;
        sink.merged::<CanonicalSchool>(StreamKind::Schools, Table::Schools, &snapshot)?;
        sink.merged::<CanonicalTeam>(StreamKind::Teams, Table::Teams, &snapshot)?;
        sink.merged::<CanonicalCoach>(StreamKind::Coaches, Table::Coaches, &snapshot)?;
        let observed = sink.observed(&snapshot);
        if extra == 0 {
            check!(
                matches!(observed, Err(ReportError::Store(StoreError::BudgetExceeded { what: "capture encoded bytes", needed, available: 0 })) if needed == coach / 2)
            );
            check!(eq; sink.streams.len(), 4);
        } else {
            observed?;
            check!(eq; sink.streams.len(), 5);
        }
        check!(eq; sink.admission.budget().bytes, 0);
    }
    Ok(())
}
