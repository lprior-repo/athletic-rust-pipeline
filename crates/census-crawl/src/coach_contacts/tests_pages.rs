use super::wire::CoachContactRow;
use super::*;
use census_domain::model::CanonicalCoach;
use census_store::{Store, Table};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const PAGES_CSV: &str =
    include_str!("../../tests/fixtures/coach_contacts/pages/coach_contacts_pages_2026-10-12.csv");
const PAGES_MANIFEST: &str =
    include_str!("../../tests/fixtures/coach_contacts/pages/capture-manifest-2026-10-12.json");
const XC_BOYS: &str = include_str!(
    "../../tests/fixtures/coach_contacts/pages/coach_records_wayzata_cross-country-running-boys_2026-10-12.json"
);
const XC_GIRLS: &str = include_str!(
    "../../tests/fixtures/coach_contacts/pages/coach_records_wayzata_cross-country-running-girls_2026-10-12.json"
);

const XC_BOYS_URL: &str = "https://www.mshsl.org/api/coaches/589008";
const XC_GIRLS_URL: &str = "https://www.mshsl.org/api/coaches/589009";

fn page_rows() -> TestResult<Vec<CoachContactRow>> {
    let mut reader = csv::Reader::from_reader(PAGES_CSV.as_bytes());
    Ok(reader
        .deserialize::<CoachContactRow>()
        .collect::<Result<_, _>>()?)
}

#[test]
fn captured_team_payloads_project_exactly_into_the_derived_rows() -> TestResult {
    let rows = page_rows()?;
    for (payload, url, sport) in [
        (XC_BOYS, XC_BOYS_URL, "Cross Country Running, Boys"),
        (XC_GIRLS, XC_GIRLS_URL, "Cross Country Running, Girls"),
    ] {
        let mut published: Vec<(String, String)> = crate::mshsl::parse_coach_records(payload)
            .into_iter()
            .filter(|record| crate::mshsl::is_published_level(&record.level))
            .map(|record| (record.name, String::from_iter(record.email)))
            .collect();
        let mut derived: Vec<(String, String)> = rows
            .iter()
            .filter(|row| row.source_url.as_str() == url && row.sport.as_str() == sport)
            .map(|row| {
                (
                    row.coach_name.clone(),
                    row.public_professional_email.clone(),
                )
            })
            .collect();
        published.sort();
        derived.sort();
        check!(!published.is_empty(), "no published record in {url}");
        check!(
            eq;
            derived,
            published,
            "the rows must be exactly the published level records of {url}, no more and no fewer"
        );
        check!(
            rows.iter()
                .filter(|row| row.source_url.as_str() == url)
                .all(|row| row.ad_name.is_empty() && row.ad_email.is_empty()),
            "no school-level address is asserted on the Wayzata rows: {url}"
        );
    }
    Ok(())
}

#[test]
fn captured_page_outcomes_decide_whether_rows_exist() -> TestResult {
    let manifest: serde_json::Value = serde_json::from_str(PAGES_MANIFEST)?;
    let captures = manifest["captures"].as_array().ok_or("captures")?;
    check!(captures.len() >= 13, "every page read is recorded");
    let rows = page_rows()?;
    for capture in captures {
        let url = capture["url"].as_str().ok_or("capture url")?;
        let outcome = capture["outcome"].as_str().ok_or("capture outcome")?;
        let declared = capture["rows"].as_u64().ok_or("capture rows")?;
        let seen = rows
            .iter()
            .filter(|row| row.source_url.as_str() == url)
            .count();
        let derived = u64::try_from(seen)?;
        check!(eq; derived, declared, "row count for {url}");
        if outcome == "contacts" {
            check!(
                derived > 0,
                "a capture with contacts produced no rows: {url}"
            );
        } else {
            check!(
                eq;
                derived,
                0,
                "a capture that published no conforming contact must not produce rows: {url}"
            );
        }
    }
    for required in [
        "blocked",
        "failed",
        "completed_empty",
        "no_contact_published",
    ] {
        let recorded = captures
            .iter()
            .any(|capture| capture["outcome"] == required);
        check!(recorded, "the honest outcome {required} is not recorded");
    }
    let unwalked = manifest["not_attempted"]
        .as_array()
        .is_some_and(|links| !links.is_empty());
    check!(
        unwalked,
        "the frontier that was not walked must be recorded, not left implicit"
    );
    Ok(())
}

#[test]
fn wiaa_addresses_decode_from_the_captured_payloads() -> TestResult {
    let manifest: serde_json::Value = serde_json::from_str(PAGES_MANIFEST)?;
    let rows = page_rows()?;
    let payloads = manifest["cfemail_payloads"]
        .as_array()
        .ok_or("cfemail_payloads")?;
    check!(payloads.len() >= 4, "the four obfuscated addresses");
    for entry in payloads {
        let href = entry["href_hex"].as_str().ok_or("href_hex")?;
        let attr = entry["attr_hex"].as_str().ok_or("attr_hex")?;
        let address = entry["address"].as_str().ok_or("address")?;
        let school = entry["school"].as_str().ok_or("school")?;
        let sport = entry["sport"].as_str().ok_or("sport")?;
        let column = entry["column"].as_str().ok_or("column")?;
        let from_href = crate::mshsl::decode_cfemail(href);
        let from_attr = crate::mshsl::decode_cfemail(attr);
        check!(
            eq;
            from_href.as_deref(),
            Some(address),
            "the email-protection payload must decode to the derived address"
        );
        check!(
            eq;
            from_attr.as_deref(),
            Some(address),
            "the data-cfemail payload must decode to the same address"
        );
        let found = rows.iter().find(|row| {
            if row.school.as_str() != school {
                return false;
            }
            if column == "ad_email" {
                row.role == "Athletic Director"
            } else {
                row.sport.as_str() == sport
            }
        });
        let row = found.ok_or("row for the decoded payload")?;
        let derived = if column == "ad_email" {
            &row.ad_email
        } else {
            &row.public_professional_email
        };
        check!(
            eq;
            derived.as_str(),
            address,
            "the derived column must be the decoded published address, never a name pattern"
        );
    }
    Ok(())
}

#[test]
fn captured_page_rows_import_and_unpublished_levels_stay_out() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let path = dir.path().join("coach-contacts-pages.csv");
    std::fs::write(&path, PAGES_CSV)?;
    let report = import_csv(&store, &path, "2026-10-12")?;
    let errors = report.errors;
    check!(eq; errors, 0, "every captured row must be usable");

    let coaches: Vec<CanonicalCoach> = store.scan::<CanonicalCoach>(Table::Coaches)?;
    let names: Vec<&str> = coaches.iter().map(|coach| coach.name.as_str()).collect();
    for expected in [
        "Mark Popp",
        "Brandon Heebink",
        "Kyle Rasmussen",
        "Aaron Berndt",
        "Jerry Popp",
        "Kalsey Beach",
        "Melissa Vanderlinde",
        "Katie Dillon",
        "Roberta Granberg",
        "Libby Seering",
        "JACOB KNAPMILLER",
        "Dillon Novak",
        "Alex Larson",
        "Toni Fowler",
        "Zeb Noyd",
        "Alan Frank",
    ] {
        check!(
            names.contains(&expected),
            "published person missing after import: {expected} in {names:?}"
        );
    }
    for excluded in [
        "Chad Bartels",
        "Matthew Klein",
        "Missa Morey",
        "Tyler Morey",
        "Denise Lutgen-Gallaty",
        "Brooke Benson",
        "Alyssa Buckner",
        "Ryan Bargender",
        "Melissa Pilgrim",
    ] {
        check!(
            !names.contains(&excluded),
            "a person outside the permitted roles was imported: {excluded} in {names:?}"
        );
    }
    let heebink = coaches
        .iter()
        .find(|coach| coach.name.as_str() == "Brandon Heebink")
        .ok_or("Brandon Heebink")?;
    check!(eq; heebink.professional_email.as_deref(), None);
    let unset = heebink.has_published_email();
    check!(
        !unset,
        "the publisher omitted a mailbox; no address may be inferred"
    );
    let fowler = coaches
        .iter()
        .find(|coach| coach.name.as_str() == "Toni Fowler")
        .ok_or("Toni Fowler")?;
    check!(eq; fowler.professional_email.as_deref(), None);
    Ok(())
}
