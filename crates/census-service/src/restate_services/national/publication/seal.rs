use super::KEY_SEAL;
use crate::restate_services::census::CensusClient;
use crate::restate_services::wire::SealRequest;
use census_reconcile::identity::WorkflowIdentity;
use restate_sdk::prelude::*;
use std::time::Duration;

pub(super) async fn measure(
    ctx: &WorkflowContext<'_>,
    identity: &WorkflowIdentity,
    request: SealRequest,
    delay: Duration,
    pass_budget: usize,
) -> Result<(), HandlerError> {
    for round in 0..=pass_budget {
        let outcome = ctx
            .service_client::<CensusClient>()
            .publication(Json(request.clone()))
            .idempotency_key(format!("{}:seal:{round}", identity.as_str()))
            .call()
            .await;
        match outcome {
            Ok(reply) => {
                ctx.set(KEY_SEAL, reply);
                return Ok(());
            }
            Err(error) if error.message().contains("the census seal refused: ") => {
                ctx.set("publication_refusal", Json(error.message().to_owned()));
                if round == pass_budget {
                    return Err(TerminalError::new_with_code(
                        409,
                        format!(
                            "publication seal retry budget {pass_budget} exhausted: {}",
                            error.message()
                        ),
                    )
                    .into());
                }
                ctx.sleep(delay).await?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    Err(TerminalError::new("publication seal measurement exhausted").into())
}
