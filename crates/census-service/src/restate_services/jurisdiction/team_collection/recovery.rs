use restate_sdk::prelude::*;

use super::super::{team_source, TeamsSourceClient};
use crate::restate_services::wire::{
    TeamsAttemptProgress, TeamsSourceInspection, TeamsSourceOutcome, TeamsSourceRequest,
};

pub(super) async fn invoke(
    ctx: &ObjectContext<'_>,
    input: TeamsSourceRequest,
) -> TeamsSourceOutcome {
    let client = ctx.object_client::<TeamsSourceClient>(team_source::key(&input));
    let error = match client.run(Json(input)).call().await {
        Ok(Json(outcome)) => return outcome,
        Err(error) => error,
    };
    let message = format!(
        "native source invocation failed with {}: {}",
        error.code(),
        error.message()
    );
    match client.inspection().call().await {
        Ok(Json(TeamsSourceInspection::Settled { outcome })) => outcome,
        Ok(Json(TeamsSourceInspection::Unsettled { progress })) => {
            let attempts = progress.last().map(|step| match step {
                TeamsAttemptProgress::Unknown { attempt }
                | TeamsAttemptProgress::Completed { attempt, .. }
                | TeamsAttemptProgress::Transient { attempt, .. }
                | TeamsAttemptProgress::Terminal { attempt, .. } => *attempt,
            });
            TeamsSourceOutcome::Interrupted {
                attempts,
                message,
                progress,
            }
        }
        Err(error) => TeamsSourceOutcome::Interrupted {
            attempts: None,
            progress: Vec::new(),
            message: format!(
                "{message}; coherent source inspection unavailable with {}: {}",
                error.code(),
                error.message()
            ),
        },
    }
}
