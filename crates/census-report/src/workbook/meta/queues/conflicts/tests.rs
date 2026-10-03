use super::*;

use census_domain::model::{normalize_name, Gender, GradYear, SourceIdentity, SourceNamespace};
use census_domain::UsJurisdiction;

use census_review::athlete_flags::key;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name)).0
}

fn athlete(
    name: &str,
    school: &CanonicalSchool,
    gender: Gender,
    year: i16,
) -> TestResult<CanonicalAthlete> {
    let grad_year = GradYear::new(year).ok_or("invalid fixture graduation year")?;
    let source = SourceIdentity::new(
        SourceNamespace::Other("fixture".to_string()),
        format!(
            "{}:{}:{}:{year}",
            normalize_name(name),
            school.id.as_str(),
            gender.stable_key()
        ),
    );
    Ok(CanonicalAthlete::new(
        &school.id, name, grad_year, gender, source,
    ))
}

fn cases() -> TestResult<Vec<(&'static str, Vec<CanonicalAthlete>)>> {
    let west = school("Madison West High School");
    let east = school("Madison East High School");
    Ok(vec![
        (
            "one name, one school, one cohort: the two ids the merge kept apart",
            vec![
                athlete("Jordan Smith", &west, Gender::Boys, 2027)?,
                athlete("Jordan Smith", &west, Gender::Girls, 2027)?,
            ],
        ),
        (
            "one name as another source spelled it: an initial where the other row is complete",
            vec![
                athlete("Jordan Smith", &west, Gender::Boys, 2027)?,
                athlete("J. Smith", &west, Gender::Girls, 2027)?,
            ],
        ),
        (
            "one name at two schools: the school is part of the key, so there is nothing to retain",
            vec![
                athlete("Jordan Smith", &west, Gender::Boys, 2027)?,
                athlete("Jordan Smith", &east, Gender::Girls, 2027)?,
            ],
        ),
        (
            "one cohort against another: the class is part of the key",
            vec![
                athlete("Jordan Smith", &west, Gender::Boys, 2027)?,
                athlete("Jordan Smith", &west, Gender::Girls, 2026)?,
            ],
        ),
        (
            "a third row in the same class: the group is every row the key collides",
            vec![
                athlete("Jordan Smith", &west, Gender::Boys, 2027)?,
                athlete("Jordan Smith", &west, Gender::Girls, 2027)?,
                athlete("Jordan Smith", &west, Gender::Unknown, 2027)?,
            ],
        ),
        (
            "both rows outside the published cohort: the class filter drops them before the key",
            vec![
                athlete("Jordan Smith", &west, Gender::Boys, 2028)?,
                athlete("Jordan Smith", &west, Gender::Girls, 2028)?,
            ],
        ),
        (
            "one row in the cohort and one outside it: neither is retained",
            vec![
                athlete("Jordan Smith", &west, Gender::Boys, 2027)?,
                athlete("Jordan Smith", &west, Gender::Girls, 2028)?,
            ],
        ),
        (
            "a row nobody collides with: retained by nobody",
            vec![athlete("Sam Rivers", &west, Gender::Girls, 2027)?],
        ),
    ])
}

fn family_of(rows: &[CanonicalAthlete]) -> Family {
    athlete_identity(&cohort_of_2027(rows), &HashMap::new())
}

fn cohort_of_2027(rows: &[CanonicalAthlete]) -> Vec<CanonicalAthlete> {
    rows.iter()
        .filter(|athlete| athlete.grad_year == GradYear::CO2027)
        .cloned()
        .collect()
}

fn retained_by_key(rows: &[CanonicalAthlete]) -> Vec<String> {
    let cohort = cohort_of_2027(rows);
    let mut classes: BTreeMap<IdentityKey, Vec<&CanonicalAthlete>> = BTreeMap::new();
    for athlete in &cohort {
        classes.entry(key(athlete)).or_default().push(athlete);
    }
    let mut ids: Vec<String> = classes
        .into_values()
        .filter(|group| group.len() > 1)
        .flatten()
        .map(|athlete| athlete.id.to_string())
        .collect();
    ids.sort();
    ids
}

fn retained_by_writer(rows: &[CanonicalAthlete]) -> Vec<String> {
    let mut ids: Vec<String> = family_of(rows)
        .rows
        .iter()
        .map(|row| row.subject_id.clone())
        .collect();
    ids.sort();
    ids
}

fn class_members(rows: &[CanonicalAthlete], subject_id: &str) -> TestResult<Vec<String>> {
    let subject = rows
        .iter()
        .find(|athlete| athlete.id.as_str() == subject_id);
    subject
        .map(|subject| {
            let mut ids: Vec<String> = rows
                .iter()
                .filter(|athlete| key(athlete) == key(subject))
                .map(|athlete| athlete.id.to_string())
                .collect();
            ids.sort();
            ids
        })
        .ok_or_else(|| "retained row does not name a table row".into())
}

#[test]
fn the_family_retains_exactly_what_the_identity_key_groups() -> TestResult {
    for (what, rows) in cases()? {
        check!(eq; retained_by_writer(&rows),
        retained_by_key(&rows),
        "the queue and the key disagree on: {what}");
    }
    Ok(())
}

#[test]
fn every_retained_row_states_the_class_the_key_put_it_in() -> TestResult {
    for (what, rows) in cases()? {
        for row in family_of(&rows).rows {
            let members = class_members(&rows, row.subject_id.as_str())?;
            check!(
                members.len() > 1,
                "{what}: a retained row must be one of a class, not {members:?}"
            );
            for id in &members {
                check!(
                    row.detail.contains(id.as_str()),
                    "{what}: '{}' states every id in its class: {}",
                    row.subject_id,
                    row.detail
                );
            }
        }
    }
    Ok(())
}

#[test]
fn the_table_holds_cases_that_group_and_cases_that_do_not() -> TestResult {
    let cases = cases()?;
    let grouped = cases
        .iter()
        .filter(|(_, rows)| !retained_by_key(rows).is_empty())
        .count();
    check!(grouped > 0 && grouped < cases.len(),
    "the table must hold both answers, or the agreement above proves nothing: {grouped} of {} group",
    cases.len());
    Ok(())
}
