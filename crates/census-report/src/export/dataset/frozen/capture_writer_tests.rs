use super::super::fixtures::{coach, school, TestResult};
use super::*;
use crate::report::ExportResourceError;

fn padded_coach(name: &str, bytes: usize) -> TestResult<CanonicalCoach> {
    let mut row = coach(&school("Synthetic").id, name);
    let original = serde_json::to_vec(&row)?.len();
    let padding = bytes.checked_sub(original).ok_or("synthetic row budget")?;
    row.name.push_str(&"x".repeat(padding));
    check!(eq; serde_json::to_vec(&row)?.len(), bytes);
    Ok(row)
}

fn advance_to_coaches(sink: &mut StreamSink<'_>, view: &StoreSnapshot<'_>) -> TestResult {
    sink.athletes(view)?;
    sink.merged::<CanonicalSchool>(StreamKind::Schools, Table::Schools, view)?;
    sink.merged::<CanonicalTeam>(StreamKind::Teams, Table::Teams, view)?;
    Ok(())
}

#[test]
fn merged_writer_resource_error_survives_store_callback_bridge() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let school = school("Synthetic");
    let mut candidates = [
        coach(&school.id, "A"),
        coach(&school.id, "B"),
        coach(&school.id, "C"),
    ];
    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    let [mut row, next, last] = candidates;
    let initial = serde_json::to_vec(&row)?.len();
    let padding = census_store::MAX_OBSERVATION_RECORD_BYTES
        .checked_sub(initial)
        .ok_or("synthetic row size")?;
    row.name.push_str(&"x".repeat(padding));
    let expected = serde_json::to_vec(
        &serde_json::json!({"ordinal":0,"entity_id":row.entity_id(),"payload":&row}),
    )?
    .len()
    .checked_add(1)
    .ok_or("synthetic line size")?;
    store.append(Table::Coaches, &row)?;
    store.append(Table::Coaches, &next)?;
    store.append(Table::Coaches, &serde_json::json!({"id": last.id}))?;
    let view = store.snapshot();
    let objects = tempfile::tempdir()?;
    let mut sink = StreamSink::new(objects.path());
    advance_to_coaches(&mut sink, &view)?;
    let result = sink.merged::<CanonicalCoach>(StreamKind::Coaches, Table::Coaches, &view);
    check!(
        matches!(result, Err(ReportError::Resource(ExportResourceError::RowTooLarge { what: "frozen line including its newline", bytes, max: 1_048_576 })) if bytes == u64::try_from(expected)?)
    );
    check!(eq; sink.streams.len(), 3);
    Ok(())
}

#[test]
fn observed_writer_io_error_survives_store_callback_bridge() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    for name in ["A", "B", "C", "D", "E", "F"] {
        store.append(Table::Coaches, &padded_coach(name, 900_000)?)?;
    }
    let view = store.snapshot();
    let directory = tempfile::tempdir()?;
    let objects = directory.path().join("objects");
    std::fs::create_dir(&objects)?;
    let mut sink = StreamSink::new(&objects);
    advance_to_coaches(&mut sink, &view)?;
    sink.merged::<CanonicalCoach>(StreamKind::Coaches, Table::Coaches, &view)?;
    std::fs::rename(&objects, directory.path().join("preserved-objects"))?;
    std::fs::write(&objects, b"synthetic blocker")?;
    let result = sink.observed(&view);
    check!(
        matches!(result, Err(ReportError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotADirectory)
    );
    check!(eq; sink.streams.len(), 4);
    check!(directory.path().join("preserved-objects").is_dir());
    Ok(())
}
