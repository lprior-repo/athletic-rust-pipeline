use super::*;
use census_crawl::participation_evidence::recorded::record_provider_school_intake;
use census_crawl::Recording;
use census_domain::model::SourceObservation;

const OP_B: &str = "controlled-revision-refused-packet-b";
const INGEST_B: &str = "controlled-revision-physical-ingestion-b";

#[test]
fn missing_target_case_refuses_review_atomically() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let source_a = fixture::install(&fixture, "22222", "77", Sport::OutdoorTrack, true)?;
    fixture.accept(&source_a.athlete.id)?;
    issuance::review_revision(
        &fixture,
        &source_a,
        "controlled-revision-original-source",
        issuance::CaptureRevision::RowLocalV3,
    )?;

    let packet_b = build_packet_b(&source_a.packet)?;
    let recording = Recording::new();
    record_provider_school_intake(&fixture.store, &recording, &packet_b)?;
    let mut recorded = recording.drain();
    let observations = recorded
        .rows
        .iter()
        .position(|batch| batch.table == Table::SourceObservations)
        .ok_or("SourceObservations batch absent from ingestion")?;
    recorded.rows.remove(observations);
    let prepared = census_crawl::recording::prepare_recorded(recorded, INGEST_B)?;
    let applied = prepared.apply(&fixture.store)?;
    check!(matches!(applied, census_store::Application::Written(_)));

    let school = fixture
        .store
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .find(|row| row.id == source_a.school.id)
        .ok_or("merged school row")?;
    let program = fixture
        .store
        .scan::<CanonicalTeam>(Table::Teams)?
        .into_iter()
        .find(|row| row.id == source_a.program.id)
        .ok_or("merged program row")?;
    let capture_b = fixture::Capture {
        packet: packet_b,
        school,
        program,
        athlete: source_a.athlete.clone(),
        roster_body: source_a.roster_body.clone(),
    };

    let head_before = authority_head(&fixture.store)?;
    let rows_before = physical_rows(&fixture.store)?;
    let seq_before = fixture.store.snapshot().sequence();
    let review = issuance::review_revision(
        &fixture,
        &capture_b,
        OP_B,
        issuance::CaptureRevision::RowLocalV3,
    );
    check!(review.is_err());
    check!(eq; authority_head(&fixture.store)?, head_before);
    check!(eq; physical_rows(&fixture.store)?, rows_before);
    check!(eq; fixture.store.snapshot().sequence(), seq_before);
    remove_sources(fixture, tempfile::tempdir()?)?;
    Ok(())
}

#[test]
fn same_key_contradiction_refuses_review_atomically() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let source_a = fixture::install(&fixture, "22222", "77", Sport::OutdoorTrack, true)?;
    fixture.accept(&source_a.athlete.id)?;
    issuance::review_revision(
        &fixture,
        &source_a,
        "controlled-revision-original-source",
        issuance::CaptureRevision::RowLocalV3,
    )?;

    let packet_b = build_packet_b(&source_a.packet)?;
    let recording = Recording::new();
    record_provider_school_intake(&fixture.store, &recording, &packet_b)?;
    let prepared = census_crawl::recording::prepare_recorded(recording.drain(), INGEST_B)?;
    let applied = prepared.apply(&fixture.store)?;
    check!(matches!(applied, census_store::Application::Written(_)));

    let school = fixture
        .store
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .find(|row| row.id == source_a.school.id)
        .ok_or("merged school row")?;
    let program = fixture
        .store
        .scan::<CanonicalTeam>(Table::Teams)?
        .into_iter()
        .find(|row| row.id == source_a.program.id)
        .ok_or("merged program row")?;
    let capture_b = fixture::Capture {
        packet: packet_b,
        school,
        program,
        athlete: source_a.athlete.clone(),
        roster_body: source_a.roster_body.clone(),
    };

    let mut changed = capture_b.packet.read()?.observation().clone();
    changed.observed_name = "Contradictory Name".into();
    let row = SourceObservation::School(changed);
    fixture.store.append(Table::SourceObservations, &row)?;

    let head_before = authority_head(&fixture.store)?;
    let rows_before = physical_rows(&fixture.store)?;
    let seq_before = fixture.store.snapshot().sequence();
    let review = issuance::review_revision(
        &fixture,
        &capture_b,
        OP_B,
        issuance::CaptureRevision::RowLocalV3,
    );
    check!(review.is_err());
    check!(eq; authority_head(&fixture.store)?, head_before);
    check!(eq; physical_rows(&fixture.store)?, rows_before);
    check!(eq; fixture.store.snapshot().sequence(), seq_before);
    remove_sources(fixture, tempfile::tempdir()?)?;
    Ok(())
}
