use crate::bests::SelectionOutcome;
use crate::report::{Derivation, ReportResult};
use crate::workbook::publication::{current_affiliations, participation_exclusions, selected};
use std::path::Path;

pub(super) fn write(
    directory: &Path,
    derivation: &Derivation<'_>,
    outcome: &SelectionOutcome,
    revision: u32,
    counts: &mut crate::workbook::publication::inventory::RowCounts,
) -> ReportResult<()> {
    if !matches!(
        revision,
        selected::REVISION
            | selected::REPORTED_REVISION
            | selected::PROFILE_PRS_REVISION
            | selected::OWNED_PROFILE_PRS_REVISION
    ) {
        return Ok(());
    }
    let dataset = derivation.dataset();
    counts.insert(
        "current_affiliations.csv".into(),
        current_affiliations::write(directory, dataset)?,
    );
    counts.insert(
        "participation_exclusions.csv".into(),
        participation_exclusions::write(
            directory,
            dataset,
            dataset.verified_participation()?,
            outcome,
        )?,
    );
    if matches!(
        revision,
        selected::REPORTED_REVISION
            | selected::PROFILE_PRS_REVISION
            | selected::OWNED_PROFILE_PRS_REVISION
    ) {
        counts.insert(
            "reported_performances.csv".into(),
            super::write_reported(directory, derivation, outcome)?,
        );
    }
    if matches!(
        revision,
        selected::PROFILE_PRS_REVISION | selected::OWNED_PROFILE_PRS_REVISION
    ) {
        counts.insert(
            "source_profile_prs.csv".into(),
            super::source_profile_prs::write(directory, derivation, outcome)?,
        );
    }
    Ok(())
}
