#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use calamine::{open_workbook, Reader, Xlsx};
use census_crawl::net::FetchOutcome;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalSchool, CoachContactClaim, CoachContactProgram,
    CoachRole, CoachTenure, CoachTenureEvidence, Evidence, Gender, GradYear, PublishedGraduation,
    SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_report::{report::Scope, workbook};
use census_store::{Store, Table};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const EXTRACT: &[u8] = include_bytes!(
    "../../census-crawl/src/coach_directories/generic/fixtures/cac-contact-extract.html"
);

fn capture() -> FetchOutcome {
    FetchOutcome {
        url: "https://cacmustangs.org/about/contact/".to_owned(),
        response_url: None,
        method: "GET".to_owned(),
        status: 200,
        content_digest: format!("{:x}", Sha256::digest(EXTRACT)),
        bytes: EXTRACT.len(),
        fetched_at: "2026-10-07T12:34:56Z".to_owned(),
        from_cache: true,
        content_type: Some("text/html".to_owned()),
        body: EXTRACT.to_vec(),
    }
}

fn seed(store: &Store, school: &CanonicalSchool) -> TestResult {
    let source = SourceRef::new("school_sites", Some("https://cacmustangs.org/".to_owned()));
    store.append(Table::Schools, school)?;
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        "Contact Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(
            SourceNamespace::Other("fixture".to_owned()),
            "contact-runner",
        ),
    );
    athlete.sports = vec![Sport::OutdoorTrack];
    athlete
        .evidence
        .push(Evidence::parsed(source.clone(), "2026-10-07"));
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source,
    });
    store.append(Table::Athletes, &athlete)?;
    Ok(())
}

fn publish(store: &Store, out: PathBuf) -> TestResult<PathBuf> {
    Ok(workbook::build(
        store,
        &workbook::Options {
            grad_year: Some(2027),
            out: Some(out),
            limit: None,
            scope: Scope::AllSources,
            school_year: SchoolYear::new(2026).ok_or("year")?,
        },
    )?)
}

fn sheet_row(
    path: &Path,
    sheet: &str,
    identity: &str,
    value: &str,
) -> TestResult<BTreeMap<String, String>> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;
    let range = workbook.worksheet_range(sheet)?;
    let mut rows = range.rows();
    let headers = rows.next().ok_or("headers")?;
    let column = headers
        .iter()
        .position(|header| header.to_string() == identity)
        .ok_or("identity column")?;
    let row = rows
        .find(|row| {
            row.get(column)
                .is_some_and(|cell| cell.to_string() == value)
        })
        .ok_or("published row")?;
    Ok(headers
        .iter()
        .zip(row)
        .map(|(header, value)| (header.to_string(), value.to_string()))
        .collect())
}

fn csv_office(path: &Path) -> TestResult<BTreeMap<String, String>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    let purpose = headers
        .iter()
        .position(|header| header == "Purpose")
        .ok_or("purpose column")?;
    for row in reader.records() {
        let row = row?;
        if row.get(purpose) == Some("school_office") {
            return Ok(headers
                .iter()
                .zip(row.iter())
                .map(|(header, value)| (header.to_owned(), value.to_owned()))
                .collect());
        }
    }
    Err("office CSV row".into())
}

fn field<'a>(row: &'a BTreeMap<String, String>, key: &str) -> TestResult<&'a str> {
    row.get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing {key}").into())
}

fn assert_office(row: &BTreeMap<String, String>, capture: &FetchOutcome) -> TestResult {
    check!(eq; field(row, "Mailbox")?, "cac@cacmustangs.org");
    check!(eq; field(row, "Source URL")?, capture.url.as_str());
    check!(eq; field(row, "Capture SHA256")?, capture.content_digest.as_str());
    check!(eq; field(row, "Acquired At")?, "2026-10-07T12:34:56Z");
    Ok(())
}

#[test]
fn cen07_real_published_generic_capture_survives_store_workbook_csv_without_named_coaches(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Arkansas,
        "Central Arkansas Christian Schools",
        "central arkansas christian schools",
        None,
    );
    school.school_website = Some("https://cacmustangs.org/".to_owned());
    let capture = capture();
    census_crawl::coach_directories::apply_school_mailbox_capture(
        &mut school,
        SchoolYear::new(2026).ok_or("year")?,
        &capture,
    )?;
    seed(&store, &school)?;
    let path = publish(&store, dir.path().join("publication"))?;
    assert_office(
        &sheet_row(&path, "School Contacts", "Purpose", "school_office")?,
        &capture,
    )?;
    assert_office(
        &csv_office(
            &path
                .parent()
                .ok_or("publication root")?
                .join("school-contacts.csv"),
        )?,
        &capture,
    )?;
    let athlete = sheet_row(&path, "Athletes", "Name", "Contact Runner")?;
    check!(eq; field(&athlete, "Preferred Contact Email")?, "");
    check!(eq; field(&athlete, "AD Email")?, "");
    let research = sheet_row(&path, "Contact Research", "Subject", "school_office")?;
    check!(eq; field(&research, "Research Outcome")?, "completed_claims");
    workbook::publication::verify_published(&path)?;
    Ok(())
}

fn named_fixture(
    school: &CanonicalSchool,
    name: &str,
    role: CoachRole,
    mailbox: &str,
) -> TestResult<CanonicalCoach> {
    let sport = (role == CoachRole::HeadCoach).then_some(Sport::OutdoorTrack);
    let gender = if sport.is_some() {
        Gender::Boys
    } else {
        Gender::Unknown
    };
    let mut coach = CanonicalCoach::new(&school.id, name, sport, gender, role);
    coach.professional_email = Some(mailbox.to_owned());
    let program = if let Some(sport) = sport {
        CoachContactProgram::Team { sport, gender }
    } else {
        CoachContactProgram::SchoolAthletics
    };
    let source = SourceRef::new(
        "fixture",
        Some(format!(
            "https://example.test/cac/fixture/{}",
            role.stable_key()
        )),
    );
    let statement = format!(
        "{name} currently serves as {} for {} in 2026-27; contact {mailbox}",
        role.stable_key(),
        school.name
    );
    let fact = CoachTenureEvidence {
        tenure: CoachTenure::Current {
            school_year: SchoolYear::new(2026).ok_or("year")?,
        },
        source: source.clone(),
        source_sha256: format!("{:x}", Sha256::digest(statement.as_bytes())),
        retrieved_at: "2026-10-07T12:34:56Z".to_owned(),
        statement,
        claim: Some(CoachContactClaim {
            coach: coach.id.clone(),
            school: school.id.clone(),
            role,
            program,
            mailbox: Some(mailbox.to_owned()),
        }),
    };
    let mut appointment = fact.clone();
    if let Some(claim) = appointment.claim.as_mut() {
        claim.mailbox = None;
    }
    coach.tenure_evidence = vec![appointment, fact];
    coach.evidence.push(Evidence::parsed(source, "2026-10-07"));
    Ok(coach)
}

#[test]
fn cen07_three_independent_mailboxes_survive_real_publication() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Arkansas,
        "Central Arkansas Christian Schools",
        "central arkansas christian schools",
        None,
    );
    school.school_website = Some("https://cacmustangs.org/".to_owned());
    let capture = capture();
    census_crawl::coach_directories::apply_school_mailbox_capture(
        &mut school,
        SchoolYear::new(2026).ok_or("year")?,
        &capture,
    )?;
    seed(&store, &school)?;
    store.append(
        Table::Coaches,
        &named_fixture(
            &school,
            "Fixture Track Coach",
            CoachRole::HeadCoach,
            "head@school.edu",
        )?,
    )?;
    store.append(
        Table::Coaches,
        &named_fixture(
            &school,
            "Fixture Athletic Director",
            CoachRole::AthleticDirector,
            "ad@school.edu",
        )?,
    )?;
    let path = publish(&store, dir.path().join("publication"))?;
    let athlete = sheet_row(&path, "Athletes", "Name", "Contact Runner")?;
    check!(eq; field(&athlete, "Head TF Coach Email")?, "head@school.edu");
    check!(eq; field(&athlete, "AD Email")?, "ad@school.edu");
    check!(eq; field(&athlete, "Preferred Contact Email")?, "head@school.edu");
    assert_office(
        &sheet_row(&path, "School Contacts", "Purpose", "school_office")?,
        &capture,
    )?;
    assert_office(
        &csv_office(
            &path
                .parent()
                .ok_or("publication root")?
                .join("school-contacts.csv"),
        )?,
        &capture,
    )?;
    workbook::publication::verify_published(&path)?;
    Ok(())
}

#[test]
fn real_school_mailbox_capture_supersedes_never_attempted_after_store_reopen_and_publication(
) -> TestResult {
    use census_domain::model::{
        ContactResearch, ContactResearchOutcome, ContactResearchSubject, SchoolMailboxPurpose,
    };
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let store = Store::open(&root)?;
    let year = SchoolYear::new(2026).ok_or("year")?;
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Arkansas,
        "Central Arkansas Christian Schools",
        "central arkansas christian schools",
        None,
    );
    school.school_website = Some("https://cacmustangs.org/".to_owned());
    school.contact_research.push(ContactResearch {
        school: school.id.clone(),
        subject: ContactResearchSubject::SchoolMailbox(SchoolMailboxPurpose::SchoolOffice),
        school_year: year,
        outcome: ContactResearchOutcome::Unattempted,
        attempts: Vec::new(),
    });
    let capture = capture();
    census_crawl::coach_directories::apply_school_mailbox_capture(&mut school, year, &capture)?;
    seed(&store, &school)?;
    drop(store);
    let store = Store::open(&root)?;
    let path = publish(&store, dir.path().join("publication"))?;
    assert_office(
        &sheet_row(&path, "School Contacts", "Purpose", "school_office")?,
        &capture,
    )?;
    let research = sheet_row(&path, "Contact Research", "Subject", "school_office")?;
    check!(eq; field(&research, "Research Outcome")?, "completed_claims");
    workbook::publication::verify_published(&path)?;
    Ok(())
}
