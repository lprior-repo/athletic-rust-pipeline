use restate_sdk::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    future::Future,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct EffectFailure {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub status: Option<u16>,
}

impl EffectFailure {
    pub fn new(code: &str, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
            retryable,
            status: None,
        }
    }
}

pub(super) async fn cached<T: DeserializeOwned + 'static>(
    ctx: &ObjectContext<'_>,
    key: &str,
) -> Result<Option<T>, TerminalError> {
    Ok(ctx.get::<Json<T>>(key).await?.map(|value| value.0))
}

pub(super) async fn journal<T, F, Fut>(
    ctx: &ObjectContext<'_>,
    name: &str,
    effect: F,
) -> Result<Result<T, EffectFailure>, TerminalError>
where
    T: Serialize + DeserializeOwned + Send + 'static,
    F: FnOnce() -> Fut + Send,
    Fut: Future<Output = Result<T, EffectFailure>> + Send,
{
    // Application failures are journaled data, not errors that multiply transport retries.
    let result = ctx
        .run(|| async move {
            let value = effect().await;
            let value = bounded(value);
            Ok(Json(value))
        })
        .name(name)
        .retry_policy(
            RunRetryPolicy::new()
                .initial_delay(Duration::from_secs(1))
                .max_attempts(3),
        )
        .await?;
    Ok(result.0)
}

fn bounded<T: Serialize>(value: Result<T, EffectFailure>) -> Result<T, EffectFailure> {
    let mut counter = SizeLimit { bytes: 0 };
    if let Err(error) = serde_json::to_writer(&mut counter, &value) {
        return Err(EffectFailure::new(
            "CHECKPOINT_SIZE",
            format!("checkpoint serialization exceeded limit or failed: {error}"),
            false,
        ));
    }
    value
}

struct SizeLimit {
    bytes: usize,
}
impl std::io::Write for SizeLimit {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let size = self
            .bytes
            .checked_add(bytes.len())
            .filter(|size| *size <= 8_388_608)
            .ok_or_else(|| std::io::Error::other("8 MiB checkpoint limit"))?;
        self.bytes = size;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) async fn timestamp(ctx: &ObjectContext<'_>) -> Result<u64, TerminalError> {
    ctx.run(|| async {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|time| time.as_secs())
            .map_err(|error| HandlerError::from(TerminalError::new(error.to_string())))
    })
    .name("completion-time")
    .await
}
