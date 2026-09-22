use super::*;
use crate::store::Table;
use calamine::{open_workbook, Reader, Xlsx};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, CompetitionLevel, EventKind, Evidence, Gender, GradYear, Mark, SchoolYear,
    SourceRef, Sport,
};
use census_domain::UsJurisdiction;

#[test]
fn the_workbook_carries_the_scopes_the_bests_and_the_meet_inventory() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let day = "2026-09-21";

    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    store.append(Table::Schools, &school).unwrap();

    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
    );
    athlete
        .evidence
        .push(Evidence::parsed(SourceRef::new("wiaa_results", None), day));
    athlete
        .public_profile_urls
        .push("https://example.test/julian".to_string());
    store.append(Table::Athletes, &athlete).unwrap();

    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "WIAA Division 3 State",
        "2026-06-06",
        CompetitionLevel::State,
    );
    let meet_id = meet.id.clone();
    meet.evidence
        .push(Evidence::parsed(SourceRef::new("wiaa_results", None), day));
    store.append(Table::Meets, &meet).unwrap();
    let team_id = CanonicalTeam::mint(
        &school_id,
        Sport::OutdoorTrack,
        Gender::Boys,
        SchoolYear(2026),
    );

    for (kind, marks) in [
        (EventKind::Track400m, &[49.80_f64, 48.55, 49.10][..]),
        (EventKind::LongJump, &[6.10_f64, 6.42][..]),
    ] {
        let mut event = CanonicalEvent::new(&meet_id, kind.clone(), Gender::Boys, None, None);
        let event_id = event.id.clone();
        event
            .evidence
            .push(Evidence::parsed(SourceRef::new("wiaa_results", None), day));
        store.append(Table::Events, &event).unwrap();
        for mark in marks {
            let value = if matches!(kind, EventKind::Track400m) {
                Mark::TimeSeconds(*mark)
            } else {
                Mark::DistanceMetres(*mark)
            };
            let source_key = format!("test:{kind:?}:{mark}");
            let performance = CanonicalPerformance {
                id: CanonicalPerformance::mint(&athlete.id, &meet_id, &kind, day, &source_key),
                athlete: athlete.id.clone(),
                team: team_id.clone(),
                event: event_id.clone(),
                meet: meet_id.clone(),
                date: day.to_string(),
                mark: value,
                wind_mps: None,
                place: None,
                heat: None,
                round: None,
                timing: None,
                observed_grade: None,
                evidence: vec![Evidence::parsed(SourceRef::new("wiaa_results", None), day)],
                source_key,
            };
            store.append(Table::Performances, &performance).unwrap();
        }
    }

    crate::census::consolidate(&store).unwrap();
    let options = Options::default();
    let path = build(&store, &options).unwrap();
    assert!(path.exists(), "the workbook exists at {}", path.display());

    let mut book: Xlsx<_> = open_workbook(&path).unwrap();
    let names = book.sheet_names().to_vec();

    // The objective's sheets come first, in its own order (§50 Athletes, §51 PRs, §52 Performances_001,
    // §53 Coaches, §54 Schools/Meets/Sources/Coverage/Conflicts/Review/Run Metrics), then every legacy
    // census sheet the objective does not supersede, ending with the renamed `Meets summary`.
    let objective = [
        "Athletes",
        "PRs",
        "Performances_001",
        "Coaches",
        "Schools",
        "Meets",
        "Sources",
        "Coverage",
        "Conflicts",
        "Review",
        "Run Metrics",
    ];
    let legacy = [
        "Goal & method",
        "Summary",
        "By state - core",
        "By state - all sources",
        "Athletic.net marginal",
        "Best results",
        "Meets summary",
        "Evidence mix",
        "Method notes",
    ];
    let expected: Vec<String> = objective
        .iter()
        .chain(legacy.iter())
        .map(|name| (*name).to_string())
        .collect();
    assert_eq!(names, expected, "the published sheet list, in order");
    assert_eq!(
        names.len(),
        names
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        "no sheet name is written twice"
    );

    // The best-mark reduction picked the fastest 400 and the longest jump.
    let bests = bests::build(
        &store,
        &bests::Options {
            scope: Scope::Core,
            grad_year: Some(2027),
            limit: None,
        },
    )
    .unwrap();
    let sprint = bests
        .iter()
        .find(|row| row.event.contains("400m"))
        .expect("a 400m best");
    assert_eq!(sprint.best_mark, "48.55");
    assert_eq!(sprint.marks_in_event, 3);
    let jump = bests
        .iter()
        .find(|row| row.event.contains("LongJump"))
        .expect("a long jump best");
    assert_eq!(jump.best_mark, "6.42 m");
    assert!(jump.place.is_none());

    let range = book.worksheet_range("Best results").unwrap();
    assert_eq!(
        range.get_value((0, 0)).map(|v| v.to_string()),
        Some("Athlete".to_string())
    );
    let header: Vec<String> = (0..16)
        .map(|col| {
            range
                .get_value((0, col))
                .map(|v| v.to_string())
                .unwrap_or_default()
        })
        .collect();
    assert!(header.contains(&"Best mark".to_string()));
    assert!(header.contains(&"Profile URL".to_string()));
}
