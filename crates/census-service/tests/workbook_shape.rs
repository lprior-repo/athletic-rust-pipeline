#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use calamine::{open_workbook, Reader, Xlsx};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, CentiMetres, CentiSeconds, CompetitionLevel, EventKind, Evidence, Gender,
    GradYear, Mark, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_report::bests;
use census_report::export::ExportDataset;
use census_report::report::Scope;
use census_report::workbook::build;
use census_service::consolidate;
use census_store::{Store, Table};

#[test]
fn the_workbook_carries_the_scopes_the_bests_and_the_meet_inventory(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let day = "2026-09-21";

    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford", None);
    store.append(Table::Schools, &school)?;

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
        .published_graduations
        .push(census_domain::model::PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: SourceRef::id("wiaa_results"),
        });
    athlete
        .public_profile_urls
        .push("https://example.test/julian".to_string());
    store.append(Table::Athletes, &athlete)?;

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
    store.append(Table::Meets, &meet)?;
    let school_year = SchoolYear::new(2025).ok_or("invalid fixture season")?;
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
    store.append(Table::Teams, &team)?;

    for (kind, marks) in [
        (EventKind::Track400m, &[49.80_f64, 48.55, 49.10][..]),
        (EventKind::LongJump, &[6.10_f64, 6.42][..]),
    ] {
        let mut event = CanonicalEvent::new(&meet_id, kind.clone(), Gender::Boys, None, None);
        let event_id = event.id.clone();
        event
            .evidence
            .push(Evidence::parsed(SourceRef::new("wiaa_results", None), day));
        store.append(Table::Events, &event)?;
        for (attempt, mark) in marks.iter().enumerate() {
            let performance_date = format!("2026-06-{:02}", 6 + attempt);
            let value = if matches!(kind, EventKind::Track400m) {
                Mark::TimeSeconds(
                    CentiSeconds::try_from_seconds_f64(*mark).ok_or("invalid fixture time")?,
                )
            } else {
                Mark::DistanceMetres(
                    CentiMetres::try_from_metres_f64(*mark).ok_or("invalid fixture distance")?,
                )
            };
            let source_key = format!("test:{kind:?}:{mark}");
            let performance = CanonicalPerformance {
                id: CanonicalPerformance::mint(
                    &athlete.id,
                    &meet_id,
                    &kind,
                    &performance_date,
                    &source_key,
                ),
                athlete: athlete.id.clone(),
                team: team_id.clone(),
                event: event_id.clone(),
                meet: meet_id.clone(),
                date: performance_date,
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
            store.append(Table::Performances, &performance)?;
        }
    }

    consolidate(&store)?;
    let options = census_report::workbook::Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: Scope::Core,
        school_year: None,
    };
    let path = build(&store, &options)?;

    let mut book: Xlsx<_> = open_workbook(&path)?;
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
    check!(eq; names
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>(),
    expected);

    let dataset = ExportDataset::load(&store)?;
    let bests = bests::build_from_dataset(
        &dataset,
        &bests::Options {
            scope: Scope::Core,
            grad_year: Some(2027),
            limit: None,
        },
    );
    let sprint = bests
        .iter()
        .find(|row| row.key.event_kind == EventKind::Track400m)
        .ok_or("missing 400m best")?;
    check!(eq; sprint.value, 4_855);
    check!(eq; sprint.population.marks, 3);
    let jump = bests
        .iter()
        .find(|row| row.key.event_kind == EventKind::LongJump)
        .ok_or("missing long jump best")?;
    check!(eq; jump.value, 6_420_000);

    let range = book.worksheet_range("PRs")?;
    check!(eq; range.height(), 3);
    let headers = range.rows().next().ok_or("missing PR headers")?;
    let mark_column = headers
        .iter()
        .position(|cell| cell == "Mark Value")
        .ok_or("missing mark column")?;
    let unit_column = headers
        .iter()
        .position(|cell| cell == "Unit")
        .ok_or("missing unit column")?;
    let marks: std::collections::BTreeSet<_> = range
        .rows()
        .skip(1)
        .map(|row| (row[mark_column].to_string(), row[unit_column].to_string()))
        .collect();
    check!(eq; marks,
    std::collections::BTreeSet::from([
        ("48.55".to_string(), "s".to_string()),
        ("6.42".to_string(), "m".to_string()),
    ]));
    Ok(())
}
