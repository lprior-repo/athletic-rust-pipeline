use super::super::super::map::Accumulator;
use super::*;
use census_domain::model::{
    CanonicalEvent, CanonicalMeet, CanonicalSchool, CompetitionLevel, EventKind, Evidence, Grade,
    SchoolYear, SourceRef, Sport,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use std::collections::{BTreeSet, HashMap};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn shared_names_keep_row_owners_separate_and_reuse_only_the_same_native_owner() -> TestResult {
    let state = UsJurisdiction::Iowa;
    let school = CanonicalSchool::new(state, "Example School", "example school").0;
    let meet = CanonicalMeet::new(
        Some(state),
        "Example Meet",
        "2026-04-01",
        CompetitionLevel::Unknown,
    );
    let event = CanonicalEvent::new(&meet.id, EventKind::Track1600m, Gender::Boys, None, None);
    let index = SchoolIndex::from_schools(std::slice::from_ref(&school));
    let mut accumulator = Accumulator::default();
    let mut stats = ResultStats::default();
    let mut resolved = HashMap::new();
    let cases = [
        ("timer-a", 0, None),
        ("timer-a", 1, None),
        ("timer-b", 0, None),
        ("timer-a", 2, Some(111)),
        ("timer-a", 3, Some(222)),
        ("timer-b", 99, Some(111)),
    ];
    let mut owners = Vec::new();
    for (provider, row, native_id) in cases {
        let source = SourceRef::new(
            "fixture",
            Some(format!("https://example.test/{provider}/event/7")),
        );
        let evidence = Evidence::parsed(source.clone(), "2026-04-01");
        let context = RowContext {
            meet: &meet,
            source: &source,
            evidence: &evidence,
            event_id: &event.id,
            kind: &EventKind::Track1600m,
            gender: Gender::Boys,
            round: None,
            sport: Sport::OutdoorTrack,
            school_year: SchoolYear::new(2026).ok_or("2026 school year")?,
            event_key: "event:7".to_owned(),
            provider,
            jurisdiction: state,
        };
        let identity = RowIdentity {
            name: "Alex Rivera",
            school_name: "Example School",
            grade: Grade::new(11).ok_or("junior grade")?,
            gender: Gender::Boys,
            an_athlete_id: native_id,
            timer_team_id: None,
            an_team_id: None,
        };
        let mut writer = Writer {
            index: &index,
            resolved: &mut resolved,
            stats: &mut stats,
            accumulator: &mut accumulator,
        };
        owners.push(
            map_identity(&mut writer, &context, &school.id, &identity, row)
                .ok_or("mapped row owner")?
                .athlete,
        );
    }
    check!(eq; owners[3], owners[5]);
    check!(eq; owners.iter().collect::<BTreeSet<_>>().len(), 5);
    check!(eq; accumulator.athletes.len(), 5);
    let candidate_keys: BTreeSet<_> = accumulator
        .athletes
        .values()
        .map(CanonicalAthlete::candidate_key)
        .collect();
    check!(eq; candidate_keys.len(), 1);
    let retained = accumulator
        .athletes
        .get(owners[3].as_str())
        .ok_or("native subject")?;
    let evidence_urls: BTreeSet<_> = retained
        .evidence
        .iter()
        .filter_map(|evidence| evidence.source.url.as_deref())
        .collect();
    check!(eq;
        evidence_urls,
        BTreeSet::from([
            "https://example.test/timer-a/event/7",
            "https://example.test/timer-b/event/7",
        ])
    );
    Ok(())
}
