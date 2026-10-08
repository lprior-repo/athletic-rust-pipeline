use super::map::{school_entities, ProfileFacts};
use super::parse::parse_staff_directory;
use super::{DIRECTORY_URL, HOST};
use crate::CrawlError;
use census_domain::model::{CoachRole, Gender, Sport};
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FIXTURE: &str =
    include_str!("../../tests/fixtures/sidearm_staff/gomats.org__staff-directory__full.html");

fn page(rows: &str) -> String {
    format!("<h1>Example High School</h1><article class='sidearm-staff'><table>{rows}</table></article>")
}

fn member(id: &str, sport: &str, role: &str, script: &str) -> String {
    format!(
        "<tr class='sidearm-staff-member' data-member-id='{id}' data-category-id='xc'>\
         <td headers='col-fullname category-xc'><a>Coach {id}</a></td>\
         <td headers='col-staff_custom_1 category-xc'>{sport}</td>\
         <td headers='col-staff_custom_2 category-xc'>Varsity</td>\
         <td headers='col-staff_title category-xc'>{role}</td>\
         <td headers='col-staff_email category-xc'><script>{script}</script></td></tr>"
    )
}

#[test]
fn fixture_keeps_cross_country_track_and_school_director_contacts() -> TestResult {
    let directory = parse_staff_directory(FIXTURE)?;
    check!(eq; directory.name.as_str(), "Miramonte High School");
    let extract = school_entities(
        &ProfileFacts {
            state: UsJurisdiction::California,
            host: HOST,
            url: DIRECTORY_URL,
            observed_on: "2026-10-04",
        },
        &directory,
    );
    check!(eq; extract.school.state, Some(UsJurisdiction::California));
    check!(eq; extract.school.city, None);
    let actual: Vec<_> = extract
        .coaches
        .iter()
        .map(|coach| {
            (
                coach.name.as_str(),
                coach.sport,
                coach.role,
                coach.gender,
                coach.professional_email.as_deref(),
                coach.personal_email.as_deref(),
            )
        })
        .collect();
    check!(eq; actual, vec![
        ("Sean Hennessy", None, CoachRole::AthleticDirector, Gender::Unknown,
         Some("shennessy@auhsdschools.org"), None),
        ("Brian Henderson", Some(Sport::CrossCountry), CoachRole::HeadCoach, Gender::Unknown,
         Some("bhenderson@auhsdschools.org"), None),
        ("Robert Kennedy", Some(Sport::OutdoorTrack), CoachRole::HeadCoach, Gender::Unknown,
         None, Some("caljumper259@gmail.com")),
    ]);
    check!(extract
        .coaches
        .iter()
        .all(|coach| coach.school == extract.school.id));
    Ok(())
}

#[test]
fn fixture_trims_published_email_halves_and_preserves_other_sports() -> TestResult {
    let directory = parse_staff_directory(FIXTURE)?;
    let row = directory
        .members
        .iter()
        .find(|row| row.id == "168")
        .ok_or("Kennedy's flag-football row must be retained")?;
    check!(eq; row.name.as_str(), "Robert Kennedy");
    check!(eq; row.sport.as_str(), "Flag Football");
    check!(eq; row.level.as_str(), "Varsity");
    check!(eq; row.email.as_str(), "caljumper259@gmail.com");
    let padded = directory
        .members
        .iter()
        .find(|row| row.id == "157")
        .ok_or("Ace Wright's trailing-space mailbox must be retained")?;
    check!(eq; padded.name.as_str(), "Ace Wright");
    check!(eq; padded.email.as_str(), "antuanishawright@gmail.com");
    check!(directory
        .members
        .iter()
        .all(|row| row.email == row.email.trim()));
    check!(directory
        .members
        .iter()
        .any(|row| row.email == "caljumper259@gmail.com" && row.id == "160"));
    Ok(())
}

#[test]
fn missing_and_blank_email_rows_retain_published_staff_appointments() -> TestResult {
    let body = page(
        &[
            member("missing", "Cross Country", "Head Coach", ""),
            member(
                "blank",
                "Cross Country",
                "Head Coach",
                "var firstHalf = \" \"; var secondHalf = \"school.org\";",
            ),
            member(
                "kept",
                "Cross Country",
                "Head Coach",
                "var firstHalf = \" coach \"; var secondHalf = \"school.org  \";",
            ),
        ]
        .concat(),
    );
    let directory = parse_staff_directory(&body)?;
    let actual: Vec<_> = directory
        .members
        .iter()
        .map(|row| (row.id.as_str(), row.email.as_str()))
        .collect();
    check!(eq; actual, vec![("missing", ""), ("blank", ""), ("kept", "coach@school.org")]);
    Ok(())
}

#[test]
fn email_halves_never_pair_across_member_or_category_rows() -> TestResult {
    let body = page(&[
        member("first", "Cross Country", "Head Coach", "var firstHalf = \"wrong\";"),
        "<tr class='sidearm-staff-category' data-category-id='xc'><th>Cross Country<script>var secondHalf = \"wrong.org\";</script></th></tr>".to_string(),
        member("second", "Cross Country", "Head Coach", "var secondHalf = \"wrong.org\";"),
        member("correct", "Cross Country", "Head Coach", "var firstHalf = \"correct\"; var secondHalf = \"school.org\";"),
    ].concat());
    let directory = parse_staff_directory(&body)?;
    let actual: Vec<_> = directory
        .members
        .iter()
        .map(|row| (row.id.as_str(), row.email.as_str()))
        .collect();
    check!(eq; actual, vec![("first", ""), ("second", ""), ("correct", "correct@school.org")]);
    Ok(())
}

#[test]
fn category_sport_is_fallback_only_and_director_title_is_exact() -> TestResult {
    let body = page(&[
        "<tr class='sidearm-staff-category' data-category-id='xc'><th>CROSS COUNTRY (FALL)</th></tr>".to_string(),
        member("fallback", "", "Head Coach", "var firstHalf = \"fallback\"; var secondHalf = \"school.org\";"),
        member("explicit", "Track &amp; Field", "Head Coach", "var firstHalf = \"explicit\"; var secondHalf = \"school.org\";"),
        member("assistant", "", "Assistant Athletic Director", "var firstHalf = \"assistant\"; var secondHalf = \"school.org\";"),
    ].concat());
    let directory = parse_staff_directory(&body)?;
    check!(eq; directory.name.as_str(), "Example High School");
    let extract = school_entities(
        &ProfileFacts {
            state: UsJurisdiction::California,
            host: HOST,
            url: DIRECTORY_URL,
            observed_on: "2026-10-04",
        },
        &directory,
    );
    let actual: Vec<_> = extract
        .coaches
        .iter()
        .map(|coach| (coach.name.as_str(), coach.sport))
        .collect();
    check!(eq; actual, vec![("Coach fallback", Some(Sport::CrossCountry)), ("Coach explicit", Some(Sport::OutdoorTrack))]);
    Ok(())
}

#[test]
fn non_sidearm_body_returns_schema_error() -> TestResult {
    let result = parse_staff_directory("<h1>Example High School</h1><p>sidearm-staff</p>");
    check!(matches!(result, Err(CrawlError::Schema { .. })));
    Ok(())
}

#[test]
fn directory_without_school_name_returns_schema_error() -> TestResult {
    let result = parse_staff_directory("<article class='sidearm-staff'><table></table></article>");
    check!(matches!(result, Err(CrawlError::Schema { .. })));
    Ok(())
}
