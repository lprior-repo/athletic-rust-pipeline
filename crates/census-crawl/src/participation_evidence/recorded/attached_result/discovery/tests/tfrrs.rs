use super::*;
use crate::participation_evidence::physical_result::{
    project_checked_html_result_row, ProposedHtmlResultTargetV1,
};
use crate::participation_evidence::recorded::{
    read_recorded_tfrrs_list_capture, record_retained_tfrrs_list_capture,
};
use crate::participation_evidence::result_ownership::{
    common_fixture, tfrrs::read_retained_tfrrs_list_rows, CheckedHtmlResultDocumentV1,
    CheckedHtmlResultSourceV1,
};
use crate::Recording;
use census_domain::evidence_subject_attachment::*;
use census_domain::model::{
    serialized_bytes, CanonicalAthlete, Evidence, ExactSeconds, GradYear, Mark, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::public_identity_history::PublicAthleteId;
use census_store::{
    Application, AttachmentContextObjectV1, AttachmentInspectionRequestV1,
    EvidenceSubjectAttachmentInspectionV1, EvidenceSubjectAttachmentPreparationV1,
    FrozenEvidenceSubjectAttachmentBundleV1, JournalReadBudget, RetainedCaptureLeafKindV1,
    RetainedCaptureLeafV1, RetainedCaptureRootSchemaV1, RetainedCaptureSlotV1, Store, Table,
};
use serde::Serialize;

mod attachment;
mod genuine_tessa;
mod objects;
mod observe;
mod refusal;
mod setup;

fn text(value: &str) -> TestResult<AttachmentTextV1> {
    Ok(AttachmentTextV1::parse(value)?)
}

fn list<T>(values: Vec<T>) -> TestResult<AttachmentListV1<T>> {
    Ok(AttachmentListV1::new(values)?)
}

pub(in crate::participation_evidence) fn install_controlled_tfrrs_result_attachments(
) -> TestResult<(tempfile::TempDir, Store, String)> {
    let (directory, store, input) = setup::install()?;
    attachment::apply(&store, &input, attachment::Case::Original, 2)?;
    Ok((directory, store, input.context))
}

#[test]
fn genuine_tfrrs_controlled_transport_accepted_rows_match_current_frozen_and_cold() -> TestResult {
    let (directory, store, input) = setup::install()?;
    attachment::apply(&store, &input, attachment::Case::Original, 2)?;
    let before = setup::state(&store, &input.context)?;
    let original = observe::current(&store)?;
    let bundle = observe::freeze(&store)?;
    check!(eq; observe::frozen(&store, &bundle)?, original);
    check!(eq; setup::state(&store, &input.context)?, before);
    refusal::foreign_grant(&store)?;
    check!(eq; setup::state(&store, &input.context)?, before);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("store"))?.into_parts();
    drop(writer);
    check!(eq; setup::state(&store, &input.context)?, before);
    check!(eq; observe::current(&store)?, original);
    check!(eq; observe::frozen(&store, &bundle)?, original);
    check!(eq; setup::state(&store, &input.context)?, before);
    Ok(())
}
