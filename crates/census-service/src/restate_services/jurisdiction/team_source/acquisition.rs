use super::execution_ledger::ExecutionLedger;
use super::{access_snapshot, ledger, stage, TeamsSource};
use crate::restate_services::teams_arms;
use crate::restate_services::wire::TeamsSourceRequest;

mod public_resources;

impl TeamsSource {
    pub(super) async fn acquire(
        &self,
        request: &TeamsSourceRequest,
        observed_on: String,
        fetcher: &std::sync::Arc<census_crawl::net::Fetcher>,
        scope: &ExecutionLedger,
    ) -> (
        ledger::Attempt,
        Vec<census_domain::model::SourceAccessCondition>,
    ) {
        let acquired = match (&self.owner.capture, scope) {
            (
                crate::restate_services::CaptureExecution::FreshQualification(writer),
                ExecutionLedger::OriginContinuation(_),
            ) => {
                self.acquire_public_team_source(request, fetcher, writer)
                    .await
            }
            (_, ExecutionLedger::OriginContinuation(_)) => {
                Err(crate::restate_services::JobError::Terminal {
                    message: "public directory continuation requires current capture qualification"
                        .to_string(),
                })
            }
            (_, ExecutionLedger::Original) => {
                teams_arms::team_source(
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
            }
        };
        let result = match acquired {
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
