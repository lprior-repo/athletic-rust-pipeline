use super::super::chunks::ChunkWriter;
use super::super::codec::{FrozenCodecError, CHUNK_BYTES_LIMIT};
use super::super::fixtures::{
    athlete, bundle, coach, contains, read_back, refusal, school, TestResult,
};
use super::super::inventory::StreamInventory;
use super::super::line::{decode_line, encode_line, DecodedLine};
use super::super::stream::StreamKind;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalTeam, CompetitionLevel, EventIdentity, EventKind, EventSpecification, ExactSeconds,
    Gender, Mark, PerformanceIdentity, PerformanceResult, ReviewVerdictRecord, SchoolYear, Sport,
};
use census_domain::UsJurisdiction;

#[test]
fn frozen_codec_line_round_trips_typed_payload_and_entity_id() -> TestResult {
    let school = school("Round Trip High");
    let runner = athlete(&school.id, "Ada Runner", "1")?;
    let line = encode_line(7, &runner)?;
    check!(contains(&line, "{\"ordinal\":7,\"entity_id\":\""));
    let decoded: DecodedLine<CanonicalAthlete> = decode_line(&line)?;
    check!(eq; decoded.ordinal, 7);
    check!(eq; decoded.entity_id, runner.id.as_str());
    check!(eq; decoded.payload, runner);
    Ok(())
}

#[test]
fn frozen_codec_line_refuses_a_payload_id_that_disagrees_with_entity_id() -> TestResult {
    let school = school("Mismatch High");
    let runner = athlete(&school.id, "Ada Runner", "1")?;
    let line = String::from_utf8(encode_line(0, &runner)?)?;
    let foreign = line.replacen(
        &format!("\"entity_id\":\"{}\"", runner.id.as_str()),
        "\"entity_id\":\"ath_foreign\"",
        1,
    );
    check!(foreign != line);
    let error = refusal(decode_line::<CanonicalAthlete>(foreign.as_bytes()))?;
    check!(matches!(error, FrozenCodecError::CorruptArtifact { .. }));
    Ok(())
}

#[test]
fn frozen_codec_line_pins_ordinal_field_order_and_exact_decimal() -> TestResult {
    let school = school("Fixture Union High");
    let runner = athlete(&school.id, "Ada Runner", "1")?;
    let meet = CanonicalMeet::new_checked(
        Some(UsJurisdiction::Wisconsin),
        "Fixture Invite",
        "2026-05-01",
        CompetitionLevel::Invitational,
    )?;
    let kind = EventKind::Discus;
    let specification = EventSpecification::from_published_label("Discus Throw", &kind)?;
    let event = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind,
            gender: Gender::Girls,
            division: None,
            round: None,
        },
        specification,
    )?);
    let team = CanonicalTeam::mint(
        &school.id,
        Sport::OutdoorTrack,
        Gender::Girls,
        SchoolYear::new(2026).ok_or("fixture season")?,
    );
    let performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &runner.id,
            event: &event.id,
            meet: &meet.id,
            date: "2026-05-01",
            source_key: "fixture-1",
        },
        PerformanceResult {
            team: &team,
            mark: Mark::TimeSeconds(ExactSeconds::parse("12.34")?),
            adjustment: None,
            wind_mps: None,
            place: Some(1),
        },
    )?;
    let line = encode_line(4, &performance)?;
    check!(contains(
        &line,
        &format!(
            "{{\"ordinal\":4,\"entity_id\":\"{}\",\"payload\":",
            performance.id.as_str()
        )
    ));
    check!(contains(
        &line,
        "\"mark\":{\"TimeSeconds\":{\"nanoseconds\":12340000000,\"precision\":2}}"
    ));
    let decoded: DecodedLine<CanonicalPerformance> = decode_line(&line)?;
    check!(eq; decoded.ordinal, 4);
    check!(eq; decoded.payload, performance);
    Ok(())
}

#[test]
fn frozen_codec_chunk_writer_never_splits_a_record_and_round_trips_order() -> TestResult {
    let school = school("Split High");
    let mut rows: Vec<CanonicalAthlete> = (0..20)
        .map(|index| {
            let mut row = athlete(
                &school.id,
                &format!("Runner {index:02}"),
                &format!("{index}"),
            )
            .map_err(|e| format!("athlete construction: {e}"))?;
            row.canonical_name = "n".repeat(200 * 1024);
            row.known_names = vec![row.canonical_name.clone()];
            Ok(row)
        })
        .collect::<Result<Vec<_>, String>>()?;
    rows.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));
    let bundle = bundle(StreamKind::Athletes, &rows)?;
    check!(bundle.objects.len() >= 2);
    for object in &bundle.objects {
        check!(u64::try_from(object.len())? <= CHUNK_BYTES_LIMIT);
        check!(eq; object.last(), Some(&b'\n'));
    }
    let decoded: Vec<CanonicalAthlete> =
        read_back(StreamKind::Athletes, &bundle.objects, &bundle.inventory)?;
    check!(eq; decoded, rows);
    check!(eq; bundle.inventory.rows(), 20);
    Ok(())
}

#[test]
fn frozen_codec_coach_observations_keep_repeated_ids_in_scan_order() -> TestResult {
    let school = school("Repeat High");
    let first = coach(&school.id, "Dana Coach");
    let mut second = first.clone();
    second.professional_email = Some("dana@example.invalid".to_string());
    let mut rows = vec![first, second];
    rows.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));
    check!(eq;
        rows.first().map(|row| row.id.as_str()),
        rows.last().map(|row| row.id.as_str())
    );
    let bundle = bundle(StreamKind::CoachObservations, &rows)?;
    let decoded: Vec<CanonicalCoach> = read_back(
        StreamKind::CoachObservations,
        &bundle.objects,
        &bundle.inventory,
    )?;
    check!(eq; decoded, rows);
    check!(eq; bundle.inventory.rows(), 2);
    Ok(())
}

#[test]
fn frozen_codec_canonical_streams_refuse_duplicate_and_decreasing_ids() -> TestResult {
    let school = school("Order High");
    let first = athlete(&school.id, "First Runner", "1")?;
    let second = athlete(&school.id, "Second Runner", "2")?;
    let (greater, smaller) = if first.id.as_str() > second.id.as_str() {
        (first, second)
    } else {
        (second, first)
    };
    let mut writer = ChunkWriter::new(StreamKind::Athletes);
    writer.push(0, &greater)?;
    let duplicate = refusal(writer.push(1, &greater))?;
    check!(matches!(
        duplicate,
        FrozenCodecError::CorruptArtifact { .. }
    ));
    let decreasing = refusal(writer.push(1, &smaller))?;
    check!(matches!(
        decreasing,
        FrozenCodecError::CorruptArtifact { .. }
    ));
    check!(eq; writer.rows(), 1);
    let (_, inventory) = writer.finish()?;
    check!(eq; inventory.rows(), 1);
    Ok(())
}

#[test]
fn frozen_codec_empty_stream_carries_an_explicit_inventory() -> TestResult {
    let rows: Vec<ReviewVerdictRecord> = Vec::new();
    let bundle = bundle(StreamKind::Verdicts, &rows)?;
    check!(bundle.objects.is_empty());
    check!(eq; bundle.inventory.rows(), 0);
    check!(eq; bundle.inventory.bytes(), 0);
    check!(bundle.inventory.chunks().is_empty());
    let encoded = bundle.inventory.encode()?;
    let decoded = StreamInventory::decode(&encoded)?;
    check!(eq; decoded, bundle.inventory);
    Ok(())
}
