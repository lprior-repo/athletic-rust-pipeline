use super::{access_snapshot, ledger, stage, TeamsSource};
use crate::restate_services::teams_arms;
use crate::restate_services::wire::TeamsSourceRequest;

impl TeamsSource {
    pub(super) async fn acquire(
        &self,
        request: &TeamsSourceRequest,
        observed_on: String,
        fetcher: &std::sync::Arc<census_crawl::net::Fetcher>,
    ) -> (
        ledger::Attempt,
        Vec<census_domain::model::SourceAccessCondition>,
    ) {
        let result = match teams_arms::team_source(
            teams_arms::SourceRuntime {
                store: &self.owner.store,
                fetcher,
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
            Err(error) => {
                let retained = access_snapshot::failure(result, error);
                return (retained, Vec::new());
            }
        };
        (result, conditions)
    }
}
