mod multi_subject;
mod multi_subject_fixture;
mod multi_subject_frozen;
mod proposal;
pub(in crate::export::dataset::selected::tests::publication) mod revocation;
mod size_bound;

use super::documentary_lifecycle;
use super::documentary_material;
use super::documentary_review::{self, AcceptedSource};
use super::point_source;
use super::school_authority;
use super::source::{self, InstalledSource, TestResult};
use census_crawl::participation_evidence::ParticipationEvidenceRefV1;
use census_domain::evidence_subject_attachment::{
    AttachmentPredecessorV1, AttachmentReplayGrantV1,
};
use census_domain::result_calendar_correspondence::{
    ResultCalendarReviewActorV1, ResultCalendarTextV1,
};
use census_store::{Application, ResultCalendarEnrollmentReviewV1, Store};
use chrono::{DateTime, Utc};

pub(super) fn install_reviewed() -> TestResult<(
    InstalledSource,
    Vec<ParticipationEvidenceRefV1>,
    AcceptedSource,
)> {
    let installed = source::install()?;
    let points = point_source::install(&installed)?;
    let fifty = points.get(1).ok_or("May15 source missing")?;
    school_authority::establish(&installed, fifty)?;
    let accepted = documentary_review::accept(
        &installed,
        fifty,
        documentary_material::Kind::Point,
        AttachmentPredecessorV1::First,
        "controlled-calendar-lifecycle-status",
    )?;
    source::require_originals(&installed.store, &installed.source_records)?;
    Ok((installed, points, accepted))
}

pub(super) fn calendar_text(value: &str) -> TestResult<ResultCalendarTextV1> {
    Ok(ResultCalendarTextV1::parse(value)?)
}

pub(super) fn review_time() -> TestResult<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339("2026-10-17T02:00:00Z")?.with_timezone(&Utc))
}

pub(super) fn protected_state(store: &Store) -> TestResult<(u64, u64, u64)> {
    let snapshot = store.snapshot();
    Ok((
        snapshot.evidence_generation(),
        snapshot.derived_generation(),
        store.receipt_count()?,
    ))
}
fn require_lifecycle(store: &Store, expected: Option<u32>) -> TestResult {
    let format = store.store_format()?;
    check!(eq; format.lifecycle_revision, expected);
    check!(eq; format.schema_version, Some(2));
    check!(eq; format.key_format, Some(1));
    check!(format.migration_target.is_none());
    Ok(())
}

fn prepare_enrollment<'s>(
    store: &'s Store,
    operation: &str,
    rationale: &str,
) -> TestResult<(
    census_store::PreparedResultCalendarEnrollmentV1<'s>,
    census_domain::result_calendar_correspondence::ResultCalendarDigestV1,
    Vec<u8>,
)> {
    let inputs = store.inspect_result_calendar_enrollment_v1(AttachmentReplayGrantV1::new())?;
    let prepared = store.prepare_result_calendar_enrollment_v1(
        inputs,
        ResultCalendarEnrollmentReviewV1 {
            operation: calendar_text(operation)?,
            actor: ResultCalendarReviewActorV1::LocalOperator,
            reviewed_at: review_time()?,
            rationale: calendar_text(rationale)?,
        },
    )?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    Ok((prepared, digest, approved))
}

fn authenticate_predecessor(installed: &InstalledSource, school_bytes: &[u8]) -> TestResult {
    let calendar_proof = installed
        .store
        .read_result_calendar_enrollment_proof_v1(&mut AttachmentReplayGrantV1::new())?;
    let store_identity = installed.store.store_identity()?;
    let capability = census_store::replay_result_calendar_enrollment_proof_v1(
        calendar_proof.bytes(),
        &store_identity,
        &mut AttachmentReplayGrantV1::new(),
    )?;
    let expected = census_store::replay_result_school_enrollment_proof_v1(
        school_bytes,
        &store_identity,
        &mut AttachmentReplayGrantV1::new(),
    )?;
    check!(eq; capability, expected);
    let authenticated = census_store::require_result_calendar_school_predecessor_v1(
        calendar_proof.bytes(),
        school_bytes,
        &store_identity,
        &mut AttachmentReplayGrantV1::new(),
    )?;
    check!(eq; authenticated, expected);
    check!(matches!(authenticated.lifecycle(), 7 | 8));
    Ok(())
}

#[test]
fn calendar_enrollment_advances_nine_to_ten_with_exact_school_predecessor() -> TestResult {
    let (installed, _points, accepted) = install_reviewed()?;
    require_lifecycle(&installed.store, Some(9))?;
    let school_proof = installed
        .store
        .read_result_school_enrollment_proof_v1(&mut AttachmentReplayGrantV1::new())?;
    let before = protected_state(&installed.store)?;
    check!(installed
        .store
        .snapshot()
        .result_calendar_enrollment_proof_v1(&mut AttachmentReplayGrantV1::new())?
        .is_none());
    let (prepared, digest, approved) = prepare_enrollment(
        &installed.store,
        "controlled-calendar-enroll",
        "Explicit controlled calendar enrollment after school9 and reviewed status",
    )?;
    check!(eq; protected_state(&installed.store)?, before);
    require_lifecycle(&installed.store, Some(9))?;
    match installed
        .store
        .apply_result_calendar_enrollment_v1(prepared, &approved, &digest)?
    {
        Application::Written(receipt) => drop(receipt),
        Application::Repeated(_) => {
            return Err("calendar enrollment unexpectedly replayed".into());
        }
    }
    require_lifecycle(&installed.store, Some(10))?;
    check!(protected_state(&installed.store)? != before);
    source::require_originals(&installed.store, &installed.source_records)?;
    authenticate_predecessor(&installed, school_proof.bytes())?;
    documentary_lifecycle::revoke(&installed, &accepted)?;
    documentary_lifecycle::replay_without_reactivation(&installed, &accepted)?;
    source::require_originals(&installed.store, &installed.source_records)?;
    require_cold_enrollment(installed)
}

fn require_cold_enrollment(installed: InstalledSource) -> TestResult {
    let identity = installed.store.store_identity()?;
    let proof = installed
        .store
        .read_result_calendar_enrollment_proof_v1(&mut AttachmentReplayGrantV1::new())?;
    drop(installed.identity_writer);
    drop(installed.store);
    let reopened = super::reopen(&installed.directory)?;
    require_lifecycle(&reopened, Some(10))?;
    check!(eq; reopened.store_identity()?, identity);
    let retained =
        reopened.read_result_calendar_enrollment_proof_v1(&mut AttachmentReplayGrantV1::new())?;
    check!(eq; retained.bytes(), proof.bytes());
    source::require_originals(&reopened, &installed.source_records)?;
    Ok(())
}