use super::*;
use crate::bests;
use crate::report::Scope;
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use census_domain::model::{
    normalize_name, AthleteId, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, CentiMetres, CentiSeconds, CoachRole,
    CoachTenure, CoachTenureEvidence, CompetitionLevel, EventId, EventKind, Evidence, Gender,
    GradYear, Grade, Mark, MeetId, ObservedGrade, SchoolId, SchoolYear, SourceIdentity,
    SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use rust_xlsxwriter::Workbook;

mod identity;
mod postal;
mod summaries;
const DAY: &str = "2026-09-20";

struct Fixture {
    dir: tempfile::TempDir,
    store: Store,
    julian: AthleteId,
    nadia: AthleteId,
    wi_school: SchoolId,
}

fn evidence(source: &str, url: Option<&str>) -> Vec<Evidence> {
    vec![Evidence::parsed(
        SourceRef::new(source, url.map(str::to_string)),
        DAY,
    )]
}

fn school(store: &Store, state: UsJurisdiction, name: &str) -> SchoolId {
    let (mut school, id) = CanonicalSchool::new(state, name, normalize_name(name));
    school.athletics_website = Some(format!("https://{}.test/athletics", name.to_lowercase()));
    school.city = Some(name.to_string());
    school.evidence = evidence("wiaa_results", Some("https://wiaa.test/schools"));
    store.append(Table::Schools, &school).unwrap();
    id
}

fn julian(store: &Store, school: &SchoolId) -> (AthleteId, SourceIdentity) {
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "wi-999")
        .with_url("https://wi.milesplit.com/athletes/999");
    let id = CanonicalAthlete::mint(
        school,
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
        &source,
    );
    for _ in 0..2 {
        let mut athlete = CanonicalAthlete::new(
            school,
            "Julian Aguilera",
            GradYear::CO2027,
            Gender::Boys,
            source.clone(),
        );
        athlete.observed_grades.push(ObservedGrade {
            grade: Grade::new(11).unwrap(),
            school_year: SchoolYear::new(2025).expect("2025 is a season"),
            source: SourceRef::new("wiaa_results", None),
        });
        athlete.sports = vec![Sport::OutdoorTrack, Sport::CrossCountry];
        athlete.public_profile_urls = vec!["https://example.test/julian".to_string()];
        athlete.add_identity(
            SourceIdentity::new(SourceNamespace::athletic_net("athlete"), "123")
                .with_url("https://www.athletic.net/athlete/123"),
        );
        athlete.evidence = evidence("wiaa_results", None);
        store.append(Table::Athletes, &athlete).unwrap();
    }
    (id, source)
}

fn nadia(store: &Store, school: &SchoolId) -> AthleteId {
    let mut athlete = CanonicalAthlete::new(
        school,
        "Nadia Berger",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::Other("fixture".to_owned()), "nadia"),
    );
    athlete.sports = vec![Sport::CrossCountry];
    athlete.evidence = evidence("mshsl_results", None);
    store.append(Table::Athletes, &athlete).unwrap();
    athlete.id
}

fn younger(store: &Store, school: &SchoolId) {
    let mut athlete = CanonicalAthlete::new(
        school,
        "Owen Clarke",
        GradYear::new(2028).expect("2028 is a class"),
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::Other("fixture".to_owned()), "younger"),
    );
    athlete.evidence = evidence("wiaa_results", None);
    store.append(Table::Athletes, &athlete).unwrap();
}

fn meet(
    store: &Store,
    state: UsJurisdiction,
    name: &str,
    date: &str,
    level: CompetitionLevel,
    sport: Sport,
) -> MeetId {
    let mut meet = CanonicalMeet::new(Some(state), name, date, level);
    meet.sports = vec![sport];
    meet.evidence = evidence("wiaa_results", Some("https://wiaa.test/meets"));
    let id = meet.id.clone();
    store.append(Table::Meets, &meet).unwrap();
    id
}

fn event(store: &Store, meet: &MeetId, kind: EventKind) -> EventId {
    let mut event = CanonicalEvent::new(meet, kind, Gender::Boys, None, None);
    event.evidence = evidence("wiaa_results", None);
    let id = event.id.clone();
    store.append(Table::Events, &event).unwrap();
    id
}

fn performance(store: &Store, context: &PerformanceRow<'_>, mark: Mark, source: &str, url: &str) {
    let school_year = SchoolYear::from_date(context.date).unwrap();
    let source_athlete = SourceIdentity::new(
        SourceNamespace::Other(source.to_string()),
        format!("{source}-999"),
    );
    let team = CanonicalTeam::mint(
        context.school,
        Sport::OutdoorTrack,
        Gender::Boys,
        school_year,
    );
    store
        .append(
            Table::Teams,
            &CanonicalTeam {
                id: team.clone(),
                school: context.school.clone(),
                sport: Sport::OutdoorTrack,
                gender: Gender::Boys,
                school_year,
                level: None,
                source_identities: Vec::new(),
                evidence: evidence(source, Some(url)),
                retained_conflicts: Vec::new(),
            },
        )
        .unwrap();
    let source_key = format!("{source}:{}:{}", context.date, mark.raw());
    store
        .append(
            Table::Performances,
            &CanonicalPerformance {
                id: CanonicalPerformance::mint(
                    context.athlete,
                    context.meet,
                    context.kind,
                    context.date,
                    &source_key,
                ),
                athlete: context.athlete.clone(),
                team,
                event: context.event.clone(),
                meet: context.meet.clone(),
                date: context.date.to_string(),
                mark,
                wind_mps: None,
                place: None,
                heat: None,
                round: None,
                timing: None,
                observed_grade: None,
                evidence: evidence(source, Some(url)),
                source_key,
                source_athlete: Some(source_athlete),
                retained_conflicts: Vec::new(),
            },
        )
        .unwrap();
}

struct PerformanceRow<'a> {
    athlete: &'a AthleteId,
    school: &'a SchoolId,
    meet: &'a MeetId,
    event: &'a EventId,
    kind: &'a EventKind,
    date: &'a str,
}

fn coach(
    store: &Store,
    school: &SchoolId,
    name: &str,
    sport: Option<Sport>,
    role: CoachRole,
    email: &str,
) {
    let mut coach = CanonicalCoach::new(school, name, sport, Gender::Mixed, role);
    coach.professional_email = Some(email.to_string());
    coach.evidence = evidence("coach_contacts_csv", Some("https://contacts.test/schools"));
    coach.tenure_evidence = vec![current_tenure()];
    store.append(Table::Coaches, &coach).unwrap();
}

fn personal_coach(store: &Store, school: &SchoolId) {
    let mut coach = CanonicalCoach::new(
        school,
        "Morgan Personal",
        Some(Sport::IndoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    coach.personal_email = Some("morgan@gmail.com".to_string());
    coach.evidence = evidence("coach_contacts_csv", Some("https://contacts.test/schools"));
    coach.tenure_evidence = vec![current_tenure()];
    store.append(Table::Coaches, &coach).unwrap();
}

fn current_tenure() -> CoachTenureEvidence {
    CoachTenureEvidence {
        tenure: CoachTenure::Current {
            school_year: SchoolYear::new(2026).unwrap(),
        },
        source: SourceRef::new(
            "synthetic_directory",
            Some("https://contacts.test/schools".into()),
        ),
        source_sha256: "a".repeat(64),
        retrieved_at: "2026-09-20T00:00:00Z".into(),
        statement: "Synthetic academic-year appointment".into(),
    }
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let wi = school(&store, UsJurisdiction::Wisconsin, "Abbotsford");
    let mn = school(&store, UsJurisdiction::Minnesota, "Ada-Borup");
    let (julian, _) = julian(&store, &wi);
    let nadia = nadia(&store, &mn);
    younger(&store, &wi);

    let state = meet(
        &store,
        UsJurisdiction::Wisconsin,
        "WIAA Division 3 State",
        "2026-06-06",
        CompetitionLevel::State,
        Sport::OutdoorTrack,
    );
    let invite = meet(
        &store,
        UsJurisdiction::Wisconsin,
        "Abbotsford Invitational",
        "2026-05-01",
        CompetitionLevel::Invitational,
        Sport::OutdoorTrack,
    );
    let sprint = event(&store, &state, EventKind::Track400m);
    let sprint_invite = event(&store, &invite, EventKind::Track400m);
    let jump = event(&store, &invite, EventKind::LongJump);
    let relay = event(&store, &state, EventKind::Relay4x400);

    for (meet, event, kind, date, mark, source, url) in [
        (
            &state,
            &sprint,
            &EventKind::Track400m,
            "2026-06-06",
            Mark::TimeSeconds(CentiSeconds::new(4980)),
            "wiaa_results",
            "https://wiaa.test/results/state",
        ),
        (
            &state,
            &sprint,
            &EventKind::Track400m,
            "2026-06-06",
            Mark::TimeSeconds(CentiSeconds::new(4971)),
            "pttiming_live",
            "https://pttiming.test/live/state",
        ),
        (
            &invite,
            &sprint_invite,
            &EventKind::Track400m,
            "2026-05-01",
            Mark::TimeSeconds(CentiSeconds::new(4855)),
            "wiaa_results",
            "https://wiaa.test/results/invite",
        ),
        (
            &invite,
            &jump,
            &EventKind::LongJump,
            "2026-05-01",
            Mark::DistanceMetres(CentiMetres::new(762)),
            "athleticlive_athletes",
            "https://athleticlive.test/athletes/999",
        ),
        (
            &state,
            &relay,
            &EventKind::Relay4x400,
            "2026-06-06",
            Mark::TimeSeconds(CentiSeconds::new(31950)),
            "wiaa_results",
            "https://wiaa.test/results/state",
        ),
    ] {
        performance(
            &store,
            &PerformanceRow {
                athlete: &julian,
                school: &wi,
                meet,
                event,
                kind,
                date,
            },
            mark,
            source,
            url,
        );
    }

    coach(
        &store,
        &wi,
        "Paula Johnson",
        Some(Sport::OutdoorTrack),
        CoachRole::HeadCoach,
        "pj@abbotsford.test",
    );
    coach(
        &store,
        &wi,
        "Teresa Johnson",
        Some(Sport::CrossCountry),
        CoachRole::AssistantCoach,
        "tj@abbotsford.test",
    );
    coach(
        &store,
        &mn,
        "Erik Lund",
        Some(Sport::CrossCountry),
        CoachRole::HeadCoach,
        "elund@adaborup.test",
    );

    Fixture {
        dir,
        store,
        julian,
        nadia,
        wi_school: wi,
    }
}

fn recruiting(store: &Store, scope: Scope, grad_year: Option<i16>) -> Recruiting {
    let dataset = crate::export::ExportDataset::load(store).unwrap();
    let derivation = crate::report::Derivation::of(&dataset, scope, grad_year);
    let prs = bests::build_from_dataset(
        &dataset,
        &bests::Options {
            scope,
            grad_year,
            limit: None,
        },
    );
    Recruiting::of(&derivation, SchoolYear::new(2026).unwrap(), prs).unwrap()
}

fn verified_publication(store: &Store, expected_athletes: u64) {
    let options = crate::workbook::Options {
        school_year: Some(SchoolYear::new(2026).unwrap()),
        ..crate::workbook::Options::default()
    };
    let published = crate::workbook::build(store, &options).unwrap();
    let dataset = crate::export::ExportDataset::load(store).unwrap();
    let verified = crate::workbook::verify::verify_frozen(&published, &dataset, &options).unwrap();
    assert_eq!(verified.mapped_athletes, expected_athletes);
}

fn written(fixture: &Fixture) -> (Xlsx<std::io::BufReader<std::fs::File>>, std::path::PathBuf) {
    written_scope(fixture, Scope::Core)
}

fn written_scope(
    fixture: &Fixture,
    scope: Scope,
) -> (Xlsx<std::io::BufReader<std::fs::File>>, std::path::PathBuf) {
    let projection = recruiting(&fixture.store, scope, Some(2027));
    let path = fixture.dir.path().join("recruiting.xlsx");
    let mut book = Workbook::new();
    projection.write_athletes(&mut book, &path).unwrap();
    projection.write_prs(&mut book, &path).unwrap();
    projection.write_coaches(&mut book, &path).unwrap();
    book.save(&path).unwrap();
    (open_workbook(&path).unwrap(), path)
}

fn text(range: &Range<Data>, row: usize, column: usize) -> String {
    let row = u32::try_from(row).expect("the fixture sheet fits u32 rows");
    let column = u32::try_from(column).expect("the fixture sheet fits u32 columns");
    range
        .get_value((row, column))
        .map(|value| value.to_string())
        .unwrap_or_default()
}

fn row_of(range: &Range<Data>, key: &str) -> usize {
    row_where(range, |row| text(range, row, 0) == key)
}

fn row_of_event(range: &Range<Data>, athlete: &str, event: &str) -> usize {
    row_where(range, |row| {
        text(range, row, 0) == athlete && text(range, row, 7) == event
    })
}

fn column_of(range: &Range<Data>, header: &str) -> usize {
    (0..range.width())
        .find(|column| text(range, 0, *column) == header)
        .unwrap_or_else(|| panic!("the sheet publishes the {header} column"))
}

fn column_carries(range: &Range<Data>, column: usize, value: &str) -> bool {
    (1..range.height()).any(|row| text(range, row, column) == value)
}

fn row_where(range: &Range<Data>, matches: impl Fn(usize) -> bool) -> usize {
    (1..range.height())
        .find(|row| matches(*row))
        .expect("the sheet publishes the row")
}

fn sheet(book: &mut Xlsx<std::io::BufReader<std::fs::File>>, name: &str) -> Range<Data> {
    book.worksheet_range(name).unwrap()
}

#[test]
fn the_three_recruiting_sheets_are_named_after_the_objective() {
    let fixture = fixture();
    let (book, path) = written(&fixture);
    let mut names = book.sheet_names().to_vec();
    names.sort();
    assert_eq!(names, vec!["Athletes", "Coaches", "PRs"]);
    assert!(path.exists());
}

#[test]
fn the_athletes_sheet_publishes_the_objective_columns_and_the_stored_facts() {
    let fixture = fixture();
    let (mut book, _) = written(&fixture);
    let range = sheet(&mut book, "Athletes");
    let row = row_of(&range, fixture.julian.as_str());
    let expected = [
        ("Name", "Julian Aguilera"),
        ("Gender", "Boys"),
        ("Graduation Year", "2027"),
        ("Observed School Year", "2025-26"),
        ("State", "WI"),
        ("School", "Abbotsford"),
        ("School ID", fixture.wi_school.as_str()),
        ("School City", "Abbotsford"),
        ("TF", "yes"),
        ("XC", "yes"),
        ("Indoor", ""),
        ("Outdoor", "yes"),
        ("Event list", "400m"),
    ];
    for (header, value) in expected {
        assert_eq!(
            text(&range, row, column_of(&range, header)),
            value,
            "{header}"
        );
    }
}

#[test]
fn the_prs_sheet_retains_shared_winners_units_dates_and_provenance() {
    let fixture = fixture();
    let (mut book, _) = written(&fixture);
    let range = sheet(&mut book, "PRs");

    let dataset = crate::export::ExportDataset::load(&fixture.store).unwrap();
    let canonical = bests::build_from_dataset(
        &dataset,
        &bests::Options {
            scope: Scope::Core,
            grad_year: Some(2027),
            limit: None,
        },
    );
    assert_eq!(
        range.height() - 1,
        canonical.len(),
        "one row per core-scope athlete/event"
    );

    let sprint = row_of_event(&range, fixture.julian.as_str(), "Track400m");
    assert_eq!(text(&range, sprint, 1), "Julian Aguilera");
    assert_eq!(text(&range, sprint, 2), "Boys");
    assert_eq!(text(&range, sprint, 3), "Abbotsford");
    assert_eq!(text(&range, sprint, 4), "WI");
    assert_eq!(text(&range, sprint, 5), "2027");
    assert_eq!(text(&range, sprint, 6), "Track");
    assert_eq!(
        text(&range, sprint, 9),
        "48.55",
        "the fastest of the three marks"
    );
    assert_eq!(text(&range, sprint, 10), "48.55");
    assert_eq!(text(&range, sprint, 11), "s");
    assert_eq!(text(&range, sprint, 12), "", "no wind was stored");
    assert_eq!(text(&range, sprint, 13), "2026-05-01");
    assert_eq!(text(&range, sprint, 14), "Abbotsford Invitational");
    assert_eq!(text(&range, sprint, 15), "", "no place was stored");
    assert_eq!(text(&range, sprint, 16), "https://wiaa.test/results/invite");
    assert_eq!(
        text(&range, sprint, 17),
        "2",
        "two sources report this event"
    );
    assert_eq!(
        text(&range, sprint, 18),
        "WIAA Division 3 State: 49.71 | 49.80",
        "the losing marks stay attached to the winner"
    );
    assert_eq!(text(&range, sprint, 19), "outdoor");
    assert_eq!(text(&range, sprint, 20), "na");
    assert_eq!(text(&range, sprint, 21), "unknown");
    assert!(text(&range, sprint, 22).starts_with("perf_"));
    assert!(text(&range, sprint, 23).starts_with("meet_"));
    assert_eq!(text(&range, sprint, 24), "wiaa_results:2026-05-01:time");

    let (mut all, _) = written_scope(&fixture, Scope::AllSources);
    let all_range = sheet(&mut all, "PRs");
    assert_eq!(
        all_range.height(),
        range.height() + 1,
        "the athleticlive jump PR joins the sheet only outside the core scope"
    );
    let jump = row_of_event(&all_range, fixture.julian.as_str(), "LongJump");
    assert_eq!(text(&all_range, jump, 6), "Field");
    assert_eq!(text(&all_range, jump, 9), "7.62 m");
    assert_eq!(text(&all_range, jump, 10), "7.62");
    assert_eq!(text(&all_range, jump, 11), "m");
    assert_eq!(text(&all_range, jump, 17), "1");
    assert_eq!(text(&all_range, jump, 18), "", "one report cannot conflict");
}

#[test]
fn the_coaches_sheet_publishes_the_school_contact_graph() {
    let fixture = fixture();
    let (mut book, _) = written(&fixture);
    let range = sheet(&mut book, "Coaches");
    let coach = column_of(&range, "Coach");
    let row = row_where(&range, |row| text(&range, row, coach) == "Paula Johnson");
    let expected = [
        ("Sport", Sport::OutdoorTrack.stable_key()),
        ("Role", CoachRole::HeadCoach.stable_key()),
        ("Professional Email", "pj@abbotsford.test"),
    ];
    for (header, value) in expected {
        assert_eq!(
            text(&range, row, column_of(&range, header)),
            value,
            "{header}"
        );
    }
}

#[test]
fn an_athlete_without_a_performance_is_published_as_identity_only() {
    let fixture = fixture();
    let (mut book, _) = written(&fixture);
    let range = sheet(&mut book, "Athletes");
    let row = row_of(&range, fixture.nadia.as_str());
    assert_eq!(text(&range, row, column_of(&range, "Event list")), "");
    assert_eq!(
        text(&range, row, column_of(&range, "Headline PR summary")),
        ""
    );
}

#[test]
fn the_url_columns_follow_the_evidence_scope() {
    let fixture = fixture();
    let (mut book, _) = written_scope(&fixture, Scope::AllSources);
    let range = sheet(&mut book, "Athletes");
    let row = row_of(&range, fixture.julian.as_str());
    assert_eq!(
        text(&range, row, column_of(&range, "Athletic.net URL")),
        "https://www.athletic.net/athlete/123"
    );
    assert_eq!(
        text(&range, row, column_of(&range, "MileSplit URL")),
        "https://wi.milesplit.com/athletes/999"
    );
    assert_eq!(
        text(&range, row, column_of(&range, "Other profile URLs")),
        "https://example.test/julian"
    );
}

#[test]
fn an_unrelated_programmes_personal_address_is_retained_only_in_raw_coach_history() {
    let fixture = fixture();
    personal_coach(&fixture.store, &fixture.wi_school);
    let (mut book, _) = written(&fixture);
    let range = sheet(&mut book, "Coaches");
    let coach = column_of(&range, "Coach");
    let professional = column_of(&range, "Professional Email");
    let personal = column_of(&range, "Personal Email");
    assert!(
        !column_carries(&range, professional, "morgan@gmail.com"),
        "the personal address never becomes a professional contact"
    );
    let row = row_where(&range, |row| text(&range, row, coach) == "Morgan Personal");
    assert_eq!(text(&range, row, professional), "");
    assert_eq!(text(&range, row, personal), "morgan@gmail.com");
}

#[test]
fn the_run_audits_every_sheet_against_the_store_counts_behind_it() {
    let fixture = fixture();
    let projection = recruiting(&fixture.store, Scope::Core, Some(2027));
    let audit = projection.dataset.audit();
    assert_eq!(audit.store_athletes, 3);
    assert_eq!(audit.cohort_athletes, 2);
    assert_eq!(audit.pr_rows, 1);
    assert_eq!(audit.coach_rows, 3);
    assert_eq!(audit.contact_conflicts, 0);
}
