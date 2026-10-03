use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use census_crawl::ihsa;
use census_domain::model::{CanonicalCoach, CanonicalSchool, SchoolId};
use serde::Serialize;

use super::{assert_rollup, common, golden_case, stem_of, OBSERVED_ON};

#[derive(Serialize)]
struct SchoolRow {
    school_id: String,
    name_formal: String,
    name_ihsa: Option<String>,
    name_short: Option<String>,
    city: String,
    membership_type: Option<String>,
    kind: Option<String>,
    enrollment_type: Option<String>,
    has_boundary: Option<String>,
    is_cps: Option<String>,
    url: Option<String>,
}

impl SchoolRow {
    fn of(record: &ihsa::SchoolRecord) -> Self {
        let ihsa::SchoolRecord {
            school_id,
            name_formal,
            name_ihsa,
            name_short,
            city,
            membership_type,
            r#type,
            enrollment_type,
            has_boundary,
            is_cps,
            url,
        } = record;
        Self {
            school_id: school_id.clone(),
            name_formal: name_formal.clone(),
            name_ihsa: name_ihsa.clone(),
            name_short: name_short.clone(),
            city: city.clone(),
            membership_type: membership_type.clone(),
            kind: r#type.clone(),
            enrollment_type: enrollment_type.clone(),
            has_boundary: has_boundary.clone(),
            is_cps: is_cps.clone(),
            url: url.clone(),
        }
    }
}

#[derive(Serialize)]
struct StaffRow {
    person_id: i64,
    name: String,
    default_title: String,
    has_email: Option<bool>,
    last_name: Option<String>,
    role_id: Option<String>,
    phone: Option<String>,
    fax: Option<String>,
    email: Option<String>,
}

impl StaffRow {
    fn of(person: &ihsa::StaffPerson) -> Self {
        let ihsa::StaffPerson {
            person_id,
            name,
            default_title,
            has_email,
            last_name,
            role_id,
            phone,
            fax,
            email,
        } = person;
        Self {
            person_id: *person_id,
            name: name.clone(),
            default_title: default_title.clone(),
            has_email: *has_email,
            last_name: last_name.clone(),
            role_id: role_id.clone(),
            phone: phone.clone(),
            fax: fax.clone(),
            email: email.clone(),
        }
    }
}

#[derive(Serialize)]
struct SchoolPair {
    school: CanonicalSchool,
    school_id: SchoolId,
}

#[derive(Serialize)]
struct StaffFacts {
    staff: Vec<StaffRow>,
    coaches: Vec<CanonicalCoach>,
    paid_reveal_names: Vec<String>,
}

#[derive(Serialize)]
struct DirectoryFacts {
    records: Vec<SchoolRow>,
    schools: Vec<SchoolPair>,
}

fn ihsa_anchor_school(records: &[ihsa::SchoolRecord], school_id: &str) -> Result<SchoolPair> {
    let record = records
        .iter()
        .find(|record| record.school_id == school_id)
        .with_context(|| format!("the schools fixture carries school {school_id}"))?;
    let (school, id) = ihsa::parse_school(record, "https://api.ihsa.org/v1/schools", OBSERVED_ON)
        .context("the anchored school has a name")?;
    Ok(SchoolPair {
        school,
        school_id: id,
    })
}

fn ihsa_case(file: &str, body: &str) -> Result<(String, String)> {
    let stem = stem_of(file);
    let name = format!("ihsa__{stem}");
    let records = ihsa::parse_schools(&common::fixture("ihsa", "v1_schools.json")?)?;
    if stem.starts_with("v1_schools") {
        let mut schools = Vec::new();
        for record in &records {
            let (school, school_id) =
                ihsa::parse_school(record, "https://api.ihsa.org/v1/schools", OBSERVED_ON)
                    .context("every fixture school has a name")?;
            schools.push(SchoolPair { school, school_id });
        }
        let facts = DirectoryFacts {
            records: records.iter().map(SchoolRow::of).collect(),
            schools,
        };
        return golden_case(&name, &facts);
    }
    if stem.starts_with("staff2_") {
        let staff = ihsa::parse_staff(body)?;
        let anchor = ihsa_anchor_school(&records, "0101")?;
        let mut coaches = Vec::new();
        let mut paid_reveal_names = Vec::new();
        for person in &staff {
            if let Some(coach) = ihsa::parse_coach(
                person,
                &anchor.school_id,
                "https://api.ihsa.org/v1/schools/0101/staff2",
                OBSERVED_ON,
            ) {
                if person.has_email == Some(true) {
                    paid_reveal_names.push(coach.name.clone());
                }
                coaches.push(coach);
            }
        }
        let facts = StaffFacts {
            staff: staff.iter().map(StaffRow::of).collect(),
            coaches,
            paid_reveal_names,
        };
        return golden_case(&name, &facts);
    }
    bail!("fixture {file} matches no IHSA payload kind: v1_schools or staff2_")
}

#[test]
fn ihsa_fixtures_match_the_golden_corpus() -> Result<()> {
    let paths = common::fixtures("ihsa")?;
    let mut cases = BTreeMap::new();
    for path in &paths {
        let file = common::file_name(path)?;
        let (name, digest) = ihsa_case(&file, &common::fixture("ihsa", &file)?)?;
        cases.insert(name, digest);
    }
    assert_rollup("ihsa", cases, paths.len())
}
