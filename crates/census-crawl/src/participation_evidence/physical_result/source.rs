use super::tfrrs::{project_retained_tfrrs_result_identity, project_retained_tfrrs_result_row};
use super::{
    project_retained_html_result_identity, project_retained_html_result_row,
    HtmlResultProjectionError as Error, ProposedHtmlResultIdentityV1, ProposedHtmlResultTargetV1,
    Result, UnadmittedHtmlResultRowV1,
};
use crate::participation_evidence::result_ownership::CheckedHtmlResultDocumentV1;
use census_domain::model::{AthleteId, CanonicalMeet};

pub fn project_checked_html_result_identity(
    document: &CheckedHtmlResultDocumentV1<'_>,
    index: usize,
    athlete: &AthleteId,
) -> Result<ProposedHtmlResultIdentityV1> {
    match document {
        CheckedHtmlResultDocumentV1::MileSplit(source) => {
            project_retained_html_result_identity(source, index, athlete)
        }
        CheckedHtmlResultDocumentV1::Tfrrs(source) => {
            project_retained_tfrrs_result_identity(source, index, athlete)
        }
    }
}

pub fn project_checked_html_result_row(
    document: &CheckedHtmlResultDocumentV1<'_>,
    index: usize,
    target: &ProposedHtmlResultTargetV1<'_>,
) -> Result<(CanonicalMeet, UnadmittedHtmlResultRowV1)> {
    match document {
        CheckedHtmlResultDocumentV1::MileSplit(source) => {
            let (meet, rows) =
                project_retained_html_result_row(source, index, target)?.into_proposed_entities();
            if rows.len() != 1 {
                return Err(Error::Invalid(
                    "single row projection returned unexpected row count",
                ));
            }
            let row = rows
                .into_iter()
                .next()
                .ok_or(Error::Invalid("single row projection vanished"))?;
            Ok((meet, row))
        }
        CheckedHtmlResultDocumentV1::Tfrrs(source) => {
            project_retained_tfrrs_result_row(source, index, target)
        }
    }
}

#[cfg(test)]
mod tests;
