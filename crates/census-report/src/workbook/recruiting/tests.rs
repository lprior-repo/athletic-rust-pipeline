use super::*;
use crate::bests;
use crate::report::Scope;
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use census_domain::model::{
    normalize_name, AthleteId, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, CentiMetres, CoachContactClaim,
    CoachContactProgram, CoachRole, CoachTenure, CoachTenureEvidence, CompetitionLevel, EventId,
    EventIdentity, EventKind, EventSpecification, Evidence, ExactSeconds, Gender, GradYear, Grade,
    Mark, MeetId, ObservedGrade, PublishedGraduation, SchoolId, SchoolYear, SourceIdentity,
    SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use rust_xlsxwriter::Workbook;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

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

fn publish_fixture_cohort(
    athlete: &mut CanonicalAthlete,
    provider: &str,
    purpose: &str,
    observed_on: &str,
) {
    let source = SourceRef::new(
        provider,
        Some(format!(
            "https://fixtures.test/recruiting/{purpose}/{}/2027",
            athlete.id
        )),
    );
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: source.clone(),
    });
    let mut claim_evidence = Evidence::parsed(source, observed_on);
    claim_evidence.note = Some("Synthetic public class-of-2027 fixture claim".to_owned());
    athlete.evidence.push(claim_evidence);
}

fn school(store: &Store, state: UsJurisdiction, name: &str) -> TestResult<SchoolId> {
    let (mut school, id) = CanonicalSchool::new(state, name, normalize_name(name), Some(name));
    school.athletics_website = Some(format!("https://{}.test/athletics", name.to_lowercase()));
    school.evidence = evidence("wiaa_results", Some("https://wiaa.test/schools"));
    store.append(Table::Schools, &school)?;
    Ok(id)
}

fn julian(store: &Store, school: &SchoolId) -> TestResult<(AthleteId, SourceIdentity)> {
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
            grade: Grade::new(11).ok_or("invalid fixture grade")?,
            school_year: SchoolYear::new(2025).ok_or("invalid fixture season")?,
            source: SourceRef::new("wiaa_results", None),
        });
        athlete.sports = vec![Sport::OutdoorTrack, Sport::CrossCountry];
        athlete.public_profile_urls = vec!["https://example.test/julian".to_string()];
        athlete.add_identity(
            SourceIdentity::new(SourceNamespace::athletic_net("athlete"), "123")
                .with_url("https://www.athletic.net/athlete/123"),
        );
        athlete.evidence = evidence("wiaa_results", None);
        store.append(Table::Athletes, &athlete)?;
    }
    Ok((id, source))
}

fn nadia(store: &Store, school: &SchoolId) -> TestResult<AthleteId> {
    let mut athlete = CanonicalAthlete::new(
        school,
        "Nadia Berger",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::Other("fixture".to_owned()), "nadia"),
    );
    athlete.sports = vec![Sport::CrossCountry];
    athlete.evidence = evidence("mshsl_results", None);
    publish_fixture_cohort(&mut athlete, "mshsl_results", "identity-only", DAY);
    store.append(Table::Athletes, &athlete)?;
    Ok(athlete.id)
}

fn younger(store: &Store, school: &SchoolId) -> TestResult {
    let mut athlete = CanonicalAthlete::new(
        school,
        "Owen Clarke",
        GradYear::new(2028).ok_or("invalid fixture graduation year")?,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::Other("fixture".to_owned()), "younger"),
    );
    athlete.evidence = evidence("wiaa_results", None);
    store.append(Table::Athletes, &athlete)?;
    Ok(())
}

fn meet(
    store: &Store,
    state: UsJurisdiction,
    name: &str,
    date: &str,
    level: CompetitionLevel,
    sport: Sport,
) -> TestResult<MeetId> {
    let mut meet = CanonicalMeet::new(Some(state), name, date, level);
    meet.sports = vec![sport];
    meet.evidence = evidence("wiaa_results", Some("https://wiaa.test/meets"));
    let id = meet.id.clone();
    store.append(Table::Meets, &meet)?;
    Ok(id)
}

fn event(store: &Store, meet: &MeetId, kind: EventKind) -> TestResult<EventId> {
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet,
            kind,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    event.evidence = evidence("wiaa_results", None);
    let id = event.id.clone();
    store.append(Table::Events, &event)?;
    Ok(id)
}

fn performance(
    store: &Store,
    context: &PerformanceRow<'_>,
    mark: Mark,
    source: &str,
    url: &str,
) -> TestResult {
    let school_year = SchoolYear::from_date(context.date).ok_or("invalid fixture result date")?;
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
    store.append(
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
    )?;
    let source_key = format!("{source}:{}:{}", context.date, mark.raw());
    store.append(
        Table::Performances,
        &CanonicalPerformance {
            id: CanonicalPerformance::mint(
                context.athlete,
                context.meet,
                context.event,
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
    )?;
    Ok(())
}

struct PerformanceRow<'a> {
    athlete: &'a AthleteId,
    school: &'a SchoolId,
    meet: &'a MeetId,
    event: &'a EventId,
    date: &'a str,
}

fn coach(
    store: &Store,
    school: &SchoolId,
    name: &str,
    sport: Option<Sport>,
    role: CoachRole,
    email: &str,
) -> TestResult {
    let mut coach = CanonicalCoach::new(school, name, sport, Gender::Mixed, role);
    coach.professional_email = Some(email.to_string());
    coach.evidence = evidence("coach_contacts_csv", Some("https://contacts.test/schools"));
    coach.tenure_evidence = vec![current_tenure(&coach, email)?];
    store.append(Table::Coaches, &coach)?;
    Ok(())
}

fn personal_coach(store: &Store, school: &SchoolId) -> TestResult {
    let mut coach = CanonicalCoach::new(
        school,
        "Morgan Personal",
        Some(Sport::IndoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    coach.personal_email = Some("morgan@gmail.com".to_string());
    coach.evidence = evidence("coach_contacts_csv", Some("https://contacts.test/schools"));
    coach.tenure_evidence = vec![current_tenure(&coach, "morgan@gmail.com")?];
    store.append(Table::Coaches, &coach)?;
    Ok(())
}

fn current_tenure(coach: &CanonicalCoach, mailbox: &str) -> TestResult<CoachTenureEvidence> {
    Ok(CoachTenureEvidence {
        tenure: CoachTenure::Current {
            school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        },
        source: SourceRef::new(
            "synthetic_directory",
            Some("https://contacts.test/schools".into()),
        ),
        source_sha256: "a".repeat(64),
        retrieved_at: "2026-09-20T00:00:00Z".into(),
        statement: "Synthetic academic-year appointment".into(),
        claim: Some(CoachContactClaim {
            coach: coach.id.clone(),
            school: coach.school.clone(),
            role: coach.role,
            program: match coach.role {
                CoachRole::AthleticDirector => CoachContactProgram::SchoolAthletics,
                _ => CoachContactProgram::Team {
                    sport: coach.sport.ok_or("fixture coach has no program")?,
                    gender: coach.gender,
                },
            },
            mailbox: Some(mailbox.to_string()),
        }),
    })
}

fn fixture() -> TestResult<Fixture> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let wi = school(&store, UsJurisdiction::Wisconsin, "Abbotsford")?;
    let mn = school(&store, UsJurisdiction::Minnesota, "Ada-Borup")?;
    let (julian, _) = julian(&store, &wi)?;
    let nadia = nadia(&store, &mn)?;
    younger(&store, &wi)?;

    let state = meet(
        &store,
        UsJurisdiction::Wisconsin,
        "WIAA Division 3 State",
        "2026-06-06",
        CompetitionLevel::State,
        Sport::OutdoorTrack,
    )?;
    let invite = meet(
        &store,
        UsJurisdiction::Wisconsin,
        "Abbotsford Invitational",
        "2026-05-01",
        CompetitionLevel::Invitational,
        Sport::OutdoorTrack,
    )?;
    let sprint = event(&store, &state, EventKind::Track400m)?;
    let sprint_invite = event(&store, &invite, EventKind::Track400m)?;
    let jump = event(&store, &invite, EventKind::LongJump)?;
    let relay = event(&store, &state, EventKind::Relay4x400)?;

    for (meet, event, date, mark, source, url) in [
        (
            &state,
            &sprint,
            "2026-06-06",
            Mark::TimeSeconds(ExactSeconds::parse("49.80")?),
            "wiaa_results",
            "https://wiaa.test/results/state",
        ),
        (
            &state,
            &sprint,
            "2026-06-06",
            Mark::TimeSeconds(ExactSeconds::parse("49.71")?),
            "pttiming_live",
            "https://pttiming.test/live/state",
        ),
        (
            &invite,
            &sprint_invite,
            "2026-05-01",
            Mark::TimeSeconds(ExactSeconds::parse("48.55")?),
            "wiaa_results",
            "https://wiaa.test/results/invite",
        ),
        (
            &invite,
            &jump,
            "2026-05-01",
            Mark::DistanceMetres(CentiMetres::new(762)),
            "athleticlive_athletes",
            "https://athleticlive.test/athletes/999",
        ),
        (
            &state,
            &relay,
            "2026-06-06",
            Mark::TimeSeconds(ExactSeconds::parse("319.50")?),
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
                date,
            },
            mark,
            source,
            url,
        )?;
    }

    coach(
        &store,
        &wi,
        "Paula Johnson",
        Some(Sport::OutdoorTrack),
        CoachRole::HeadCoach,
        "pj@abbotsford.test",
    )?;
    coach(
        &store,
        &wi,
        "Teresa Johnson",
        Some(Sport::CrossCountry),
        CoachRole::AssistantCoach,
        "tj@abbotsford.test",
    )?;
    coach(
        &store,
        &mn,
        "Erik Lund",
        Some(Sport::CrossCountry),
        CoachRole::HeadCoach,
        "elund@adaborup.test",
    )?;

    Ok(Fixture {
        dir,
        store,
        julian,
        nadia,
        wi_school: wi,
    })
}

fn recruiting(store: &Store, scope: Scope, grad_year: Option<i16>) -> TestResult<Recruiting> {
    let dataset = crate::export::ExportDataset::load(store)?;
    let derivation = crate::report::Derivation::of(&dataset, scope, grad_year);
    let prs = bests::build_from_dataset(
        &dataset,
        &bests::Options {
            scope,
            grad_year,
            limit: None,
        },
    );
    Ok(Recruiting::of(
        &derivation,
        SchoolYear::new(2026).ok_or("invalid fixture season")?,
        prs,
    )?)
}

fn verified_publication(store: &Store, expected_athletes: u64) -> TestResult {
    let options = crate::workbook::Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
    };
    let published = crate::workbook::build(store, &options)?;
    let dataset = crate::export::ExportDataset::load(store)?;
    let verified = crate::workbook::verify::verify_frozen(&published, &dataset, &options)?;
    check!(eq; verified.mapped_athletes, expected_athletes);
    Ok(())
}

fn written(
    fixture: &Fixture,
) -> TestResult<(Xlsx<std::io::BufReader<std::fs::File>>, std::path::PathBuf)> {
    written_scope(fixture, Scope::Core)
}

fn written_scope(
    fixture: &Fixture,
    scope: Scope,
) -> TestResult<(Xlsx<std::io::BufReader<std::fs::File>>, std::path::PathBuf)> {
    let projection = recruiting(&fixture.store, scope, Some(2027))?;
    let path = fixture.dir.path().join("recruiting.xlsx");
    let mut book = Workbook::new();
    projection.write_athletes(&mut book, &path)?;
    projection.write_prs(&mut book, &path)?;
    projection.write_coaches(&mut book, &path)?;
    book.save(&path)?;
    Ok((open_workbook(&path)?, path))
}

fn text(range: &Range<Data>, row: usize, column: usize) -> String {
    range
        .get((row, column))
        .map(|value| value.to_string())
        .map_or(Default::default(), core::convert::identity)
}

fn row_of(range: &Range<Data>, key: &str) -> TestResult<usize> {
    row_where(range, |row| text(range, row, 0) == key)
}

fn row_of_event(range: &Range<Data>, athlete: &str, event: &str) -> TestResult<usize> {
    row_where(range, |row| {
        text(range, row, 0) == athlete && text(range, row, 7) == event
    })
}

fn column_of(range: &Range<Data>, header: &str) -> TestResult<usize> {
    (0..range.width())
        .find(|column| text(range, 0, *column) == header)
        .ok_or_else(|| format!("missing sheet column {header}").into())
}

fn column_carries(range: &Range<Data>, column: usize, value: &str) -> bool {
    (1..range.height()).any(|row| text(range, row, column) == value)
}

fn row_where(range: &Range<Data>, matches: impl Fn(usize) -> bool) -> TestResult<usize> {
    (1..range.height())
        .find(|row| matches(*row))
        .ok_or_else(|| "missing published sheet row".into())
}

fn sheet(
    book: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    name: &str,
) -> TestResult<Range<Data>> {
    Ok(book.worksheet_range(name)?)
}

#[test]
fn the_three_recruiting_sheets_are_named_after_the_objective() -> TestResult {
    let fixture = fixture()?;
    let (book, path) = written(&fixture)?;
    let mut names = book.sheet_names().to_vec();
    names.sort();
    check!(eq; names, vec!["Athletes", "Coaches", "PRs"]);
    check!(path.exists());
    Ok(())
}

#[test]
fn the_athletes_sheet_publishes_the_objective_columns_and_the_stored_facts() -> TestResult {
    let fixture = fixture()?;
    let (mut book, _) = written(&fixture)?;
    let range = sheet(&mut book, "Athletes")?;
    let row = row_of(&range, fixture.julian.as_str())?;
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
        check!(eq; text(&range, row, column_of(&range, header)?),
        value,
        "{header}");
    }
    Ok(())
}

#[test]
fn the_prs_sheet_retains_shared_winners_units_dates_and_provenance() -> TestResult {
    let fixture = fixture()?;
    let (mut book, _) = written(&fixture)?;
    let range = sheet(&mut book, "PRs")?;

    let dataset = crate::export::ExportDataset::load(&fixture.store)?;
    let canonical = bests::build_from_dataset(
        &dataset,
        &bests::Options {
            scope: Scope::Core,
            grad_year: Some(2027),
            limit: None,
        },
    );
    check!(eq; range.height() - 1,
    canonical.len(),
    "one row per core-scope athlete/event");

    let sprint = row_of_event(&range, fixture.julian.as_str(), "Track400m")?;
    check!(eq; text(&range, sprint, 1), "Julian Aguilera");
    check!(eq; text(&range, sprint, 2), "Boys");
    check!(eq; text(&range, sprint, 3), "Abbotsford");
    check!(eq; text(&range, sprint, 4), "WI");
    check!(eq; text(&range, sprint, 5), "2027");
    check!(eq; text(&range, sprint, 6), "Track");
    check!(eq; text(&range, sprint, 9),
    "48.55",
    "the fastest of the three marks");
    check!(eq; text(&range, sprint, 10), "48.55");
    check!(eq; text(&range, sprint, 11), "s");
    check!(eq; text(&range, sprint, 12), "", "no wind was stored");
    check!(eq; text(&range, sprint, 13), "2026-05-01");
    check!(eq; text(&range, sprint, 14), "Abbotsford Invitational");
    check!(eq; text(&range, sprint, 15), "", "no place was stored");
    check!(eq; text(&range, sprint, 16), "https://wiaa.test/results/invite");
    check!(eq; text(&range, sprint, 17),
    "2",
    "two sources report this event");
    check!(eq; text(&range, sprint, 18),
    "WIAA Division 3 State: 49.71 | 49.80",
    "the losing marks stay attached to the winner");
    check!(eq; text(&range, sprint, 19), "outdoor");
    check!(eq; text(&range, sprint, 20), "na");
    check!(eq; text(&range, sprint, 21), "unknown");
    check!(text(&range, sprint, 22).starts_with("perf_"));
    check!(text(&range, sprint, 23).starts_with("meet_"));
    check!(eq; text(&range, sprint, 24), "wiaa_results:2026-05-01:time");

    let (mut all, _) = written_scope(&fixture, Scope::AllSources)?;
    let all_range = sheet(&mut all, "PRs")?;
    check!(eq; all_range.height(),
    range.height() + 1,
    "the athleticlive jump PR joins the sheet only outside the core scope");
    let jump = row_of_event(&all_range, fixture.julian.as_str(), "LongJump")?;
    check!(eq; text(&all_range, jump, 6), "Field");
    check!(eq; text(&all_range, jump, 9), "7.62 m");
    check!(eq; text(&all_range, jump, 10), "7.62");
    check!(eq; text(&all_range, jump, 11), "m");
    check!(eq; text(&all_range, jump, 17), "1");
    check!(eq; text(&all_range, jump, 18), "", "one report cannot conflict");
    Ok(())
}

#[test]
fn the_coaches_sheet_publishes_the_school_contact_graph() -> TestResult {
    let fixture = fixture()?;
    let (mut book, _) = written(&fixture)?;
    let range = sheet(&mut book, "Coaches")?;
    let coach = column_of(&range, "Coach")?;
    let row = row_where(&range, |row| text(&range, row, coach) == "Paula Johnson")?;
    let expected = [
        ("Sport", Sport::OutdoorTrack.stable_key()),
        ("Role", CoachRole::HeadCoach.stable_key()),
        ("Professional Email", "pj@abbotsford.test"),
    ];
    for (header, value) in expected {
        check!(eq; text(&range, row, column_of(&range, header)?),
        value,
        "{header}");
    }
    Ok(())
}

#[test]
fn an_athlete_without_a_performance_is_published_as_identity_only() -> TestResult {
    let fixture = fixture()?;
    let (mut book, _) = written(&fixture)?;
    let range = sheet(&mut book, "Athletes")?;
    let row = row_of(&range, fixture.nadia.as_str())?;
    check!(eq; text(&range, row, column_of(&range, "Event list")?), "");
    check!(eq; text(&range, row, column_of(&range, "Headline PR summary")?),
    "");
    Ok(())
}

#[test]
fn the_url_columns_follow_the_evidence_scope() -> TestResult {
    let fixture = fixture()?;
    let (mut book, _) = written_scope(&fixture, Scope::AllSources)?;
    let range = sheet(&mut book, "Athletes")?;
    let row = row_of(&range, fixture.julian.as_str())?;
    check!(eq; text(&range, row, column_of(&range, "Athletic.net URL")?),
    "https://www.athletic.net/athlete/123");
    check!(eq; text(&range, row, column_of(&range, "MileSplit URL")?),
    "https://wi.milesplit.com/athletes/999");
    check!(eq; text(&range, row, column_of(&range, "Other profile URLs")?),
    "https://example.test/julian");
    Ok(())
}

#[test]
fn an_unrelated_programmes_personal_address_is_retained_only_in_raw_coach_history() -> TestResult {
    let fixture = fixture()?;
    personal_coach(&fixture.store, &fixture.wi_school)?;
    let (mut book, _) = written(&fixture)?;
    let range = sheet(&mut book, "Coaches")?;
    let coach = column_of(&range, "Coach")?;
    let professional = column_of(&range, "Professional Email")?;
    let personal = column_of(&range, "Personal Email")?;
    check!(
        !column_carries(&range, professional, "morgan@gmail.com"),
        "the personal address never becomes a professional contact"
    );
    let row = row_where(&range, |row| text(&range, row, coach) == "Morgan Personal")?;
    check!(eq; text(&range, row, professional), "");
    check!(eq; text(&range, row, personal), "morgan@gmail.com");
    Ok(())
}

#[test]
fn the_run_audits_every_sheet_against_the_store_counts_behind_it() -> TestResult {
    let fixture = fixture()?;
    let projection = recruiting(&fixture.store, Scope::Core, Some(2027))?;
    let audit = projection.dataset.audit();
    check!(eq; audit.store_athletes, 3);
    check!(eq; audit.cohort_athletes, 2);
    check!(eq; audit.pr_rows, 1);
    check!(eq; audit.coach_rows, 3);
    check!(eq; audit.published_coach_rows, 3);
    check!(eq; audit.contact_conflicts, 0);
    Ok(())
}

#[test]
fn duplicate_coach_observations_publish_one_row() -> TestResult {
    let fixture = fixture()?;
    let first = CanonicalCoach::new(
        &fixture.wi_school,
        "Duplicate Tester",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    let second = CanonicalCoach::new(
        &fixture.wi_school,
        "Duplicate Tester",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    fixture.store.append(Table::Coaches, &first)?;
    fixture.store.append(Table::Coaches, &second)?;
    let projection = recruiting(&fixture.store, Scope::Core, Some(2027))?;
    check!(eq; projection.dataset.published_coaches.len(), 4);
    let duplicate_ids: Vec<_> = projection
        .dataset
        .published_coaches
        .iter()
        .filter(|c| c.name == "Duplicate Tester")
        .map(|c| c.id.clone())
        .collect();
    check!(eq; duplicate_ids.len(), 1);
    Ok(())
}

#[test]
fn merged_coach_email_routes_into_published_row() -> TestResult {
    let fixture = fixture()?;
    let first = CanonicalCoach::new(
        &fixture.wi_school,
        "Merge Tester",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    fixture.store.append(Table::Coaches, &first)?;
    let second = CanonicalCoach::new(
        &fixture.wi_school,
        "Merge Tester",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    let mut second = second;
    second.professional_email = Some("merge-later@example.test".to_string());
    fixture.store.append(Table::Coaches, &second)?;
    let projection = recruiting(&fixture.store, Scope::Core, Some(2027))?;
    let merged = projection
        .dataset
        .published_coaches
        .iter()
        .find(|c| c.name == "Merge Tester")
        .ok_or("merged coach not found")?;
    check!(eq; merged.professional_email, Some("merge-later@example.test".to_string()));
    Ok(())
}

#[test]
fn distinct_coach_ids_produce_distinct_published_rows() -> TestResult {
    let fixture = fixture()?;
    let first = CanonicalCoach::new(
        &fixture.wi_school,
        "Distinct One",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    fixture.store.append(Table::Coaches, &first)?;
    let second = CanonicalCoach::new(
        &fixture.wi_school,
        "Distinct One",
        Some(Sport::CrossCountry),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    fixture.store.append(Table::Coaches, &second)?;
    let projection = recruiting(&fixture.store, Scope::Core, Some(2027))?;
    let distinct_names: Vec<_> = projection
        .dataset
        .published_coaches
        .iter()
        .filter(|c| c.name == "Distinct One")
        .collect();
    check!(eq; distinct_names.len(), 2);
    Ok(())
}

#[test]
fn every_sheet_publishes_hostile_source_text_as_literal_cells() -> TestResult {
    const FORMULA_SCHOOL: &str = "=cmd|' /C calc'!A0";
    const FORMULA_CITY: &str = "\u{feff}+1+1";
    const FORMULA_ATHLETE: &str = "-2+3";
    const FORMULA_COACH: &str = "@SUM(1)";
    const FORMULA_EMAIL: &str = "=2+2@contacts.test";
    const TOKENS: [&str; 5] = [
        FORMULA_SCHOOL,
        FORMULA_CITY,
        FORMULA_ATHLETE,
        FORMULA_COACH,
        FORMULA_EMAIL,
    ];

    let fixture = fixture()?;
    let store = &fixture.store;
    let (mut school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        FORMULA_SCHOOL,
        normalize_name(FORMULA_SCHOOL),
        Some(FORMULA_CITY),
    );
    school.athletics_website = Some("https://schools.test/athletics".to_string());
    school.evidence = evidence("wiaa_results", Some("https://wiaa.test/schools"));
    store.append(Table::Schools, &school)?;
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        FORMULA_ATHLETE,
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "hostile-text"),
    );
    athlete.sports = vec![Sport::OutdoorTrack];
    publish_fixture_cohort(&mut athlete, "wiaa_results", "hostile-text", DAY);
    store.append(Table::Athletes, &athlete)?;
    coach(
        store,
        &school_id,
        FORMULA_COACH,
        Some(Sport::OutdoorTrack),
        CoachRole::HeadCoach,
        FORMULA_EMAIL,
    )?;

    let options = crate::workbook::Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
    };
    let published = crate::workbook::build(store, &options)?;
    let frozen = published
        .parent()
        .ok_or("publication generation")?
        .join("frozen-input.json");
    let dataset = crate::export::ExportDataset::reopen_frozen(&frozen)?;
    crate::workbook::verify::verify_frozen(&published, &dataset, &options)?;
    crate::workbook::publication::verify_published(&published)?;

    let mut book: Xlsx<std::io::BufReader<std::fs::File>> = open_workbook(&published)?;
    let names = book.sheet_names().to_vec();
    for expected in ["Athletes", "Coaches", "PRs"] {
        check!(
            names.iter().any(|name| name == expected),
            "missing sheet {expected}"
        );
    }
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for name in &names {
        let range = book.worksheet_range(name)?;
        for row in 0..range.height() {
            for column in 0..range.width() {
                let Some(value) = range.get((row, column)) else {
                    continue;
                };
                match value {
                    Data::Error(error) => {
                        return Err(format!(
                            "{name} R{}C{} is a spreadsheet error: {error:?}",
                            row + 1,
                            column + 1
                        )
                        .into());
                    }
                    Data::String(text) => {
                        for token in TOKENS {
                            if text.contains(token) {
                                seen.insert(token);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    check!(eq; seen.len(), TOKENS.len(), "every hostile token stays literal string text");
    Ok(())
}
