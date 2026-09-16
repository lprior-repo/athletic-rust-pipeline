use crate::exhaustive_engine::Engine;
use anyhow::{Context, Result};
use futures::{stream, TryStreamExt};

use super::RunState;

pub(super) async fn process_rows(engine: &mut Engine, state: &mut RunState) -> Result<()> {
    let pending = std::mem::take(&mut state.pending_indices);
    let interrupt = tokio::signal::ctrl_c();
    tokio::pin!(interrupt);
    stream::iter(pending.into_iter().map(Ok::<_, anyhow::Error>))
        .try_fold(
            (engine, state, interrupt.as_mut()),
            |(engine, state, mut interrupt), index| async move {
                let prospect = state
                    .prospects
                    .get(index)
                    .context("row index outside selected population")?;
                let record = tokio::select! {
                    biased;
                    signal = &mut interrupt => {
                        signal.context("installing Ctrl-C handler")?;
                        anyhow::bail!("Cancelled: current row not committed; prior rows retained");
                    }
                    result = engine.process(prospect) => result?,
                };
                // Once accepted, a record's durable commit is drained before observing cancellation.
                state.commit(record).await?;
                Ok((engine, state, interrupt))
            },
        )
        .await
        .map(|_| ())
}
