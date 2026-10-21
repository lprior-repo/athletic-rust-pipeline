use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Retained {
    pub(super) token: String,
    pub(super) sequence: u64,
}

#[derive(Clone)]
pub(super) struct OldOwner {
    pub(super) effects: Arc<Effects>,
}

#[derive(Clone)]
pub(super) struct NewOwner {
    pub(super) effects: Arc<Effects>,
}

#[derive(Default)]
pub(super) struct Effects {
    pub(super) runs: AtomicUsize,
    pub(super) active: AtomicUsize,
}

struct Active<'a>(&'a Effects);

impl Drop for Active<'_> {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
    }
}

async fn retained(
    ctx: ObjectContext<'_>,
    input: Retained,
    effects: &Arc<Effects>,
) -> Result<Retained, HandlerError> {
    effects.active.fetch_add(1, Ordering::SeqCst);
    let _active = Active(effects);
    let Json(value) = ctx
        .run(|| async move {
            effects.runs.fetch_add(1, Ordering::SeqCst);
            Ok::<_, HandlerError>(Json(input))
        })
        .await?;
    ctx.sleep(Duration::from_secs(15)).await?;
    Ok(value)
}

#[object(
    name = "NativeDeploymentHandoff",
    journal_retention = "1 hour",
    idempotency_retention = "1 hour"
)]
impl OldOwner {
    #[handler]
    async fn run(
        &self,
        ctx: ObjectContext<'_>,
        Json(input): Json<Retained>,
    ) -> Result<Json<Retained>, HandlerError> {
        retained(ctx, input, &self.effects).await.map(Json)
    }
}

#[object(
    name = "NativeDeploymentHandoff",
    journal_retention = "1 hour",
    idempotency_retention = "1 hour"
)]
impl NewOwner {
    #[handler]
    async fn run(
        &self,
        ctx: ObjectContext<'_>,
        Json(input): Json<Retained>,
    ) -> Result<Json<Retained>, HandlerError> {
        retained(ctx, input, &self.effects).await.map(Json)
    }

    #[handler]
    async fn added(&self, _ctx: SharedObjectContext<'_>) -> Result<Json<String>, HandlerError> {
        Ok(Json("added".to_owned()))
    }
}
