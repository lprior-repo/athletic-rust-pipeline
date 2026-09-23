//! The athlete family against the one definition of its key.
//!
//! The family retains the rows a key groups, and the key is `identity::athlete_flags::key` — the same
//! function the identity lane states to a model as `name_school_cohort_agree`. A second rule derived
//! here instead cannot be caught by either module's own tests: the lane would keep asking about the
//! groups the queue mints, while the flag that says the rows agree on their key quietly stopped
//! arriving. So the table below asserts the two agree row by row, case by case: reintroduce a
//! grouping rule beside the key and the family stops matching the key's own classes.

use super::*;

use census_domain::model::{normalize_name, Gender, GradYear};
use census_domain::UsJurisdiction;

use census_review::athlete_flags::key;

/// A school the table can name, minted the way the store mints one.
fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name)).0
}

/// One athlete in the table, at one school, in one cohort.
fn athlete(name: &str, school: &CanonicalSchool, gender: Gender, year: i16) -> CanonicalAthlete {
    CanonicalAthlete::new(
        &school.id,
        name,
        GradYear::new(year).expect("a year inside the accepted window"),
        gender,
    )
}

/// The cases: what each table of rows is for, and the rows themselves.
fn cases() -> Vec<(&'static str, Vec<CanonicalAthlete>)> {
    let west = school("Madison West High School");
    let east = school("Madison East High School");
    vec![
        (
            "one name, one school, one cohort: the two ids the merge kept apart",
            vec![
                athlete("Jordan Smith", &west, Gender::Boys, 2027),
                athlete("Jordan Smith", &west, Gender::Girls, 2027),
            ],
        ),
        (
            "one name as another source spelled it: an initial where the other row is complete",
            vec![
                athlete("Jordan Smith", &west, Gender::Boys, 2027),
                athlete("J. Smith", &west, Gender::Girls, 2027),
            ],
        ),
        (
            "one name at two schools: the school is part of the key, so there is nothing to retain",
            vec![
                athlete("Jordan Smith", &west, Gender::Boys, 2027),
                athlete("Jordan Smith", &east, Gender::Girls, 2027),
            ],
        ),
        (
            "one cohort against another: the class is part of the key",
            vec![
                athlete("Jordan Smith", &west, Gender::Boys, 2027),
                athlete("Jordan Smith", &west, Gender::Girls, 2026),
            ],
        ),
        (
            "a third row in the same class: the group is every row the key collides",
            vec![
                athlete("Jordan Smith", &west, Gender::Boys, 2027),
                athlete("Jordan Smith", &west, Gender::Girls, 2027),
                athlete("Jordan Smith", &west, Gender::Unknown, 2027),
            ],
        ),
        (
            "both rows outside the published cohort: the class filter drops them before the key",
            vec![
                athlete("Jordan Smith", &west, Gender::Boys, 2028),
                athlete("Jordan Smith", &west, Gender::Girls, 2028),
            ],
        ),
        (
            "one row in the cohort and one outside it: neither is retained",
            vec![
                athlete("Jordan Smith", &west, Gender::Boys, 2027),
                athlete("Jordan Smith", &west, Gender::Girls, 2028),
            ],
        ),
        (
            "a row nobody collides with: retained by nobody",
            vec![athlete("Sam Rivers", &west, Gender::Girls, 2027)],
        ),
    ]
}

/// The family the writer publishes for one table of athlete rows.
fn family_of(rows: &[CanonicalAthlete]) -> Family {
    let store = StoreRows {
        schools: Vec::new(),
        meets: Vec::new(),
        athletes: rows.to_vec(),
        coaches: Vec::new(),
    };
    athlete_identity(&store, &HashMap::new())
}

/// The ids the key says are collisions: every member of a class the published cohort holds twice.
fn retained_by_key(rows: &[CanonicalAthlete]) -> Vec<String> {
    let mut classes: BTreeMap<IdentityKey, Vec<&CanonicalAthlete>> = BTreeMap::new();
    for athlete in class_of_2027(rows) {
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

/// The ids the writer retained, read off the family it published.
fn retained_by_writer(rows: &[CanonicalAthlete]) -> Vec<String> {
    let mut ids: Vec<String> = family_of(rows)
        .rows
        .iter()
        .map(|row| row.subject_id.clone())
        .collect();
    ids.sort();
    ids
}

/// The ids of every row sharing `subject_id`'s key: the class a retained row's detail names.
fn class_members(rows: &[CanonicalAthlete], subject_id: &str) -> Vec<String> {
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
        .expect("a retained row names a row of the table")
}

#[test]
fn the_family_retains_exactly_what_the_identity_key_groups() {
    for (what, rows) in cases() {
        assert_eq!(
            retained_by_writer(&rows),
            retained_by_key(&rows),
            "the queue and the key disagree on: {what}"
        );
    }
}

#[test]
fn every_retained_row_states_the_class_the_key_put_it_in() {
    for (what, rows) in cases() {
        for row in family_of(&rows).rows {
            let members = class_members(&rows, row.subject_id.as_str());
            assert!(
                members.len() > 1,
                "{what}: a retained row must be one of a class, not {members:?}"
            );
            for id in &members {
                assert!(
                    row.detail.contains(id.as_str()),
                    "{what}: '{}' states every id in its class: {}",
                    row.subject_id,
                    row.detail
                );
            }
        }
    }
}

#[test]
fn the_table_holds_cases_that_group_and_cases_that_do_not() {
    let grouped = cases()
        .iter()
        .filter(|(_, rows)| !retained_by_key(rows).is_empty())
        .count();
    assert!(
        grouped > 0 && grouped < cases().len(),
        "the table must hold both answers, or the agreement above proves nothing: {grouped} of {} group",
        cases().len()
    );
}
