use calamine::{open_workbook, Reader, Xlsx};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, CentiMetres, CentiSeconds, CompetitionLevel, EventKind, Evidence, Gender,
    GradYear, Mark, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_report::bests;
use census_report::report::Scope;
use census_report::workbook::build;
use census_service::consolidate;
use census_store::{Store, Table};

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
        SourceIdentity::new(SourceNamespace::Other("fixture".to_string()), "athlete-1"),
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
    meet.sports.push(Sport::OutdoorTrack);
    let meet_id = meet.id.clone();
    meet.evidence
        .push(Evidence::parsed(SourceRef::new("wiaa_results", None), day));
    store.append(Table::Meets, &meet).unwrap();
    let school_year = SchoolYear::new(2025).expect("2025 is a school year");
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(&school_id, Sport::OutdoorTrack, Gender::Boys, school_year),
        school: school_id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Boys,
        school_year,
        level: None,
        source_identities: Vec::new(),
        evidence: vec![Evidence::parsed(SourceRef::id("wiaa_results"), day)],
        retained_conflicts: Vec::new(),
    };
    let team_id = team.id.clone();
    store.append(Table::Teams, &team).unwrap();

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
                Mark::TimeSeconds(
                    CentiSeconds::try_from_seconds_f64(*mark).expect("fixture is in range"),
                )
            } else {
                Mark::DistanceMetres(
                    CentiMetres::try_from_metres_f64(*mark).expect("fixture is in range"),
                )
            };
            let source_key = format!("test:{kind:?}:{mark}");
            let performance = CanonicalPerformance {
                id: CanonicalPerformance::mint(
                    &athlete.id,
                    &meet_id,
                    &kind,
                    &meet.date,
                    &source_key,
                ),
                athlete: athlete.id.clone(),
                team: team_id.clone(),
                event: event_id.clone(),
                meet: meet_id.clone(),
                date: meet.date.clone(),
                mark: value,
                wind_mps: None,
                place: None,
                heat: None,
                round: None,
                timing: None,
                observed_grade: None,
                evidence: vec![Evidence::parsed(SourceRef::new("wiaa_results", None), day)],
                source_key,
                source_athlete: athlete.source.clone(),
                retained_conflicts: Vec::new(),
            };
            store.append(Table::Performances, &performance).unwrap();
        }
    }

    consolidate(&store).unwrap();
    let options = census_report::workbook::Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: Scope::Core,
        school_year: None,
    };
    let path = build(&store, &options).unwrap();

    let mut book: Xlsx<_> = open_workbook(&path).unwrap();
    let names = book.sheet_names().to_vec();

    let published = [
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
    let expected: std::collections::BTreeSet<_> = published.iter().copied().collect();
    assert_eq!(
        names
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        expected
    );

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
        .find(|row| row.key.event_kind == EventKind::Track400m)
        .expect("a 400m best");
    assert_eq!(sprint.value, 4_855);
    assert_eq!(sprint.population.marks, 3);
    let jump = bests
        .iter()
        .find(|row| row.key.event_kind == EventKind::LongJump)
        .expect("a long jump best");
    assert_eq!(jump.value, 6_420_000);

    let range = book.worksheet_range("PRs").unwrap();
    assert_eq!(range.height(), 3);
    let headers = range.rows().next().unwrap();
    let mark_column = headers
        .iter()
        .position(|cell| cell.to_string() == "Mark Value")
        .unwrap();
    let unit_column = headers
        .iter()
        .position(|cell| cell.to_string() == "Unit")
        .unwrap();
    let marks: std::collections::BTreeSet<_> = range
        .rows()
        .skip(1)
        .map(|row| (row[mark_column].to_string(), row[unit_column].to_string()))
        .collect();
    assert_eq!(
        marks,
        std::collections::BTreeSet::from([
            ("48.55".to_string(), "s".to_string()),
            ("6.42".to_string(), "m".to_string()),
        ])
    );
}
