use super::attachment;
use super::school_issuance as issuance;
use super::source::InstalledSource;
use super::source::TestResult;
use census_crawl::participation_evidence::ParticipationEvidenceRefV1;
use census_store::Receipt;

pub(super) fn establish(
    installed: &InstalledSource,
    target_status: &ParticipationEvidenceRefV1,
) -> TestResult<Vec<Receipt>> {
    super::source::require_originals(&installed.store, &installed.source_records)?;
    if installed.row_locators.len() != 3 {
        return Err("controlled source requires three row locators".into());
    }
    attachment::apply(installed, target_status)?;
    issuance::enroll(
        &installed.store,
        "controlled-school-enroll",
        "Explicit enrollment after source-bound public birth and controlled original row attachment",
    )?;
    let receipts = issuance::issue(installed)?;
    if receipts.len() != 3 {
        return Err("controlled establishment requires three receipts".into());
    }
    Ok(receipts)
}