use super::*;
use crate::public_identity_tests::support::{retained_state, subject, TestResult};
use crate::{Application, Store, Table};
use census_domain::result_school_correspondence::{
    ResultSchoolDigestV1, ResultSchoolReviewActorV1, ResultSchoolTextV1,
};

mod lifecycle;
mod predecessor;
mod refusal;

fn fixture() -> TestResult<(tempfile::TempDir, Store, crate::PublicIdentityWriterV1)> {
    let (directory, store, writer) = crate::public_identity_tests::support::fixture()?;
    let athlete = subject("school-enrollment-accepted", "991900")?;
    store.append(Table::Athletes, &athlete)?;
    writer.establish_source_bound(
        &store,
        "controlled-school-predecessor-identity",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    check!(eq; writer.head(&store)?.sequence, 1);
    Ok((directory, store, writer))
}

fn review() -> TestResult<ResultSchoolEnrollmentReviewV1> {
    Ok(ResultSchoolEnrollmentReviewV1 {
        operation: ResultSchoolTextV1::parse("controlled-school-enrollment")?,
        actor: ResultSchoolReviewActorV1::LocalOperator,
        reviewed_at: chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")?.to_utc(),
        rationale: ResultSchoolTextV1::parse(
            "Explicit controlled enrollment, no source-school correspondence issued",
        )?,
    })
}
