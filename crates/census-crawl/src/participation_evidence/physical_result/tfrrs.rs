use super::{
    HtmlResultProjectionError as Error, ProposedHtmlResultIdentityV1,
    ProposedHtmlResultTargetV1, Result, UnadmittedHtmlResultRowV1,
};
use crate::participation_evidence::result_ownership::tfrrs::{
    CheckedRetainedTfrrsListV1, CheckedRetainedTfrrsRowV1,
};
use census_domain::model::{AthleteId, CanonicalMeet};

mod admission;
mod project;
mod provenance;

const INTERPRETATION: &str = "tfrrs_html_physical_projection_v1";
const SOURCE_KEY: &str = "tfrrs_html_row_v1";

pub fn project_retained_tfrrs_result_row(
    list: &CheckedRetainedTfrrsListV1<'_>,
    row_index: usize,
    target: &ProposedHtmlResultTargetV1<'_>,
) -> Result<(CanonicalMeet, UnadmittedHtmlResultRowV1)> {
    let source = selected(list, row_index)?;
    admission::single(list, &source, target)?;
    let meet = project::meet(list, &source)?;
    let row = project::row(list, &meet, &source, target)?;
    Ok((meet, row))
}

pub fn project_retained_tfrrs_result_identity(
    list: &CheckedRetainedTfrrsListV1<'_>,
    row_index: usize,
    proposed_athlete: &AthleteId,
) -> Result<ProposedHtmlResultIdentityV1> {
    let source = selected(list, row_index)?;
    admission::identity(list, &source, proposed_athlete)?;
    project::identity(list, &source, proposed_athlete)
}

fn selected<'a>(
    list: &'a CheckedRetainedTfrrsListV1<'_>,
    row_index: usize,
) -> Result<CheckedRetainedTfrrsRowV1<'a>> {
    list.row(row_index)
        .ok_or(Error::Invalid("selected original TFRRS row missing"))
}

#[cfg(test)]
mod tests;
