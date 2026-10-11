#![allow(dead_code)]

use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalSchool, Gender, GradYear, ReviewCase, SchoolId,
    SourceIdentity, SourceNamespace,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub fn season() -> census_domain::model::SchoolYear {
    census_domain::model::SchoolYear::new(2026).expect("2026 starts a season")
}

pub fn school_in(store: &Store, state: UsJurisdiction, name: &str) -> SchoolId {
    let (school, id) = CanonicalSchool::new(state, name, normalize_name(name));
    store
        .append(Table::Schools, &school)
        .expect("school appends");
    id
}

pub fn athlete_in(
    store: &Store,
    school: &SchoolId,
    name: &str,
    year: GradYear,
    gender: Gender,
    object: &str,
) -> CanonicalAthlete {
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, object);
    let athlete = CanonicalAthlete::new(school, name, year, gender, source);
    store
        .append(Table::Athletes, &athlete)
        .expect("athlete appends");
    athlete
}

pub fn stored_case(store: &Store, subject: &str) -> ReviewCase {
    store
        .scan::<ReviewCase>(Table::ReviewCases)
        .expect("cases scan")
        .into_iter()
        .find(|case| case.subject_id == subject)
        .expect("the subject keeps a case")
}
