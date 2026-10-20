use super::*;
use census_crawl::participation_evidence::provider_school::ProviderSchoolPacketV1;
use census_crawl::participation_evidence::recorded::record_provider_school_intake;
use census_crawl::Recording;
use census_domain::model::SourceObservation;

mod refusals;

const OP_A: &str = "controlled-revision-original-source";
const OP_B: &str = "controlled-revision-revised-packet";
const INGEST_B: &str = "controlled-revision-physical-ingestion-b";

fn build_packet_b(packet_a: &ProviderSchoolPacketV1) -> TestResult<ProviderSchoolPacketV1> {
    let mut transport = serde_json::to_value(packet_a)?;
    let started = transport
        .pointer_mut("/directory/provenance/started_at")
        .ok_or("directory provenance start")?;
    let original = chrono::DateTime::parse_from_rfc3339(
        started
            .as_str()
            .ok_or("directory start is not a date string")?,
    )?;
    *started = serde_json::to_value(original - chrono::Duration::seconds(1))?;
    let packet_b: ProviderSchoolPacketV1 = serde_json::from_value(transport)?;
    check!(ne; packet_b.reference()?, packet_a.reference()?);
    check!(eq; packet_b.roster().body_sha256(), packet_a.roster().body_sha256());
    check!(eq; packet_b.roster().requested_url(), packet_a.roster().requested_url());
    check!(eq; packet_b.roster().acquired_at(), packet_a.roster().acquired_at());
    Ok(packet_b)
}

#[test]
fn controlled_revision_same_source_school_packet_bound_locator_both_retained() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let source_a = fixture::install(&fixture, "22222", "77", Sport::OutdoorTrack, true)?;
    fixture.accept(&source_a.athlete.id)?;
    let approval_a = issuance::review_revision(
        &fixture,
        &source_a,
        OP_A,
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let digest_a = super::super::hash(&approval_a);

    let objects = tempfile::tempdir()?;
    std::fs::create_dir(objects.path().join("g0"))?;
    std::fs::create_dir(objects.path().join("g1"))?;
    let dataset_g0 = capture(&fixture, &objects.path().join("g0"))?;
    let expected = Expected {
        athlete: source_a.athlete.id.clone(),
        current_school: source_a.school.id.clone(),
        school_name: source_a.school.name.clone(),
        case: Case::Resolved,
    };
    let before_g0 = witness(&dataset_g0, &expected)?;

    let original_rows = physical_rows(&fixture.store)?;
    let capture_b = ingest_revision(&fixture, &source_a)?;
    let after_ingestion = physical_rows(&fixture.store)?;
    for original in &original_rows {
        check!(after_ingestion.contains(original));
    }
    issuance::review_revision(
        &fixture,
        &capture_b,
        OP_B,
        issuance::CaptureRevision::RowLocalV3,
    )?;

    let dataset_after = capture(&fixture, &objects.path().join("g1"))?;
    let before_after = witness(&dataset_after, &expected)?;
    check!(eq; before_after.get("public"), before_g0.get("public"));
    check!(eq; before_after.get("school"), before_g0.get("school"));
    check!(ne; before_after.get("generation"), before_g0.get("generation"));

    let head_before_replay = fixture.store.snapshot().sequence();
    let authority_before_replay = authority_head(&fixture.store)?;
    let replay_a = fixture
        .store
        .replay_roster_source_review_v1(OP_A, &digest_a)?;
    let census_store::Application::Repeated(_) = replay_a else {
        return Err("A replay did not repeat".into());
    };
    check!(eq; fixture.store.snapshot().sequence(), head_before_replay);
    check!(eq; authority_head(&fixture.store)?, authority_before_replay);
    remove_sources(fixture, objects)?;
    Ok(())
}

fn ingest_revision(
    fixture: &fixture::Fixture,
    source_a: &fixture::Capture,
) -> TestResult<fixture::Capture> {
    let packet = build_packet_b(&source_a.packet)?;
    let recording = Recording::new();
    record_provider_school_intake(&fixture.store, &recording, &packet)?;
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
    Ok(fixture::Capture {
        packet,
        school,
        program,
        athlete: source_a.athlete.clone(),
        roster_body: source_a.roster_body.clone(),
    })
}

fn physical_rows(store: &Store) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut rows = Vec::new();
    let snapshot = store.snapshot();
    snapshot.for_each_physical_observation_bounded::<CanonicalSchool>(
        Table::Schools,
        fixture::budget(),
        |_, key, raw, _| {
            rows.push((key.to_vec(), raw.to_vec()));
            Ok(())
        },
    )?;
    snapshot.for_each_physical_observation_bounded::<SourceObservation>(
        Table::SourceObservations,
        fixture::budget(),
        |_, key, raw, _| {
            rows.push((key.to_vec(), raw.to_vec()));
            Ok(())
        },
    )?;
    Ok(rows)
}

fn authority_head(store: &Store) -> TestResult<Vec<u8>> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    Ok(current.authority_head_bytes(&mut grant)?.to_vec())
}
