use super::super::super::super::super::flow;
use super::super::super::super::super::source::{InstalledSource, TestResult};
use super::require_feb_exact;
use crate::export::DatasetLineage;
use crate::report::Scope;
use crate::workbook::publication;
use census_crawl::participation_evidence::result_ownership::calendar_correspondence::ResultCalendarSourceProposalV1;
use census_domain::model::{GradYear, QueryProfileV1, SchoolId};
use census_domain::public_identity_history::PublicAthleteId;
use census_domain::result_calendar_correspondence::{
    ResultCalendarActionV1, ResultCalendarDigestV1, ResultCalendarPredecessorV1,
    ResultCalendarRecordReferenceV1, ResultCalendarReviewCommandV1,
};
use census_store::Receipt;
use std::path::{Path, PathBuf};
pub(in super::super) fn bridge_all(
    installed: &InstalledSource,
    proposal49: &ResultCalendarSourceProposalV1,
    proposal50: &ResultCalendarSourceProposalV1,
    proposal51: &ResultCalendarSourceProposalV1,
) -> TestResult<(ResultCalendarReviewCommandV1, Receipt)> {
    super::super::super::enroll_calendar(
        &installed.store,
        "controlled-calendar-enroll",
        "Explicit controlled calendar enrollment after school9 and reviewed status",
    )?;
    super::super::super::apply_calendar_action(
        &installed.store,
        proposal49,
        "controlled-calendar-venue-accept-49",
        ResultCalendarActionV1::Accept {
            prior: ResultCalendarPredecessorV1::First,
        },
        "Explicit controlled calendar review",
    )?;
    let accepted = super::super::super::apply_calendar_action(
        &installed.store,
        proposal50,
        "controlled-calendar-venue-accept-50",
        ResultCalendarActionV1::Accept {
            prior: ResultCalendarPredecessorV1::First,
        },
        "Explicit controlled calendar review",
    )?;
    super::super::super::apply_calendar_action(
        &installed.store,
        proposal51,
        "controlled-calendar-venue-accept-51",
        ResultCalendarActionV1::Accept {
            prior: ResultCalendarPredecessorV1::First,
        },
        "Explicit controlled calendar review",
    )?;
    Ok(accepted)
}
pub(in super::super) fn publish_selected(
    installed: &InstalledSource,
    profile: &QueryProfileV1,
    current: &SchoolId,
    public: &PublicAthleteId,
    historical: &SchoolId,
    second: bool,
    output_root: &Path,
) -> TestResult<(DatasetLineage, String, PathBuf)> {
    let objects = tempfile::tempdir()?;
    let (dataset, workbook) =
        flow::publish(&installed.store, profile, objects.path(), output_root)?;
    flow::assert_output(
        &dataset,
        &workbook,
        public,
        current,
        historical,
        generation(second),
    )?;
    require_feb_exact(&dataset)?;
    super::super::super::super::super::proof_refusal::assert_required_proof_refuses(&dataset)?;
    let lineage = dataset.lineage.clone();
    let digest = publication::verify_published(&workbook)?
        .generation_digest
        .as_str()
        .to_owned();
    drop(dataset);
    objects.close()?;
    Ok((lineage, digest, workbook))
}
pub(in super::super) fn revoke_only_50(
    installed: &InstalledSource,
    proposal50: &ResultCalendarSourceProposalV1,
    accept50: ResultCalendarReviewCommandV1,
    receipt50: Receipt,
) -> TestResult {
    let digest = ResultCalendarDigestV1::parse(receipt50.digest.as_str())?;
    super::super::super::apply_calendar_action(
        &installed.store,
        proposal50,
        "controlled-calendar-venue-revoke-50",
        ResultCalendarActionV1::Revoke {
            prior: ResultCalendarRecordReferenceV1 {
                operation: accept50.operation.clone(),
                digest,
            },
        },
        "Explicit controlled calendar review",
    )?;
    super::super::super::replay_original_without_reactivation(
        &installed.store,
        accept50,
        &receipt50,
    )?;
    Ok(())
}
pub(in super::super) fn reopen(
    workbook: &Path,
    lineage: &DatasetLineage,
    public: &PublicAthleteId,
    current: &SchoolId,
    historical: &SchoolId,
    expected: &str,
    second: bool,
) -> TestResult {
    let opened =
        publication::open_publication(workbook, lineage, Scope::AllSources, GradYear::CO2027)?;
    flow::assert_output(
        &opened.dataset,
        workbook,
        public,
        current,
        historical,
        generation(second),
    )?;
    super::require_feb_exact(&opened.dataset)?;
    check!(
        eq;
        publication::verify_published(workbook)?.generation_digest.as_str(),
        expected
    );
    Ok(())
}

fn generation(second: bool) -> flow::Generation {
    if second {
        flow::Generation::CalendarWithdrawn
    } else {
        flow::Generation::Initial
    }
}