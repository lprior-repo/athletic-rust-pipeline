use super::super::codec::sha256_hex;
use super::super::fixtures::TestResult;
use super::super::stream::StreamKind;
use super::manifest_fixtures::{fixture, Fixture};
use super::FrozenManifest;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, ReviewCase, ReviewVerdictRecord, SourceAccessCondition,
};
use census_store::Entity;

#[test]
fn frozen_manifest_round_trips_lineage_token_and_inventory_refs() -> TestResult {
    let fixture = fixture()?;
    let decoded = FrozenManifest::decode(&fixture.encoded)?;
    check!(eq; decoded, fixture.manifest);
    check!(eq; decoded.lineage, fixture.dataset.lineage);
    check!(eq; decoded.lineage.input_digest, fixture.dataset.lineage.input_digest);
    check!(eq;
        decoded.legacy_logical_digest.as_deref(),
        Some(fixture.streams.legacy.as_str())
    );
    check!(eq; decoded.codec, "census-frozen-jsonl-v2");
    check!(eq; decoded.codec_revision, 2);
    check!(eq; decoded.streams.len(), 12);
    check!(eq; decoded.source_fingerprints.len(), 13);
    check!(eq; decoded.input_token.protocol, 2);
    check!(eq;
        decoded.input_token.store_identity,
        super::super::store_identity(&fixture.store)?
    );
    check!(eq;
        decoded.input_token.snapshot_sequence,
        fixture.dataset.lineage.snapshot_sequence
    );
    check!(eq; decoded.input_token.as_of, fixture.dataset.lineage.generated_on);
    for kind in StreamKind::ALL {
        let bundle = fixture.streams.find(kind)?;
        let reference = decoded
            .stream(kind)
            .ok_or("every stream needs a manifest reference")?;
        check!(eq; reference.rows, bundle.inventory.rows());
        check!(eq; reference.bytes, bundle.inventory.bytes());
        let encoded = bundle.inventory.encode()?;
        check!(eq; reference.inventory.digest, sha256_hex(&encoded));
        check!(eq; reference.inventory.bytes, u64::try_from(encoded.len())?);
    }
    Ok(())
}

#[test]
fn frozen_manifest_decoded_streams_reproduce_every_dataset_record() -> TestResult {
    let fixture = fixture()?;
    let decoded = FrozenManifest::decode(&fixture.encoded)?;
    let athletes: Vec<CanonicalAthlete> = read(&fixture, &decoded, StreamKind::Athletes)?;
    check!(eq; athletes, fixture.dataset.athletes);
    let schools: Vec<CanonicalSchool> = read(&fixture, &decoded, StreamKind::Schools)?;
    check!(eq; schools, fixture.dataset.schools.values().cloned().collect::<Vec<_>>());
    let teams: Vec<CanonicalTeam> = read(&fixture, &decoded, StreamKind::Teams)?;
    check!(eq; teams, fixture.dataset.teams.values().cloned().collect::<Vec<_>>());
    let coaches: Vec<CanonicalCoach> = read(&fixture, &decoded, StreamKind::Coaches)?;
    check!(eq; coaches, fixture.dataset.coaches);
    let observations: Vec<CanonicalCoach> =
        read(&fixture, &decoded, StreamKind::CoachObservations)?;
    check!(eq; observations, fixture.dataset.coach_observations);
    check!(eq; observations.len(), 2);
    check!(eq;
        observations.first().map(|row| row.id.as_str()),
        observations.last().map(|row| row.id.as_str())
    );
    let events: Vec<CanonicalEvent> = read(&fixture, &decoded, StreamKind::Events)?;
    check!(eq; events, fixture.dataset.events);
    let meets: Vec<CanonicalMeet> = read(&fixture, &decoded, StreamKind::Meets)?;
    check!(eq; meets, fixture.dataset.meets);
    let performances: Vec<CanonicalPerformance> =
        read(&fixture, &decoded, StreamKind::Performances)?;
    check!(eq; performances, fixture.dataset.performances);
    let cases: Vec<ReviewCase> = read(&fixture, &decoded, StreamKind::ReviewCases)?;
    check!(eq; cases, fixture.dataset.review_cases);
    let access: Vec<SourceAccessCondition> = read(&fixture, &decoded, StreamKind::SourceAccess)?;
    check!(eq; access, fixture.dataset.source_access);
    let verdicts: Vec<ReviewVerdictRecord> = read(&fixture, &decoded, StreamKind::Verdicts)?;
    check!(eq; verdicts, fixture.dataset.verdicts);
    let decisions = fixture.streams.find(StreamKind::IdentityDecisions)?;
    check!(eq; decisions.inventory.rows(), 0);
    Ok(())
}

#[test]
fn frozen_manifest_legacy_logical_digest_matches_the_dataset_encoding() -> TestResult {
    let fixture = fixture()?;
    check!(eq; fixture.streams.legacy, fixture.dataset.lineage.input_digest);
    check!(eq;
        fixture.streams.legacy,
        census_domain::model::serialized_digest(&fixture.dataset)?
    );
    Ok(())
}

fn read<T: Entity>(
    fixture: &Fixture,
    manifest: &FrozenManifest,
    kind: StreamKind,
) -> TestResult<Vec<T>> {
    let bundle = fixture.streams.find(kind)?;
    let reference = manifest
        .stream(kind)
        .ok_or("every stream needs a manifest reference")?;
    check!(eq; reference.rows, bundle.inventory.rows());
    super::super::fixtures::read_back(kind, &bundle.objects, &bundle.inventory)
}
