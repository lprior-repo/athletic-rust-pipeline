//! The recruiting sheets, asserted against a real workbook written from a fixture store.
//!
//! The fixture is the smallest store that exercises every documented rule: two schools in two
//! jurisdictions, a cohort athlete with performances from two sources (one meet reported twice with
//! different marks, one relay leg, one field event), a cohort athlete with no performance at all, and
//! an out-of-cohort athlete the sheets must not publish. Every assertion below reads the sheet the
//! workbook actually wrote, through `calamine`, exactly as the census workbook tests do.

use super::*;
use crate::bests;
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use census_domain::model::{
    normalize_name, AthleteId, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, CentiMetres, CentiSeconds, CoachRole,
    CompetitionLevel, EventId, EventKind, Evidence, Gender, GradYear, Grade, Mark, MeetId,
    ObservedGrade, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

const DAY: &str = "2026-09-20";

/// The fixture store and the ids the assertions below need.
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

/// A school with an athletics site and a city, appended once.
fn school(store: &Store, state: UsJurisdiction, name: &str) -> SchoolId {
    let (mut school, id) = CanonicalSchool::new(state, name, normalize_name(name));
    school.athletics_website = Some(format!("https://{}.test/athletics", name.to_lowercase()));
    school.city = Some(name.to_string());
    school.evidence = evidence("wiaa_results", Some("https://wiaa.test/schools"));
    store.append(Table::Schools, &school).unwrap();
    id
}

/// The cohort athlete: two sources, a grade observation that agrees with the cohort (so the merge
/// raises confidence), and the profile URLs the three URL columns split.
fn julian(store: &Store, school: &SchoolId) -> AthleteId {
    for _ in 0..2 {
        let mut athlete =
            CanonicalAthlete::new(school, "Julian Aguilera", GradYear::CO2027, Gender::Boys);
        athlete.observed_grades.push(ObservedGrade {
            grade: Grade::new(11).unwrap(),
            school_year: SchoolYear::new(2025).expect("2025 is a season"),
            source: SourceRef::new("wiaa_results", None),
        });
        athlete.sports = vec![Sport::OutdoorTrack, Sport::CrossCountry];
        athlete.public_profile_urls = vec!["https://example.test/julian".to_string()];
        athlete.source_identities = vec![
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "wi-999")
                .with_url("https://wi.milesplit.com/athletes/999"),
            SourceIdentity::new(
                SourceNamespace::AthleticNet {
                    kind: "athlete".to_string(),
                },
                "123",
            )
            .with_url("https://www.athletic.net/athlete/123"),
        ];
        athlete.evidence = evidence("wiaa_results", None);
        store.append(Table::Athletes, &athlete).unwrap();
    }
    CanonicalAthlete::mint(school, "Julian Aguilera", GradYear::CO2027, Gender::Boys)
}

/// A cohort athlete with no performance and no grade observation: the `identity-only` coverage state.
fn nadia(store: &Store, school: &SchoolId) -> AthleteId {
    let mut athlete =
        CanonicalAthlete::new(school, "Nadia Berger", GradYear::CO2027, Gender::Girls);
    athlete.sports = vec![Sport::CrossCountry];
    athlete.evidence = evidence("mshsl_results", None);
    store.append(Table::Athletes, &athlete).unwrap();
    CanonicalAthlete::mint(school, "Nadia Berger", GradYear::CO2027, Gender::Girls)
}

/// An athlete one class below the cohort: no recruiting sheet may publish this row.
fn younger(store: &Store, school: &SchoolId) {
    let mut athlete = CanonicalAthlete::new(
        school,
        "Owen Clarke",
        GradYear::new(2028).expect("2028 is a class"),
        Gender::Boys,
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
    let team = CanonicalTeam::mint(
        context.school,
        Sport::OutdoorTrack,
        Gender::Boys,
        SchoolYear::new(2026).expect("2026 is a season"),
    );
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
                source_athlete: None,
                retained_conflicts: Vec::new(),
            },
        )
        .unwrap();
}

/// The row of the performance table one call writes.
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
    store.append(Table::Coaches, &coach).unwrap();
}

/// The whole fixture: two schools, three athletes, three meets, five marks, four coaches.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let wi = school(&store, UsJurisdiction::Wisconsin, "Abbotsford");
    let mn = school(&store, UsJurisdiction::Minnesota, "Ada-Borup");
    let julian = julian(&store, &wi);
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

    // The same 400m at the same meet, reported twice with different marks: the PR row must flag it.
    for (meet, event, kind, date, mark, source, url) in [
        (
            &state,
            &sprint,
            &EventKind::Track400m,
            "2026-06-06",
            Mark::TimeSeconds(CentiSeconds(4980)),
            "wiaa_results",
            "https://wiaa.test/results/state",
        ),
        (
            &state,
            &sprint,
            &EventKind::Track400m,
            "2026-06-06",
            Mark::TimeSeconds(CentiSeconds(4971)),
            "pttiming_live",
            "https://pttiming.test/live/state",
        ),
        (
            &invite,
            &sprint_invite,
            &EventKind::Track400m,
            "2026-05-01",
            Mark::TimeSeconds(CentiSeconds(4855)),
            "wiaa_results",
            "https://wiaa.test/results/invite",
        ),
        (
            &invite,
            &jump,
            &EventKind::LongJump,
            "2026-05-01",
            Mark::DistanceMetres(CentiMetres(642)),
            "wiaa_results",
            "https://wiaa.test/results/invite",
        ),
        (
            &state,
            &relay,
            &EventKind::Relay4x400,
            "2026-06-06",
            Mark::TimeSeconds(CentiSeconds(300)),
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

/// Write the three recruiting sheets from the fixture store and open the result.
fn written(fixture: &Fixture) -> (Xlsx<std::io::BufReader<std::fs::File>>, std::path::PathBuf) {
    written_scope(fixture, Scope::Core)
}

/// The same, at a chosen evidence scope.
fn written_scope(
    fixture: &Fixture,
    scope: Scope,
) -> (Xlsx<std::io::BufReader<std::fs::File>>, std::path::PathBuf) {
    let path = fixture.dir.path().join("recruiting.xlsx");
    let recruiting = Recruiting::load(&fixture.store, scope, Some(2027)).unwrap();
    let mut book = rust_xlsxwriter::Workbook::new();
    recruiting.write_athletes(&mut book, &path).unwrap();
    recruiting.write_prs(&mut book, &path).unwrap();
    recruiting.write_coaches(&mut book, &path).unwrap();
    recruiting.reconcile(&fixture.store).unwrap();
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

/// The row whose first column holds `key`.
fn row_of(range: &Range<Data>, key: &str) -> usize {
    row_where(range, |row| text(range, row, 0) == key)
}

/// The row whose athlete column holds `athlete` and whose event column holds `event`.
fn row_of_event(range: &Range<Data>, athlete: &str, event: &str) -> usize {
    row_where(range, |row| {
        text(range, row, 0) == athlete && text(range, row, 5) == event
    })
}

/// The first data row matching `matches`, panicking with the sheet's name when there is none.
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
    assert_eq!(
        header(&range, athletes::HEADERS.len()),
        athletes::HEADERS.to_vec()
    );

    // One row per canonical class-of-2027 athlete: the out-of-cohort athlete is not published.
    assert_eq!(range.height(), 3, "header plus two cohort athletes");

    let row = row_of(&range, fixture.julian.as_str());
    assert_eq!(text(&range, row, 1), "Julian Aguilera");
    assert_eq!(text(&range, row, 2), "WI");
    assert_eq!(text(&range, row, 3), "Abbotsford");
    assert_eq!(text(&range, row, 4), "2027");
    assert_eq!(text(&range, row, 5), "11", "the newest observed grade");
    assert_eq!(text(&range, row, 6), "yes", "TF");
    assert_eq!(text(&range, row, 7), "yes", "XC");
    assert_eq!(text(&range, row, 8), "", "no indoor participation stored");
    assert_eq!(text(&range, row, 9), "yes", "Outdoor");
    assert_eq!(
        text(&range, row, 10),
        "LongJump; Relay4x400; Track400m",
        "every event the athlete has a stored mark in"
    );
    assert_eq!(
        text(&range, row, 11),
        "LongJump 6.42 m; Track400m 48.55",
        "the PRs sheet's own rows, in its own order"
    );
    assert_eq!(text(&range, row, 12), "5", "performance count");
    assert_eq!(text(&range, row, 13), "2", "meet count");
    assert_eq!(
        text(&range, row, 14),
        "",
        "a core-scope workbook drops the Athletic.net identity, so no URL is published"
    );
    assert_eq!(
        text(&range, row, 15),
        "https://wi.milesplit.com/athletes/999",
        "the MileSplit identity is core evidence"
    );
    assert_eq!(text(&range, row, 16), "https://example.test/julian");
    assert_eq!(text(&range, row, 17), "Dana Voss", "head TF coach");
    assert_eq!(text(&range, row, 18), "Kim Ruiz", "head XC coach");
    assert_eq!(text(&range, row, 19), "dvoss@abbotsford.k12.wi.us");
    assert_eq!(text(&range, row, 20), "Lee Adams", "athletic director");
    assert_eq!(text(&range, row, 21), "https://abbotsford.test/athletics");
    assert_eq!(text(&range, row, 22), "", "no GPA entity is stored");
    assert_eq!(text(&range, row, 23), "", "no GPA source is stored");
    assert_eq!(
        text(&range, row, 24),
        "1",
        "core scope keeps the MileSplit namespace and drops Athletic.net"
    );
    assert_eq!(text(&range, row, 25), "85", "agreeing grade evidence");
    assert_eq!(text(&range, row, 26), "pr");
    assert_eq!(text(&range, row, 27), "", "no conflict");
    assert_eq!(
        text(&range, row, 28),
        "",
        "confidence at HIGH needs no review"
    );
    assert_eq!(text(&range, row, 29), "Abbotsford", "the school row's city");
    assert_eq!(text(&range, row, 30), "dvoss@abbotsford.k12.wi.us");
    assert_eq!(text(&range, row, 31), "kruiz@abbotsford.k12.wi.us");
    assert_eq!(text(&range, row, 32), "ladams@abbotsford.k12.wi.us");
    assert_eq!(
        text(&range, row, 33),
        "Dana Voss",
        "the athlete's own sport picks the slot: track and field wins over cross country"
    );
    assert_eq!(text(&range, row, 34), "Head TF Coach");
    assert_eq!(text(&range, row, 35), "dvoss@abbotsford.k12.wi.us");
    assert_eq!(text(&range, row, 36), "professional_coach_email");
}

/// The three profile URL columns follow the evidence scope: the Athletic.net identity is only
/// published by an all-sources workbook.
#[test]
fn the_url_columns_follow_the_evidence_scope() {
    let fixture = fixture();
    let (mut book, _) = written_scope(&fixture, Scope::AllSources);
    let range = sheet(&mut book, "Athletes");
    let row = row_of(&range, fixture.julian.as_str());
    assert_eq!(
        text(&range, row, 14),
        "https://www.athletic.net/athlete/123"
    );
    assert_eq!(
        text(&range, row, 15),
        "https://wi.milesplit.com/athletes/999"
    );
    assert_eq!(text(&range, row, 16), "https://example.test/julian");
    assert_eq!(text(&range, row, 24), "2", "source namespaces");
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
    assert_eq!(text(&range, row, 2), "MN");
    assert_eq!(text(&range, row, 12), "0", "no stored performance");
    assert_eq!(text(&range, row, 26), "identity-only");
    assert_eq!(
        text(&range, row, 28),
        "yes",
        "no grade observation agrees with the cohort, so the row asks for review"
    );
    assert_eq!(text(&range, row, 29), "Ada-Borup");
    assert_eq!(
        text(&range, row, 30),
        "",
        "the school has no stored coach row"
    );
    assert_eq!(text(&range, row, 33), "", "nothing is named");
    assert_eq!(
        text(&range, row, 36),
        "contact_source_not_attempted",
        "no coach row for the school means no contact source reached it"
    );
}

#[test]
fn the_prs_sheet_reduces_like_bests_and_flags_a_meet_reported_twice() {
    let fixture = fixture();
    let (mut book, _) = written(&fixture);
    let range = sheet(&mut book, "PRs");
    assert_eq!(header(&range, prs::HEADERS.len()), prs::HEADERS.to_vec());

    // The crate's own reduction over the same store, scope and cohort publishes the same row count.
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

    // The relay leg is not a personal best, so only the 400m and the long jump are published.
    let events: Vec<String> = (1..range.height())
        .map(|row| text(&range, row, 5))
        .collect();
    assert_eq!(events, vec!["LongJump", "Track400m"]);

    let sprint = row_of_event(&range, fixture.julian.as_str(), "Track400m");
    assert_eq!(text(&range, sprint, 1), "Julian Aguilera");
    assert_eq!(text(&range, sprint, 2), "Abbotsford");
    assert_eq!(text(&range, sprint, 3), "WI");
    assert_eq!(text(&range, sprint, 4), "Track");
    assert_eq!(text(&range, sprint, 6), "Outdoor");
    assert_eq!(
        text(&range, sprint, 7),
        "48.55",
        "the fastest of the three marks"
    );
    assert_eq!(text(&range, sprint, 8), "2026-05-01");
    assert_eq!(text(&range, sprint, 9), "Abbotsford Invitational");
    assert_eq!(text(&range, sprint, 10), "https://wiaa.test/results/invite");
    assert_eq!(
        text(&range, sprint, 14),
        "2",
        "two sources report this event"
    );
    assert_eq!(
        text(&range, sprint, 15),
        "WIAA Division 3 State: 49.71 | 49.80",
        "the state meet's two published marks are both carried, not reduced to a flag"
    );

    // No source published a PR claim for this athlete, so the three reported-PR columns stay blank
    // rather than restating the calculated mark as if a source had published it.
    for column in [11, 12, 13] {
        assert_eq!(text(&range, sprint, column), "", "no reported PR is stored");
    }

    // The long jump was reported by one source only, at one meet: one source, no conflict.
    let jump = row_of_event(&range, fixture.julian.as_str(), "LongJump");
    assert_eq!(text(&range, jump, 4), "Field");
    assert_eq!(
        text(&range, jump, 7),
        "6.42 m",
        "the notation the source published"
    );
    assert_eq!(text(&range, jump, 14), "1");
    assert_eq!(text(&range, jump, 15), "", "one report cannot conflict");
}

#[test]
fn the_coaches_sheet_publishes_the_school_contact_graph() {
    let fixture = fixture();
    let (mut book, _) = written(&fixture);
    let range = sheet(&mut book, "Coaches");
    assert_eq!(
        header(&range, coaches::HEADERS.len()),
        coaches::HEADERS.to_vec()
    );
    assert_eq!(range.height(), 5, "header plus the four stored coaches");

    let names: Vec<String> = (1..range.height())
        .map(|row| text(&range, row, 4))
        .collect();
    assert_eq!(
        names,
        vec!["Kim Ruiz", "Pat Nolan", "Dana Voss", "Lee Adams"],
        "state, school, sport, role, then coach"
    );

    let head = 3;
    assert_eq!(text(&range, head, 0), fixture.wi_school.as_str());
    assert_eq!(text(&range, head, 1), "Abbotsford");
    assert_eq!(text(&range, head, 2), "WI");
    assert_eq!(text(&range, head, 3), "OutdoorTrack");
    assert_eq!(text(&range, head, 5), "HeadCoach");
    assert_eq!(text(&range, head, 6), "dvoss@abbotsford.k12.wi.us");
    assert_eq!(text(&range, head, 7), "Lee Adams");
    assert_eq!(text(&range, head, 8), "ladams@abbotsford.k12.wi.us");
    assert_eq!(text(&range, head, 9), "https://contacts.test/schools");
    assert_eq!(text(&range, head, 10), DAY);

    let director = 4;
    assert_eq!(
        text(&range, director, 3),
        "school_wide",
        "an AD is not bound to a sport"
    );
    assert_eq!(text(&range, director, 5), "AthleticDirector");
    assert_eq!(text(&range, director, 6), "ladams@abbotsford.k12.wi.us");
}

#[test]
fn the_run_audits_every_sheet_against_the_store_counts_behind_it() {
    let fixture = fixture();
    let recruiting = Recruiting::load(&fixture.store, Scope::Core, Some(2027)).unwrap();
    let audit = recruiting.dataset.audit();
    assert_eq!(
        audit.store_athletes, 3,
        "the merge folds the athlete's two observations into one row"
    );
    assert_eq!(audit.scoped_athletes, 3);
    assert_eq!(audit.cohort_athletes, 2);
    assert_eq!(audit.pr_rows, 2);
    assert_eq!(audit.coach_rows, 4);

    let all_sources = Recruiting::load(&fixture.store, Scope::AllSources, Some(2027)).unwrap();
    assert_eq!(all_sources.dataset.audit().cohort_athletes, 2);
    assert_eq!(all_sources.dataset.audit().pr_rows, 2);

    let every_cohort = Recruiting::load(&fixture.store, Scope::Core, None).unwrap();
    assert_eq!(
        every_cohort.dataset.audit().cohort_athletes,
        3,
        "no cohort filter publishes every in-scope athlete"
    );
}

/// A coach row built directly, with one evidence observation: the contact tests below never need a
/// store, and every choice the ladder makes is asserted on the values it publishes.
fn coach_row(
    school: &SchoolId,
    name: &str,
    sport: Option<Sport>,
    side: Gender,
    role: CoachRole,
    email: Option<&str>,
    observed: &str,
) -> CanonicalCoach {
    let mut coach = CanonicalCoach::new(school, name, sport, side, role);
    coach.professional_email = email.map(str::to_string);
    coach.evidence = vec![Evidence::parsed(
        SourceRef::new("coach_contacts_csv", None),
        observed,
    )];
    coach
}

/// A school id minted the way the school table mints it.
fn school_id(state: UsJurisdiction, name: &str) -> SchoolId {
    CanonicalSchool::new(state, name, normalize_name(name)).1
}

/// An athlete with the given sport evidence and side of the team.
fn athlete_of(school: &SchoolId, sports: &[Sport], side: Gender) -> CanonicalAthlete {
    let mut athlete =
        CanonicalAthlete::new(school, "Preferred Contact Athlete", GradYear::CO2027, side);
    athlete.sports = sports.to_vec();
    athlete
}

/// The four cells the preferred-contact block publishes, in header order: contact, role, email,
/// state.
fn contact_cells(preferred: &contact::Preferred) -> [String; 4] {
    [
        preferred.name.clone(),
        preferred.role.clone(),
        preferred.email.clone(),
        preferred.state.as_str().to_string(),
    ]
}

/// One school's contact facts, resolved from the coach rows exactly as the load pass resolves them.
fn school_contacts(coaches: &[CanonicalCoach], school: &SchoolId) -> contact::SchoolContacts {
    let mut index = contact::contacts(coaches);
    index
        .remove(school.as_str())
        .expect("the fixture school carries coach rows")
}

/// The evidence-bearing sport picks the slot: track and field for a dual athlete, cross country for
/// an athlete whose only stored sport is cross country.
#[test]
fn the_preferred_contact_follows_the_evidence_bearing_sport() {
    let school = school_id(UsJurisdiction::Wisconsin, "Sport High");
    let coaches = vec![
        coach_row(
            &school,
            "Dana Voss",
            Some(Sport::OutdoorTrack),
            Gender::Mixed,
            CoachRole::HeadCoach,
            Some("dvoss@school.test"),
            "2026-09-20",
        ),
        coach_row(
            &school,
            "Kim Ruiz",
            Some(Sport::CrossCountry),
            Gender::Mixed,
            CoachRole::HeadCoach,
            Some("kruiz@school.test"),
            "2026-09-20",
        ),
    ];
    let contacts = school_contacts(&coaches, &school);

    let dual = contact::preferred(
        Some(&contacts),
        &athlete_of(
            &school,
            &[Sport::OutdoorTrack, Sport::IndoorTrack, Sport::CrossCountry],
            Gender::Boys,
        ),
    );
    assert_eq!(
        contact_cells(&dual),
        [
            "Dana Voss",
            "Head TF Coach",
            "dvoss@school.test",
            "professional_coach_email"
        ],
        "an athlete with both sports prefers the track and field coach"
    );

    let cross_country = contact::preferred(
        Some(&contacts),
        &athlete_of(&school, &[Sport::CrossCountry], Gender::Girls),
    );
    assert_eq!(
        contact_cells(&cross_country),
        [
            "Kim Ruiz",
            "Head XC Coach",
            "kruiz@school.test",
            "professional_coach_email"
        ]
    );

    let indoor = contact::preferred(
        Some(&contacts),
        &athlete_of(&school, &[Sport::IndoorTrack], Gender::Girls),
    );
    assert_eq!(
        contact_cells(&indoor)[0],
        "Dana Voss",
        "indoor track is track"
    );
}

/// A head coach's published address beats the athletic director's; the director's address is the
/// fallback; a named contact with no address anywhere is named and flagged as such.
#[test]
fn a_head_coach_address_wins_over_the_athletic_director() {
    let school = school_id(UsJurisdiction::Minnesota, "Fallback High");
    let coach = coach_row(
        &school,
        "Dana Voss",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
        Some("dvoss@school.test"),
        "2026-09-20",
    );
    let director = coach_row(
        &school,
        "Lee Adams",
        None,
        Gender::Mixed,
        CoachRole::AthleticDirector,
        Some("ladams@school.test"),
        "2026-09-20",
    );
    let athlete = athlete_of(&school, &[Sport::OutdoorTrack], Gender::Boys);

    let both = school_contacts(&[coach.clone(), director.clone()], &school);
    assert_eq!(
        contact_cells(&contact::preferred(Some(&both), &athlete)),
        [
            "Dana Voss",
            "Head TF Coach",
            "dvoss@school.test",
            "professional_coach_email"
        ],
        "the coach's public address wins over the director's"
    );

    let mut silent_coach = coach.clone();
    silent_coach.professional_email = None;
    let director_only = school_contacts(&[silent_coach.clone(), director.clone()], &school);
    assert_eq!(
        contact_cells(&contact::preferred(Some(&director_only), &athlete)),
        [
            "Lee Adams",
            "Athletic Director",
            "ladams@school.test",
            "professional_ad_email"
        ],
        "no public coach address falls back to the director's"
    );

    let mut silent_director = director.clone();
    silent_director.professional_email = None;
    let named_only = school_contacts(&[silent_coach, silent_director], &school);
    assert_eq!(
        contact_cells(&contact::preferred(Some(&named_only), &athlete)),
        ["Dana Voss", "Head TF Coach", "", "coach_name_only"],
        "a named coach with no public address anywhere is named, with no address"
    );
}

/// A newer head coach who published no address never shadows an older one who did: otherwise the
/// ladder would fall through to the athletic director and lose the coach's own address.
#[test]
fn a_published_address_outranks_a_newer_row_without_one() {
    let school = school_id(UsJurisdiction::Illinois, "Address High");
    let coaches = vec![
        coach_row(
            &school,
            "Older Address",
            Some(Sport::CrossCountry),
            Gender::Mixed,
            CoachRole::HeadCoach,
            Some("older@school.test"),
            "2026-09-01",
        ),
        coach_row(
            &school,
            "Newer Silent",
            Some(Sport::CrossCountry),
            Gender::Mixed,
            CoachRole::HeadCoach,
            None,
            "2026-09-25",
        ),
        coach_row(
            &school,
            "Lee Adams",
            None,
            Gender::Mixed,
            CoachRole::AthleticDirector,
            Some("ladams@school.test"),
            "2026-09-20",
        ),
    ];
    let contacts = school_contacts(&coaches, &school);
    let athlete = athlete_of(&school, &[Sport::CrossCountry], Gender::Girls);
    assert_eq!(
        contact_cells(&contact::preferred(Some(&contacts), &athlete)),
        [
            "Older Address",
            "Head XC Coach",
            "older@school.test",
            "professional_coach_email"
        ]
    );
}

/// Two head coaches of one sport and one side publishing different addresses: the newest evidence
/// decides, and an equal observation date falls to the coach's name, so two runs agree.
#[test]
fn the_newest_evidence_resolves_two_head_coaches_of_one_sport() {
    let school = school_id(UsJurisdiction::Wisconsin, "Twin High");
    let older = coach_row(
        &school,
        "Alex Older",
        Some(Sport::OutdoorTrack),
        Gender::Boys,
        CoachRole::HeadCoach,
        Some("aolder@school.test"),
        "2026-09-01",
    );
    let newer = coach_row(
        &school,
        "Blair Newer",
        Some(Sport::OutdoorTrack),
        Gender::Boys,
        CoachRole::HeadCoach,
        Some("bnewer@school.test"),
        "2026-09-20",
    );
    let athlete = athlete_of(&school, &[Sport::OutdoorTrack], Gender::Boys);

    let contacts = school_contacts(&[older.clone(), newer.clone()], &school);
    assert_eq!(
        contacts.heads.conflicts(),
        1,
        "two addresses in one bucket is the disagreement the rule resolves"
    );
    assert_eq!(
        contact_cells(&contact::preferred(Some(&contacts), &athlete)),
        [
            "Blair Newer",
            "Head TF Coach (boys)",
            "bnewer@school.test",
            "professional_coach_email"
        ],
        "the newest observation wins"
    );

    // The disagreement precedence settled is still retained for a human: one row naming the school,
    // the slot it is about, and every row the rule had to choose from.
    let recorded = disagreements(&[older.clone(), newer]);
    let disagreement = recorded.first().expect("one bucket disagreed");
    assert!(
        disagreement.decided,
        "the newest observation separated the rows"
    );
    assert_eq!(disagreement.school, school.as_str());
    assert_eq!(disagreement.role, "Head TF Coach (boys)");
    assert_eq!(
        disagreement.rows,
        [
            "Alex Older: aolder@school.test (observed 2026-09-01)",
            "Blair Newer: bnewer@school.test (observed 2026-09-20)"
        ],
        "both rows, with the address and the observation that dates them"
    );

    // Two rows sharing the newest observation: neither the address nor the observation picks one, so
    // the cell says the contact is unresolved instead of a coin toss between names.
    let tied = coach_row(
        &school,
        "Blair Newer",
        Some(Sport::OutdoorTrack),
        Gender::Boys,
        CoachRole::HeadCoach,
        Some("bnewer@school.test"),
        "2026-09-01",
    );
    let tied_contacts = school_contacts(&[older.clone(), tied.clone()], &school);
    assert_eq!(
        contact_cells(&contact::preferred(Some(&tied_contacts), &athlete)),
        ["", "", "", "contact_conflict"],
        "an equal observation date leaves the bucket unresolved"
    );
    let recorded = disagreements(&[older, tied]);
    assert!(
        recorded.first().is_some_and(|row| !row.decided),
        "the undecidable bucket is retained too: {recorded:?}"
    );
}

/// A school that publishes a boys' and a girls' head coach of one sport: the athlete's own side of
/// the team picks the coach, and the role cell names the side.
#[test]
fn the_athletes_side_of_the_team_picks_the_head_coach() {
    let school = school_id(UsJurisdiction::Minnesota, "Sides High");
    let coaches = vec![
        coach_row(
            &school,
            "Jeremy Lee",
            Some(Sport::OutdoorTrack),
            Gender::Boys,
            CoachRole::HeadCoach,
            Some("jlee@school.test"),
            "2026-09-20",
        ),
        coach_row(
            &school,
            "Kelsey Daines",
            Some(Sport::OutdoorTrack),
            Gender::Girls,
            CoachRole::HeadCoach,
            Some("kdaines@school.test"),
            "2026-09-20",
        ),
    ];
    let contacts = school_contacts(&coaches, &school);

    let girls = contact::preferred(
        Some(&contacts),
        &athlete_of(&school, &[Sport::OutdoorTrack], Gender::Girls),
    );
    assert_eq!(
        contact_cells(&girls),
        [
            "Kelsey Daines",
            "Head TF Coach (girls)",
            "kdaines@school.test",
            "professional_coach_email"
        ]
    );
    let boys = contact::preferred(
        Some(&contacts),
        &athlete_of(&school, &[Sport::OutdoorTrack], Gender::Boys),
    );
    assert_eq!(
        contact_cells(&boys),
        [
            "Jeremy Lee",
            "Head TF Coach (boys)",
            "jlee@school.test",
            "professional_coach_email"
        ]
    );
    assert_eq!(
        contacts.heads.conflicts(),
        0,
        "one address per side of the team is not a disagreement"
    );
}

/// A head coach whose row carries no sport binding is reached after both sport slots are empty, and
/// an athlete that stores no sport at all still resolves through the ladder.
#[test]
fn a_school_wide_head_coach_is_the_last_coach_rung() {
    let school = school_id(UsJurisdiction::Illinois, "Wide High");
    let coaches = vec![coach_row(
        &school,
        "Robin Wide",
        None,
        Gender::Mixed,
        CoachRole::HeadCoach,
        Some("rwide@school.test"),
        "2026-09-20",
    )];
    let contacts = school_contacts(&coaches, &school);

    let cross_country = contact::preferred(
        Some(&contacts),
        &athlete_of(&school, &[Sport::CrossCountry], Gender::Girls),
    );
    assert_eq!(
        contact_cells(&cross_country),
        [
            "Robin Wide",
            "Head Coach",
            "rwide@school.test",
            "professional_coach_email"
        ]
    );

    let no_sport = contact::preferred(Some(&contacts), &athlete_of(&school, &[], Gender::Girls));
    assert_eq!(contact_cells(&no_sport)[0], "Robin Wide");
}

/// The states that say what was looked at: no coach row for the school at all is
/// `contact_source_not_attempted`, rows that name no head coach and no director are
/// `no_public_contact_found`.
#[test]
fn the_contact_state_separates_looking_from_finding_nothing() {
    let school = school_id(UsJurisdiction::Wisconsin, "Quiet High");
    let athlete = athlete_of(&school, &[Sport::CrossCountry], Gender::Girls);

    let unattempted = contact::preferred(None, &athlete);
    assert_eq!(
        contact_cells(&unattempted),
        ["", "", "", "contact_source_not_attempted"],
        "a school the coach table never reached answers that nothing was looked at"
    );

    let assistants = vec![coach_row(
        &school,
        "Pat Nolan",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::AssistantCoach,
        Some("pnolan@school.test"),
        "2026-09-20",
    )];
    let contacts = school_contacts(&assistants, &school);
    assert_eq!(
        contact_cells(&contact::preferred(Some(&contacts), &athlete)),
        ["", "", "", "no_public_contact_found"],
        "a contact source reached the school and published no contact"
    );
}
/// Two head coaches of one slot with no published address at all: nothing dates them apart, so
/// `coach_name_only` would be an alphabetical accident and the state says the bucket is unresolved.
#[test]
fn two_named_coaches_without_an_address_are_a_contact_conflict() {
    let school = school_id(UsJurisdiction::Minnesota, "Names High");
    let coaches = vec![
        coach_row(
            &school,
            "Alex Nolan",
            Some(Sport::CrossCountry),
            Gender::Girls,
            CoachRole::HeadCoach,
            None,
            "2026-09-20",
        ),
        coach_row(
            &school,
            "Blair Ortiz",
            Some(Sport::CrossCountry),
            Gender::Girls,
            CoachRole::HeadCoach,
            None,
            "2026-09-20",
        ),
    ];
    let athlete = athlete_of(&school, &[Sport::CrossCountry], Gender::Girls);
    let contacts = school_contacts(&coaches, &school);
    assert_eq!(
        contact_cells(&contact::preferred(Some(&contacts), &athlete)),
        ["", "", "", "contact_conflict"],
        "two names and no address is a disagreement, not an unpublished contact"
    );
    let recorded = disagreements(&coaches);
    assert!(
        recorded
            .first()
            .is_some_and(|row| !row.decided && row.role == "Head XC Coach (girls)"),
        "the name-only bucket is retained as an undecided disagreement: {recorded:?}"
    );
}

/// A bucket is the unit, not the school: a cross-country bucket two coaches disagree about leaves the
/// track athlete's own contact alone and reaches only the athlete whose slot is that bucket.
#[test]
fn a_conflicting_slot_does_not_deny_another_slots_athlete() {
    let school = school_id(UsJurisdiction::Illinois, "Scoped High");
    let coaches = vec![
        coach_row(
            &school,
            "Dana Voss",
            Some(Sport::OutdoorTrack),
            Gender::Boys,
            CoachRole::HeadCoach,
            Some("dvoss@school.test"),
            "2026-09-20",
        ),
        coach_row(
            &school,
            "Kim Ruiz",
            Some(Sport::CrossCountry),
            Gender::Girls,
            CoachRole::HeadCoach,
            Some("kruiz@school.test"),
            "2026-09-20",
        ),
        coach_row(
            &school,
            "Lee Sato",
            Some(Sport::CrossCountry),
            Gender::Girls,
            CoachRole::HeadCoach,
            Some("lsato@school.test"),
            "2026-09-20",
        ),
    ];
    let contacts = school_contacts(&coaches, &school);
    assert_eq!(
        contacts.heads.conflicts(),
        1,
        "only the cross-country girls bucket disagrees"
    );
    let track = contact::preferred(
        Some(&contacts),
        &athlete_of(&school, &[Sport::OutdoorTrack], Gender::Boys),
    );
    assert_eq!(
        contact_cells(&track),
        [
            "Dana Voss",
            "Head TF Coach (boys)",
            "dvoss@school.test",
            "professional_coach_email"
        ],
        "the track bucket resolved, so its athlete still has a contact"
    );
    let cross_country = contact::preferred(
        Some(&contacts),
        &athlete_of(&school, &[Sport::CrossCountry], Gender::Girls),
    );
    assert_eq!(
        contact_cells(&cross_country),
        ["", "", "", "contact_conflict"],
        "the athlete whose own bucket disagrees gets the conflict state"
    );
}
