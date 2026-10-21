use super::TeamsSource;
use crate::restate_services::wire::TeamsSourceRequest;
use crate::restate_services::{blocking, jobs::collect_error, JobError};
use census_crawl::net::{FetchOptions, Fetcher, PublicResourceAcquisition};
use census_crawl::AdapterReport;
use census_store::CaptureWriterV1;
use std::sync::Arc;

impl TeamsSource {
    pub(super) async fn acquire_public_team_source(
        &self,
        request: &TeamsSourceRequest,
        fetcher: &Fetcher,
        writer: &Arc<CaptureWriterV1>,
    ) -> Result<Option<AdapterReport>, JobError> {
        if request.source != "milesplit" {
            return Err(JobError::Terminal {
                message: "public directory continuation requires the milesplit source".to_string(),
            });
        }
        let site = request.jurisdiction.jurisdiction;
        let store = Arc::clone(&self.owner.store);
        let writer_for_owner = Arc::clone(writer);
        let owner = blocking(Arc::clone(self.jobs.region()), move || {
            writer_for_owner.public_resource_owner(&store, site)
        })
        .await?;
        let acquisition = PublicResourceAcquisition::new(&self.owner.store, writer, &owner);
        let options = FetchOptions {
            refresh: request.jurisdiction.refresh,
            ..FetchOptions::default()
        };
        let read = fetcher
            .get_public_resource_directory(&acquisition, &options)
            .await
            .map_err(census_crawl::CrawlError::from)
            .and_then(|captured| {
                census_crawl::milesplit::TeamIndexRead::from_capture(captured.directory().capture())
            });
        crate::census::accept_state_team_index(fetcher, &self.owner.store, site, read)
            .await
            .map_err(JobError::from)?;
        crate::census::team_index_report(&self.owner.store, site)
            .map(Some)
            .map_err(collect_error)
    }
}
