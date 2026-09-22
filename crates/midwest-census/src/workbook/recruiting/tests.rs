//! The recruiting sheets, asserted against a real workbook written from a fixture store.
//!
//! The fixture is the smallest store that exercises every documented rule: two schools in two
//! jurisdictions, a cohort athlete with performances from two sources (one meet reported twice with
//! different marks, one relay leg, one field event), a cohort athlete with no performance at all, and
//! an out-of-cohort athlete the sheets must not publish. Every assertion below reads the sheet the
//! workbook actually wrote, through `calamine`, exactly as the census workbook tests do.

use super::*;
use crate::bests;
use crate::store::{Store, Table};
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use census_domain::model::{
    normalize_name, AthleteId, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, CoachRole, CompetitionLevel, EventId,
    EventKind, Evidence, Gender, GradYear, Grade, Mark, MeetId, ObservedGrade, SchoolId,
    SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;

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

/// A school with an athletics site, appended once.
fn school(store: &Store, state: UsJurisdiction, name: &str) -> SchoolId {
    let (mut school, id) = CanonicalSchool::new(state, name, normalize_name(name));
    school.athletics_website = Some(format!("https://{}.test/athletics", name.to_lowercase()));
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
            school_year: SchoolYear(2025),
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
    let mut athlete = CanonicalAthlete::new(school, "Owen Clarke", GradYear(2028), Gender::Boys);
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
        SchoolYear(2026),
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
            Mark::TimeSeconds(49.80),
            "wiaa_results",
            "https://wiaa.test/results/state",
        ),
        (
            &state,
            &sprint,
            &EventKind::Track400m,
            "2026-06-06",
            Mark::TimeSeconds(49.71),
            "pttiming_live",
            "https://pttiming.test/live/state",
        ),
        (
            &invite,
            &sprint_invite,
            &EventKind::Track400m,
            "2026-05-01",
            Mark::TimeSeconds(48.55),
            "wiaa_results",
            "https://wiaa.test/results/invite",
        ),
        (
            &invite,
            &jump,
            &EventKind::LongJump,
            "2026-05-01",
            Mark::DistanceMetres(6.42),
            "wiaa_results",
            "https://wiaa.test/results/invite",
        ),
        (
            &state,
            &relay,
            &EventKind::Relay4x400,
            "2026-06-06",
            Mark::TimeSeconds(3.0),
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
        "yes",
        "the state meet was reported twice with different marks"
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
