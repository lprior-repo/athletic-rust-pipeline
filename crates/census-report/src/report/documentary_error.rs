#[derive(Debug, thiserror::Error)]
pub enum DocumentarySourceError {
    #[error(transparent)]
    Crawl(#[from] census_crawl::CrawlError),
    #[error(transparent)]
    Projection(
        #[from] census_crawl::participation_evidence::physical_result::HtmlResultProjectionError,
    ),
    #[error(transparent)]
    Assessment(#[from] census_domain::participation_candidates::ParticipationInputError),
    #[error(transparent)]
    Evidence(#[from] census_domain::affiliation_candidates::AffiliationCandidateError),
    #[error(transparent)]
    Profile(#[from] census_domain::model::ProfileError),
}

impl From<DocumentarySourceError> for super::ReportError {
    fn from(source: DocumentarySourceError) -> Self {
        Self::Documentary {
            source: Box::new(source),
        }
    }
}

pub(crate) fn source(error: impl Into<DocumentarySourceError>) -> super::ReportError {
    error.into().into()
}
