use crate::census::{self, CollectOptions, OwnedRosterSource, SourceAccessFailure, StateProgress};
use census_crawl::milesplit::TeamIndexRead;
use census_crawl::net::{
    CapturedPublicDirectory, FetchError, FetchOptions, Fetcher, PublicResourceAcquisition,
};
use census_crawl::CrawlError;
use census_domain::UsJurisdiction;
use census_store::{CaptureWriterV1, Store};

pub(super) async fn collect(
    store: &Store,
    fetcher: &Fetcher,
    writer: &CaptureWriterV1,
    options: &CollectOptions,
    jurisdiction: UsJurisdiction,
) -> Result<StateProgress, SourceAccessFailure> {
    let owner = writer
        .public_resource_owner(store, jurisdiction)
        .map_err(CrawlError::from)?;
    let run = census_domain::model::CensusRun::new(options.school_year, options.revision.get())
        .ok_or_else(|| CrawlError::Invariant {
            detail: "owned roster run revision is invalid".into(),
        })?;
    if owner.scope().run() != run || owner.scope().jurisdiction() != jurisdiction {
        return Err(CrawlError::Invariant {
            detail: "owned roster options differ from the admitted run".into(),
        }
        .into());
    }
    let acquisition = PublicResourceAcquisition::new(store, writer, &owner);
    let directory = fetcher
        .get_public_resource_directory(
            &acquisition,
            &FetchOptions {
                refresh: options.refresh,
                ..FetchOptions::default()
            },
        )
        .await;
    let directory = directory_result(directory, fetcher, store, jurisdiction).await?;
    let read = TeamIndexRead::from_capture(directory.directory().capture());
    let teams = census::accept_state_team_index(fetcher, store, jurisdiction, read).await?;
    let source = OwnedRosterSource::new(store, writer, &owner, &directory);
    census::collect_owned_state_rosters(fetcher, store, &teams, options, jurisdiction, &source)
        .await
        .map_err(SourceAccessFailure::from)
}

async fn directory_result(
    result: Result<CapturedPublicDirectory, FetchError>,
    fetcher: &Fetcher,
    store: &Store,
    jurisdiction: UsJurisdiction,
) -> Result<CapturedPublicDirectory, SourceAccessFailure> {
    match result {
        Ok(directory) => Ok(directory),
        Err(error) => {
            Err(census::retain_team_index_failure(fetcher, store, jurisdiction, error.into()).await)
        }
    }
}

#[cfg(test)]
mod tests;
