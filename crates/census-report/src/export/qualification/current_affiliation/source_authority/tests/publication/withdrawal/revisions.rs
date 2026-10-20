use super::*;
use base64::Engine;
use census_crawl::participation_evidence::provider_school::ProviderSchoolPacketV1;
use census_crawl::participation_evidence::recorded::record_provider_school_intake;
use census_crawl::Recording;
use census_domain::model::SourceObservation;

mod cold;
mod frozen;
mod refusals;
mod retain;

const OP_A: &str = "controlled-revision-original-source";
const OP_B: &str = "controlled-revision-revised-packet";
const OP_A_RETURN: &str = "controlled-revision-returned-packet-a";
const INGEST_B: &str = "controlled-revision-physical-ingestion-b";
const INGEST_A: &str = "controlled-revision-physical-ingestion-a";

fn build_packet_b(packet_a: &ProviderSchoolPacketV1) -> TestResult<ProviderSchoolPacketV1> {
    let original = packet_a.directory();
    let transport = serde_json::to_value(original)?;
    let encoded = transport
        .get("body_base64")
        .and_then(Value::as_str)
        .ok_or("controlled directory body")?;
    let mut body = base64::engine::general_purpose::STANDARD.decode(encoded)?;
    body.push(b'\n');
    let directory = census_crawl::participation_evidence::RetainedHtmlCaptureV1::new(
        original.requested_url(),
        original.requested_url(),
        200,
        original.acquired_at(),
        &body,
        original.provenance().clone(),
    )?;
    let packet_b = ProviderSchoolPacketV1::new(directory, packet_a.roster().clone())?;
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
    let recording = Recording::new();
    record_provider_school_intake(&fixture.store, &recording, &source_a.packet)?;
    let recorded_a = recording.drain();
    let prepared_a = census_crawl::recording::prepare_recorded(recorded_a.clone(), INGEST_A)?;
    let applied_a = prepared_a.apply(&fixture.store)?;
    check!(matches!(applied_a, census_store::Application::Written(_)));
    fixture.accept(&source_a.athlete.id)?;
    let approval_a = issuance::review_revision(
        &fixture,
        &source_a,
        OP_A,
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let digest_a = super::super::hash(&approval_a);

    let objects = tempfile::tempdir()?;
    let archive = tempfile::tempdir()?;
    let output_g0 = tempfile::tempdir()?;
    let output_g1 = tempfile::tempdir()?;
    let output_g2 = tempfile::tempdir()?;
    std::fs::create_dir(objects.path().join("g0"))?;
    std::fs::create_dir(objects.path().join("g1"))?;
    std::fs::create_dir(objects.path().join("g2"))?;
    let dataset_g0 = capture(&fixture, &objects.path().join("g0"))?;
    let expected = Expected {
        athlete: source_a.athlete.id.clone(),
        current_school: source_a.school.id.clone(),
        school_name: source_a.school.name.clone(),
        case: Case::Resolved,
    };
    let frozen_g0 = frozen::freeze_and_check(
        &fixture,
        &dataset_g0,
        &archive.path().join("revision-g0.json"),
        &output_g0.path().join("publication"),
        &expected,
    )?;
    let before_g0 = &frozen_g0.witness;

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
    let frozen_g1 = frozen::freeze_and_check(
        &fixture,
        &dataset_after,
        &archive.path().join("revision-g1.json"),
        &output_g1.path().join("publication"),
        &expected,
    )?;
    let before_after = &frozen_g1.witness;
    check!(eq; before_after.get("public"), before_g0.get("public"));
    check!(eq; before_after.get("school"), before_g0.get("school"));
    check!(ne; frozen_g1.digest, frozen_g0.digest);
    check!(ne; frozen_g1.generation_digest, frozen_g0.generation_digest);
    files::proof_refusal(
        &fixture,
        &dataset_after,
        &frozen_g1.path,
        &objects.path().join("g1"),
        &output_g1.path().join("publication"),
        &frozen_g1.workbook,
    )?;

    let head_before_replay = fixture.store.snapshot().sequence();
    let authority_before_replay = authority_head(&fixture.store)?;
    let rows_before_replay = physical_rows(&fixture.store)?;
    let prepared_a = census_crawl::recording::prepare_recorded(recorded_a, INGEST_A)?;
    let replayed_a = prepared_a.apply(&fixture.store)?;
    check!(matches!(replayed_a, census_store::Application::Repeated(_)));
    let replay_a = fixture
        .store
        .replay_roster_source_review_v1(OP_A, &digest_a)?;
    let census_store::Application::Repeated(_) = replay_a else {
        return Err("A replay did not repeat".into());
    };
    check!(eq; fixture.store.snapshot().sequence(), head_before_replay);
    check!(eq; authority_head(&fixture.store)?, authority_before_replay);
    check!(eq; physical_rows(&fixture.store)?, rows_before_replay);
    let returned_a = current_capture(&fixture, &source_a, source_a.packet.clone())?;
    issuance::review_revision(
        &fixture,
        &returned_a,
        OP_A_RETURN,
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let dataset_returned = capture(&fixture, &objects.path().join("g2"))?;
    let frozen_g2 = frozen::freeze_and_check(
        &fixture,
        &dataset_returned,
        &archive.path().join("revision-g2.json"),
        &output_g2.path().join("publication"),
        &expected,
    )?;
    check!(eq; frozen_g2.witness.get("public"), before_g0.get("public"));
    check!(eq; frozen_g2.witness.get("school"), before_g0.get("school"));
    check!(ne; frozen_g2.digest, frozen_g1.digest);
    check!(ne; frozen_g2.generation_digest, frozen_g1.generation_digest);
    check!(eq; physical_rows(&fixture.store)?, rows_before_replay);
    drop(dataset_g0);
    drop(dataset_after);
    drop(dataset_returned);
    remove_sources(fixture, objects)?;
    cold::proof_refusal(&frozen_g1)?;
    cold::proof_refusal(&frozen_g2)?;
    frozen::reopen_and_check(&frozen_g0, &expected)?;
    frozen::reopen_and_check(&frozen_g1, &expected)?;
    frozen::reopen_and_check(&frozen_g2, &expected)?;
    retain::maybe_retain(
        archive,
        [output_g0, output_g1, output_g2],
        [&frozen_g0, &frozen_g1, &frozen_g2],
    )?;
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
    current_capture(fixture, source_a, packet)
}

fn current_capture(
    fixture: &fixture::Fixture,
    source_a: &fixture::Capture,
    packet: ProviderSchoolPacketV1,
) -> TestResult<fixture::Capture> {
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
