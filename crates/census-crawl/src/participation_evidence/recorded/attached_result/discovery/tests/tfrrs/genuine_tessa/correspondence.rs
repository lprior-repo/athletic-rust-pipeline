use super::*;
use crate::participation_evidence::result_ownership::{
    calendar_correspondence as calendar, school_correspondence as school,
};
use census_domain::model::{Gender, ProfileDate, SchoolYear, Sport};
use census_domain::result_calendar_correspondence::*;
use census_domain::result_school_correspondence::*;
use census_store::{
    ResultCalendarCorrespondencePreparationV1, ResultCalendarEnrollmentReviewV1,
    ResultSchoolCorrespondencePreparationV1, ResultSchoolEnrollmentReviewV1,
};

mod calendar_review;
mod freeze;
mod physical;
mod proposal;
mod replay;
mod school_review;

fn at() -> TestResult<chrono::DateTime<chrono::Utc>> {
    Ok(chrono::DateTime::parse_from_rfc3339("2026-10-21T22:10:00Z")?.to_utc())
}

#[test]
fn genuine_tessa_school_and_calendar_reviews_preserve_conversion_frozen_and_cold() -> TestResult {
    let installed = source::install()?;
    review::attach(&installed, false)?;
    review::attach(&installed, true)?;
    school_review::enroll(&installed.store)?;
    let school_proposal = proposal::school(&installed)?;
    let school = school_review::accept(&installed.store, &school_proposal)?;
    calendar_review::enroll(&installed.store)?;
    let before = state(&installed.store)?;
    let calendar_proposal = proposal::calendar(&installed)?;
    check!(eq; state(&installed.store)?, before);
    let absent = freeze::capture(&installed.store)?;
    freeze::observe(
        &absent,
        &installed.school.id,
        &installed.participation,
        None,
    )?;
    let calendar = calendar_review::accept(&installed.store, &calendar_proposal)?;
    let expected = calendar.0.source.calendar.interpretation.clone();
    let before_freeze = state(&installed.store)?;
    let proof = freeze::capture(&installed.store)?;
    freeze::observe(
        &proof,
        &installed.school.id,
        &installed.participation,
        Some(&expected),
    )?;
    check!(eq; state(&installed.store)?, before_freeze);
    source::originals(&installed.store, &installed.originals)?;
    replay::school(&installed.store, school.0.clone(), &school.1)?;
    replay::calendar(&installed.store, calendar.0.clone(), &calendar.1)?;
    let protected = state(&installed.store)?;
    let source::Installed {
        directory,
        store,
        school: target,
        participation,
        originals,
        ..
    } = installed;
    store.flush()?;
    drop(store);
    freeze::observe(&proof, &target.id, &participation, Some(&expected))?;
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("store"))?.into_parts();
    drop(writer);
    check!(eq; state(&store)?, protected);
    replay::school(&store, school.0, &school.1)?;
    replay::calendar(&store, calendar.0, &calendar.1)?;
    source::originals(&store, &originals)?;
    let reopened = freeze::capture(&store)?;
    freeze::observe(&reopened, &target.id, &participation, Some(&expected))?;
    freeze::observe(&proof, &target.id, &participation, Some(&expected))?;
    check!(eq; state(&store)?, protected);
    Ok(())
}
