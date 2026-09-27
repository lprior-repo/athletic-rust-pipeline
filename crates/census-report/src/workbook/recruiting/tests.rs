
use super::*;
use crate::bests;
use crate::workbook::Options;
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use census_domain::model::{
    normalize_name, AthleteId, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, CentiMetres, CentiSeconds, CoachRole, CoachTenure, CoachTenureEvidence,
    CompetitionLevel, EventId, EventKind, Evidence, Gender, GradYear, Grade, Mark, MeetId,
    ObservedGrade, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

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
    let id = CanonicalAthlete::mint(school, "Julian Aguilera", GradYear::CO2027, Gender::Boys, &source);
    for _ in 0..2 {
        let mut athlete = CanonicalAthlete::new(school, "Julian Aguilera", GradYear::CO2027,
            Gender::Boys, source.clone());
        athlete.observed_grades.push(ObservedGrade {
            grade: Grade::new(11).unwrap(),
            school_year: SchoolYear::new(2025).expect("2025 is a season"),
            source: SourceRef::new("wiaa_results", None),
        });
        athlete.sports = vec![Sport::OutdoorTrack, Sport::CrossCountry];
        athlete.public_profile_urls = vec!["https://example.test/julian".to_string()];
        athlete.add_identity(SourceIdentity::new(
            SourceNamespace::athletic_net("athlete"), "123")
            .with_url("https://www.athletic.net/athlete/123"));
        athlete.evidence = evidence("wiaa_results", None);
        store.append(Table::Athletes, &athlete).unwrap();
    }
    (id, source)
}

fn nadia(store: &Store, school: &SchoolId) -> AthleteId {
    let mut athlete = CanonicalAthlete::new(school, "Nadia Berger", GradYear::CO2027,
        Gender::Girls, SourceIdentity::new(SourceNamespace::Other("fixture".to_owned()), "nadia"));
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
    let team = CanonicalTeam::mint(
        context.school,
        Sport::OutdoorTrack,
        Gender::Boys,
        school_year,
    );
    store.append(Table::Teams, &CanonicalTeam {
        id: team.clone(), school: context.school.clone(), sport: Sport::OutdoorTrack,
        gender: Gender::Boys, school_year, level: None, source_identities: Vec::new(),
        evidence: evidence(source, Some(url)), retained_conflicts: Vec::new(),
    }).unwrap();
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
                source_athlete: context.source_athlete.clone(),
                retained_conflicts: Vec::new(),
            },
        )
        .unwrap();
}

struct PerformanceRow<'a> {
    athlete: &'a AthleteId,
    source_athlete: &'a SourceIdentity,
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
        tenure: CoachTenure::Current { school_year: SchoolYear::new(2026).unwrap() },
        source: SourceRef::new("synthetic_directory", Some("https://contacts.test/schools".into())),
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
    let (julian, source_athlete) = julian(&store, &wi);
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
            Mark::DistanceMetres(CentiMetres::new(642)),
            "wiaa_results",
            "https://wiaa.test/results/invite",
        ),
        (
            &state,
            &relay,
            &EventKind::Relay4x400,
            "2026-06-06",
            Mark::TimeSeconds(CentiSeconds::new(300)),
            "wiaa_results",
            "https://wiaa.test/results/state",
        ),
    ] {
        performance(
            &store,
            &PerformanceRow {
                athlete: &julian,
                source_athlete: &source_athlete,
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
        "Dana Voss",
        Some(Sport::OutdoorTrack),
        CoachRole::HeadCoach,
        "dvoss@abbotsford.k12.wi.us",
    );
    coach(
        &store,
        &wi,
        "Kim Ruiz",
        Some(Sport::CrossCountry),
        CoachRole::HeadCoach,
        "kruiz@abbotsford.k12.wi.us",
    );
    coach(
        &store,
        &wi,
        "Lee Adams",
        None,
        CoachRole::AthleticDirector,
        "ladams@abbotsford.k12.wi.us",
    );
    coach(
        &store,
        &wi,
        "Pat Nolan",
        Some(Sport::OutdoorTrack),
        CoachRole::AssistantCoach,
        "pnolan@abbotsford.k12.wi.us",
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
    let prs = bests::build(store, &bests::Options { scope, grad_year, limit: None }).unwrap();
    Recruiting::load(store, scope, grad_year, SchoolYear::new(2026).unwrap(), prs).unwrap()
}

fn written(fixture: &Fixture) -> (Xlsx<std::io::BufReader<std::fs::File>>, std::path::PathBuf) {
    written_scope(fixture, Scope::Core)
}

fn written_scope(
    fixture: &Fixture,
    scope: Scope,
) -> (Xlsx<std::io::BufReader<std::fs::File>>, std::path::PathBuf) {
    let path = fixture.dir.path().join("recruiting.xlsx");
    let recruiting = recruiting(&fixture.store, scope, Some(2027));
    let mut book = rust_xlsxwriter::Workbook::new();
    recruiting.write_athletes(&mut book, &path).unwrap();
    recruiting.write_prs(&mut book, &path).unwrap();
    recruiting.write_coaches(&mut book, &path).unwrap();
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

fn header(range: &Range<Data>, width: usize) -> Vec<String> {
    (0..width).map(|column| text(range, 0, column)).collect()
}

fn row_of(range: &Range<Data>, key: &str) -> usize {
    row_where(range, |row| text(range, row, 0) == key)
}

fn row_of_event(range: &Range<Data>, athlete: &str, event: &str) -> usize {
    row_where(range, |row| {
        text(range, row, 0) == athlete && text(range, row, 7) == event
    })
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

    assert_eq!(range.height(), 3, "header plus two cohort athletes");

    let row = row_of(&range, fixture.julian.as_str());
    assert_eq!(text(&range, row, 1), "Julian Aguilera");
    assert_eq!(text(&range, row, 2), "Boys");
    assert_eq!(text(&range, row, 3), "2027");
    assert_eq!(text(&range, row, 4), "11", "the newest observed grade");
    assert_eq!(text(&range, row, 6), "WI");
    assert_eq!(text(&range, row, 7), "Abbotsford");
    assert_eq!(text(&range, row, 9), "Abbotsford", "the school row's city");
    assert_eq!(
        text(&range, row, 10),
        "yes",
        "TF (has outdoor track evidence)"
    );
    assert_eq!(text(&range, row, 11), "yes", "XC");
    assert_eq!(text(&range, row, 12), "", "no indoor participation stored");
    assert_eq!(text(&range, row, 13), "yes", "Outdoor");
    assert_eq!(text(&range, row, 16), "", "unrecorded 100m PR is blank");
    assert_eq!(text(&range, row, 17), "", "unrecorded 200m PR is blank");
    assert_eq!(text(&range, row, 18), "48.55", "400m PR");
    assert_eq!(text(&range, row, 28), "6.42", "long-jump PR");
    assert_eq!(text(&range, row, 35), "5", "performance count");
    assert_eq!(text(&range, row, 36), "2", "meet count");
    assert_eq!(text(&range, row, 37), "Dana Voss", "head TF coach");
    assert_eq!(text(&range, row, 38), "dvoss@abbotsford.k12.wi.us");
    assert_eq!(text(&range, row, 39), "Kim Ruiz", "head XC coach");
    assert_eq!(text(&range, row, 40), "kruiz@abbotsford.k12.wi.us");
    assert_eq!(
        text(&range, row, 41),
        "dvoss@abbotsford.k12.wi.us",
        "head TF coach professional email"
    );
    assert_eq!(text(&range, row, 42), "Lee Adams", "athletic director");
    assert_eq!(text(&range, row, 43), "ladams@abbotsford.k12.wi.us");
    assert_eq!(
        text(&range, row, 44),
        "https://abbotsford.test/athletics",
        "school athletics URL"
    );
    assert_eq!(text(&range, row, 45), "", "Public Recruiting GPA is blank");
    assert_eq!(text(&range, row, 46), "", "GPA Source is blank");
    let addresses = text(&range, row, 47);
    assert_eq!(addresses.split("; ").collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([
            "dvoss@abbotsford.k12.wi.us", "kruiz@abbotsford.k12.wi.us",
            "ladams@abbotsford.k12.wi.us", "pnolan@abbotsford.k12.wi.us",
        ]));
    assert_eq!(text(&range, row, 48), "Dana Voss");
    assert_eq!(text(&range, row, 50), "dvoss@abbotsford.k12.wi.us");
    assert_eq!(text(&range, row, 51), "professional_coach_email");
    assert_eq!(
        text(&range, row, 52),
        "",
        "a core-scope workbook drops the Athletic.net identity"
    );
    assert_eq!(
        text(&range, row, 53),
        "https://wi.milesplit.com/athletes/999"
    );
    assert_eq!(text(&range, row, 54), "https://example.test/julian");
    assert_eq!(text(&range, row, 55), "1");
    assert_eq!(text(&range, row, 56), "unverified", "agreeing grades are not an identity decision");
    assert_eq!(text(&range, row, 57), "pr");
    assert_eq!(text(&range, row, 58), "", "no conflict");
    assert_eq!(text(&range, row, 59), "review", "no accepted identity decision exists");
}

#[test]
fn the_url_columns_follow_the_evidence_scope() {
    let fixture = fixture();
    let (mut book, _) = written_scope(&fixture, Scope::AllSources);
    let range = sheet(&mut book, "Athletes");
    let row = row_of(&range, fixture.julian.as_str());
    assert_eq!(
        text(&range, row, 52),
        "https://www.athletic.net/athlete/123"
    );
    assert_eq!(
        text(&range, row, 53),
        "https://wi.milesplit.com/athletes/999"
    );
    assert_eq!(text(&range, row, 54), "https://example.test/julian");
    assert_eq!(text(&range, row, 55), "2", "source namespaces");
}

#[test]
fn an_athlete_without_a_performance_is_published_as_identity_only() {
    let fixture = fixture();
    let (mut book, _) = written(&fixture);
    let range = sheet(&mut book, "Athletes");
    let row = (1..range.height())
        .find(|row| text(&range, *row, 1) == "Nadia Berger")
        .expect("the second cohort athlete");
    assert_eq!(text(&range, row, 0), fixture.nadia.as_str());
    assert_eq!(text(&range, row, 2), "Girls");
    assert_eq!(text(&range, row, 10), "", "TF is blank (no track evidence)");
    assert_eq!(text(&range, row, 35), "0", "no stored performance");
    for column in 15..35 {
        assert_eq!(text(&range, row, column), "", "no PR is blank");
    }
    assert_eq!(text(&range, row, 14), "", "event list is blank");
    assert_eq!(text(&range, row, 15), "", "headline PR summary is blank");
    assert_eq!(text(&range, row, 41), "", "no coach emails");
    assert_eq!(
        text(&range, row, 44),
        "https://ada-borup.test/athletics",
        "school athletics URL"
    );
    assert_eq!(text(&range, row, 57), "identity-only");
    assert_eq!(
        text(&range, row, 59),
        "review",
        "no grade observation agrees with the cohort, so the row asks for review"
    );
    assert_eq!(
        text(&range, row, 47),
        "",
        "all emails is blank (no contacts)"
    );
    assert_eq!(text(&range, row, 48), "", "preferred contact is blank");
    assert_eq!(text(&range, row, 51), "contact_research_unknown");
}

#[test]
fn the_prs_sheet_retains_shared_winners_units_dates_and_provenance() {
    let fixture = fixture();
    let (mut book, _) = written(&fixture);
    let range = sheet(&mut book, "PRs");

    let canonical = bests::build(
        &fixture.store,
        &bests::Options {
            scope: Scope::Core,
            grad_year: Some(2027),
            limit: None,
        },
    )
    .unwrap();
    assert_eq!(
        range.height() - 1,
        canonical.len(),
        "one row per athlete/event"
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

    let jump = row_of_event(&range, fixture.julian.as_str(), "LongJump");
    assert_eq!(text(&range, jump, 6), "Field");
    assert_eq!(text(&range, jump, 9), "6.42 m");
    assert_eq!(text(&range, jump, 10), "6.42");
    assert_eq!(text(&range, jump, 11), "m");
    assert_eq!(text(&range, jump, 17), "1");
    assert_eq!(text(&range, jump, 18), "", "one report cannot conflict");
}

#[test]
fn the_coaches_sheet_publishes_the_school_contact_graph() {
    let fixture = fixture();
    let (mut book, _) = written(&fixture);
    let range = sheet(&mut book, "Coaches");
    assert_eq!(range.height(), 5, "header plus the four stored coaches");

    let head = row_where(&range, |row| text(&range, row, 5) == "Dana Voss");
    assert_eq!(text(&range, head, 0), fixture.wi_school.as_str());
    assert_eq!(text(&range, head, 1), "Abbotsford");
    assert_eq!(text(&range, head, 2), "Abbotsford");
    assert_eq!(text(&range, head, 3), "WI");
    assert_eq!(text(&range, head, 4), "OutdoorTrack");
    assert_eq!(text(&range, head, 6), "HeadCoach");
    assert_eq!(text(&range, head, 7), "dvoss@abbotsford.k12.wi.us");
    assert_eq!(text(&range, head, 8), "");
    assert_eq!(text(&range, head, 9), "");
    assert_eq!(text(&range, head, 10), "Lee Adams");
    assert_eq!(text(&range, head, 11), "ladams@abbotsford.k12.wi.us");
    assert_eq!(text(&range, head, 12), "https://contacts.test/schools");
    assert_eq!(text(&range, head, 13), DAY);

    let director = row_where(&range, |row| text(&range, row, 5) == "Lee Adams");
    assert_eq!(
        text(&range, director, 4),
        "school_wide",
        "an AD is not bound to a sport"
    );
    assert_eq!(text(&range, director, 6), "AthleticDirector");
    assert_eq!(text(&range, director, 7), "ladams@abbotsford.k12.wi.us");
    assert_eq!(text(&range, director, 8), "");
}

#[test]
fn an_unrelated_programmes_personal_address_is_retained_only_in_raw_coach_history() {
    let fixture = fixture();
    personal_coach(&fixture.store, &fixture.wi_school);
    let (mut book, _) = written(&fixture);

    let athletes = sheet(&mut book, "Athletes");
    let athlete_row = row_of(&athletes, fixture.julian.as_str());
    assert!(!text(&athletes, athlete_row, 47).contains("morgan@gmail.com"));

    let coaches = sheet(&mut book, "Coaches");
    let coach_row = row_where(&coaches, |row| text(&coaches, row, 5) == "Morgan Personal");
    assert_eq!(text(&coaches, coach_row, 7), "");
    assert_eq!(text(&coaches, coach_row, 8), "morgan@gmail.com");
    assert_eq!(text(&coaches, coach_row, 9), "");
}

#[test]
fn the_run_audits_every_sheet_against_the_store_counts_behind_it() {
    let fixture = fixture();
    let projection = recruiting(&fixture.store, Scope::Core, Some(2027));
    let audit = projection.dataset.audit();
    assert_eq!(
        audit.store_athletes, 3,
        "the merge folds the athlete's two observations into one row"
    );
    assert_eq!(audit.scoped_athletes, 3);
    assert_eq!(audit.cohort_athletes, 2);
    assert_eq!(audit.pr_rows, 2);
    assert_eq!(audit.coach_rows, 4);

    let all_sources = recruiting(&fixture.store, Scope::AllSources, Some(2027));
    assert_eq!(all_sources.dataset.audit().cohort_athletes, 2);
    assert_eq!(all_sources.dataset.audit().pr_rows, 2);

    let every_cohort = recruiting(&fixture.store, Scope::Core, None);
    assert_eq!(
        every_cohort.dataset.audit().cohort_athletes,
        3,
        "no cohort filter publishes every in-scope athlete"
    );
}

