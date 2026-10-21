use super::result_ownership::{CheckedRetainedResultDocumentV1, RetainedResultOwnershipError};
use census_domain::model::{EventIdentityError, PerformanceError, SpecificationError};
use census_domain::participation_candidates::MAX_PARTICIPATION_FACTS;

mod admission;
mod observation;
mod project;
mod provenance;
pub mod tfrrs;
mod types;

pub use types::{
    ProposedHtmlResultIdentityV1, ProposedHtmlResultTargetV1, UnadmittedHtmlResultProjectionV1,
    UnadmittedHtmlResultRowV1,
};

pub use observation::matches_physical_observation;
type Result<T> = std::result::Result<T, HtmlResultProjectionError>;
const INTERPRETATION: &str = "milesplit_html_physical_projection_v1";
const SOURCE_KEY: &str = "milesplit_html_row_v1";

#[derive(Debug, thiserror::Error)]
pub enum HtmlResultProjectionError {
    #[error("unadmitted HTML result projection: {0}")]
    Invalid(&'static str),
    #[error("HTML result projection {resource}: needed {needed}, limit {limit}")]
    Budget {
        resource: &'static str,
        needed: usize,
        limit: usize,
    },
    #[error("HTML result projection allocation refused")]
    Allocation,
    #[error("HTML result projection arithmetic overflow")]
    Overflow,
    #[error("HTML result projection meet: {0}")]
    Meet(String),
    #[error(transparent)]
    Source(#[from] Box<RetainedResultOwnershipError>),
    #[error(transparent)]
    Store(#[from] census_store::StoreError),
    #[error(transparent)]
    Event(#[from] EventIdentityError),
    #[error(transparent)]
    Specification(#[from] SpecificationError),
    #[error(transparent)]
    Performance(#[from] PerformanceError),
}

impl From<RetainedResultOwnershipError> for HtmlResultProjectionError {
    fn from(error: RetainedResultOwnershipError) -> Self {
        Self::Source(Box::new(error))
    }
}

pub fn project_retained_html_results<'a, 'capture>(
    document: &'a CheckedRetainedResultDocumentV1<'capture>,
    targets: &[ProposedHtmlResultTargetV1<'_>],
) -> Result<UnadmittedHtmlResultProjectionV1<'a, 'capture>> {
    admission::all(document, targets)?;
    let meet = project::meet(document)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(targets.len())
        .map_err(|_| HtmlResultProjectionError::Allocation)?;
    for (index, target) in targets.iter().enumerate().take(MAX_PARTICIPATION_FACTS) {
        let source = document
            .row(index)
            .ok_or(HtmlResultProjectionError::Invalid("checked row missing"))?;
        rows.push(project::row(document, &meet, &source, target)?);
    }
    Ok(UnadmittedHtmlResultProjectionV1 {
        document,
        meet,
        rows,
    })
}

pub fn project_retained_html_result_identity(
    document: &CheckedRetainedResultDocumentV1<'_>,
    row_index: usize,
    proposed_athlete: &census_domain::model::AthleteId,
) -> Result<ProposedHtmlResultIdentityV1> {
    let row = document
        .row(row_index)
        .ok_or(HtmlResultProjectionError::Invalid(
            "selected original row missing",
        ))?;
    admission::identity(document, &row, proposed_athlete)?;
    project::identity(document, &row, proposed_athlete)
}
pub fn project_retained_html_result_row<'a, 'capture>(
    document: &'a CheckedRetainedResultDocumentV1<'capture>,
    row_index: usize,
    target: &ProposedHtmlResultTargetV1<'_>,
) -> Result<UnadmittedHtmlResultProjectionV1<'a, 'capture>> {
    admission::single(document, row_index, target)?;
    let meet = project::meet(document)?;
    let source = document
        .row(row_index)
        .ok_or(HtmlResultProjectionError::Invalid(
            "selected original row missing",
        ))?;
    let row = project::row(document, &meet, &source, target)?;
    let rows = one(row)?;
    Ok(UnadmittedHtmlResultProjectionV1 {
        document,
        meet,
        rows,
    })
}

fn one<T>(value: T) -> Result<Vec<T>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(1)
        .map_err(|_| HtmlResultProjectionError::Allocation)?;
    output.push(value);
    Ok(output)
}

#[cfg(test)]
mod tests;
