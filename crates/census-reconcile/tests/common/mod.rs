#![allow(dead_code)]

use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalSchool, Gender, GradYear, ReviewCase, SchoolId,
    SourceIdentity, SourceNamespace,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub fn season() -> TestResult<census_domain::model::SchoolYear> {
    match census_domain::model::SchoolYear::new(2026) {
        Some(season) => Ok(season),
        None => Err("2026 starts a season".into()),
    }
}

pub fn school_in(store: &Store, state: UsJurisdiction, name: &str) -> TestResult<SchoolId> {
    let (school, id) = CanonicalSchool::new(state, name, normalize_name(name), None);
    store.append(Table::Schools, &school)?;
    Ok(id)
}

pub fn athlete_in(
    store: &Store,
    school: &SchoolId,
    name: &str,
    year: GradYear,
    gender: Gender,
    object: &str,
) -> TestResult<CanonicalAthlete> {
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, object);
    let athlete = CanonicalAthlete::new(school, name, year, gender, source);
    store.append(Table::Athletes, &athlete)?;
    Ok(athlete)
}

pub fn stored_case(store: &Store, subject: &str) -> TestResult<ReviewCase> {
    let case = store
        .scan::<ReviewCase>(Table::ReviewCases)?
        .into_iter()
        .find(|case| case.subject_id == subject)
        .ok_or("the subject keeps a case")?;
    Ok(case)
}
