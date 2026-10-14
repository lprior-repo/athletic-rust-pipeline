use super::{access_snapshot_failure, ledger, stage, TeamsSource};
use crate::restate_services::wire::TeamsSourceRequest;
use crate::restate_services::{teams_arms, JobError};

impl TeamsSource {
    pub(super) async fn acquire(
        &self,
        request: &TeamsSourceRequest,
        observed_on: String,
    ) -> Result<
        (
            ledger::Attempt,
            Vec<census_domain::model::SourceAccessCondition>,
        ),
        JobError,
    > {
        let fetcher = match self
            .owner
            .fetcher(
                &request.jurisdiction.authorized_hosts,
                request.jurisdiction.source_parallelism,
            )
            .await
        {
            Ok(fetcher) => fetcher,
            Err(error) => {
                return Ok((
                    ledger::Attempt::Failed(JobError::Terminal {
                        message: format!("{error:?}"),
                    }),
                    Vec::new(),
                ))
            }
        };
        let result = match teams_arms::team_source(
            teams_arms::SourceRuntime {
                store: &self.owner.store,
                fetcher: &fetcher,
                region: self.jobs.region(),
            },
            request.jurisdiction.jurisdiction,
            crate::restate_services::jobs::AdapterScope {
                season: request.jurisdiction.season,
                refresh: request.jurisdiction.refresh,
                at: &observed_on,
                as_of: request.jurisdiction.history.as_of(),
            },
            &request.source,
        )
        .await
        {
            Ok(report) => stage(report, observed_on),
            Err(error) => ledger::Attempt::Failed(error),
        };
        let conditions = match fetcher.access_conditions().await {
            Ok(conditions) => conditions,
            Err(error) => return Err(access_snapshot_failure(result, error)),
        };
        Ok((result, conditions))
    }
}
