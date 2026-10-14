use super::chunks::ChunkWriter;
use super::codec::{FrozenCodecError, FrozenCodecResult};
use super::inventory::StreamInventory;
use super::reader::ChunkReader;
use super::stream::StreamKind;
use census_domain::model::{
    AccessBlockKind, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, CentiMetres, CoachRole, CompetitionLevel,
    EventIdentity, EventKind, EventSpecification, Gender, GradYear, Mark, PerformanceIdentity,
    PerformanceResult, ReviewCase, ReviewVerdictRecord, SchoolId, SchoolYear,
    SourceAccessCondition, SourceIdentity, SourceNamespace, Sport, CONTACT_CONFLICT_FAMILY,
};
use census_domain::UsJurisdiction;
use census_store::{Entity, Store, Table};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(crate) struct Bundle {
    pub(crate) inventory: StreamInventory,
    pub(crate) objects: Vec<Vec<u8>>,
}

pub(crate) struct SeededStore {
    pub(crate) directory: tempfile::TempDir,
    pub(crate) store: Store,
}

pub(crate) fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, name.to_lowercase(), None).0
}

pub(crate) fn athlete(
    school: &SchoolId,
    name: &str,
    provider_id: &str,
) -> TestResult<CanonicalAthlete> {
    Ok(CanonicalAthlete::new(
        school,
        name,
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, provider_id),
    )?)
}

pub(crate) fn coach(school: &SchoolId, name: &str) -> CanonicalCoach {
    CanonicalCoach::new(school, name, None, Gender::Boys, CoachRole::HeadCoach)
}

pub(crate) fn bundle<T: Entity>(stream: StreamKind, rows: &[T]) -> TestResult<Bundle> {
    let mut writer = ChunkWriter::new(stream);
    let mut objects = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let ordinal = u64::try_from(index)?;
        if let Some(chunk) = writer.push(ordinal, row)? {
            check!(eq; chunk.index, u64::try_from(objects.len())?);
            objects.push(chunk.bytes);
        }
    }
    let (last, inventory) = writer.finish()?;
    if let Some(chunk) = last {
        check!(eq; chunk.index, u64::try_from(objects.len())?);
        objects.push(chunk.bytes);
    }
    Ok(Bundle { inventory, objects })
}

pub(crate) fn read_back<T: Entity>(
    stream: StreamKind,
    objects: &[Vec<u8>],
    inventory: &StreamInventory,
) -> TestResult<Vec<T>> {
    let mut rows = Vec::new();
    for (position, descriptor) in inventory.chunks().iter().enumerate() {
        let bytes = objects
            .get(position)
            .ok_or("every chunk descriptor needs its object")?;
        check!(eq; u64::try_from(bytes.len())?, descriptor.bytes);
        let mut reader = ChunkReader::open(stream, bytes, descriptor.first_ordinal)?;
        let mut expected = descriptor.first_ordinal;
        let mut first_key: Option<String> = None;
        let mut last_key: Option<String> = None;
        while let Some(decoded) = reader.next_record::<T>()? {
            check!(eq; decoded.ordinal, expected);
            expected = expected.checked_add(1).ok_or("ordinal overflow")?;
            if first_key.is_none() {
                first_key = Some(decoded.entity_id.clone());
            }
            last_key = Some(decoded.entity_id.clone());
            rows.push(decoded.payload);
        }
        check!(eq; reader.rows(), descriptor.rows);
        check!(eq; first_key.as_deref(), Some(descriptor.first_key.as_str()));
        check!(eq; last_key.as_deref(), Some(descriptor.last_key.as_str()));
    }
    check!(eq; u64::try_from(rows.len())?, inventory.rows());
    Ok(rows)
}

pub(crate) fn contains(haystack: &[u8], needle: &str) -> bool {
    let needle = needle.as_bytes();
    if needle.is_empty() {
        return true;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

pub(crate) fn refusal<T>(result: FrozenCodecResult<T>) -> TestResult<FrozenCodecError> {
    match result {
        Ok(_) => Err("expected a typed refusal".into()),
        Err(error) => Ok(error),
    }
}

pub(crate) fn refusal_what(error: &FrozenCodecError) -> Option<&'static str> {
    match error {
        FrozenCodecError::BudgetExceeded { what, .. } => Some(what),
        FrozenCodecError::UnsupportedVersion { what, .. } => Some(what),
        _ => None,
    }
}

pub(crate) fn seeded_store() -> TestResult<SeededStore> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Frozen High",
        "frozen high",
        None,
    );
    store.append(Table::Schools, &school)?;
    let athlete_a = athlete(&school_id, "Ada Runner", "1")?;
    let athlete_b = athlete(&school_id, "Bea Runner", "2")?;
    store.append(Table::Athletes, &athlete_a)?;
    store.append(Table::Athletes, &athlete_b)?;
    let season = SchoolYear::new(2026).ok_or("fixture season")?;
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(&school_id, Sport::OutdoorTrack, Gender::Girls, season),
        school: school_id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Girls,
        school_year: season,
        level: None,
        source_identities: Vec::new(),
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Teams, &team)?;
    let coach = coach(&school_id, "Dana Coach");
    store.append(Table::Coaches, &coach)?;
    store.append(Table::Coaches, &coach)?;
    let meet = CanonicalMeet::new_checked(
        Some(UsJurisdiction::Wisconsin),
        "Frozen Invite",
        "2026-05-01",
        CompetitionLevel::Invitational,
    )?;
    store.append(Table::Meets, &meet)?;
    let kind = EventKind::Discus;
    let specification = EventSpecification::from_published_label("Discus Throw", &kind)?;
    let event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind,
            gender: Gender::Girls,
            division: None,
            round: None,
        },
        specification,
    )?;
    store.append(Table::Events, &event)?;
    let performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete_a.id,
            event: &event.id,
            meet: &meet.id,
            date: "2026-05-01",
            source_key: "fixture-1",
        },
        PerformanceResult {
            team: &team.id,
            mark: Mark::DistanceMetres(CentiMetres::new(4572)),
            adjustment: None,
            wind_mps: None,
            place: Some(1),
        },
    )?;
    store.append(Table::Performances, &performance)?;
    let case = ReviewCase::pending(
        CONTACT_CONFLICT_FAMILY,
        "schools:frozen-high",
        "Frozen High",
        "fixture review",
    );
    store.replace(Table::ReviewCases, &case)?;
    store.replace(
        Table::IdentityVerdicts,
        &ReviewVerdictRecord {
            id: format!("verdict:{}", case.id),
            case_id: case.id.clone(),
            subject_id: case.subject_id.clone(),
            family: CONTACT_CONFLICT_FAMILY.to_string(),
            kind: "same_school".to_string(),
            field: String::new(),
            value: String::new(),
            accepted: true,
            confidence: 200,
            rationale: "fixture".to_string(),
            reviewer: "fixture".to_string(),
            observed_at: "2026-05-01".to_string(),
            member_ids: Vec::new(),
        },
    )?;
    store.replace(
        Table::SourceAccess,
        &SourceAccessCondition::new(
            "fixture_source",
            "example.invalid",
            AccessBlockKind::RateLimited,
            429,
            "2026-05-01T00:00:00Z",
            "fixture",
        ),
    )?;
    Ok(SeededStore { directory, store })
}

#[path = "corruption_fixtures.rs"]
mod corruption_fixtures;

#[path = "corruption_frames_tests.rs"]
mod corruption_frames_tests;

#[path = "corruption_graph_tests.rs"]
mod corruption_graph_tests;

#[path = "corruption_index_tests.rs"]
mod corruption_index_tests;
