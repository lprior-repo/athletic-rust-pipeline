use super::super::{
    row_protocol::{RowReport, ROW_PROTOCOL_REVISION},
    run_protocol::*,
    Runtime,
};
use super::{terminal, RunIdentity};
use crate::domain::identity::EvidenceDigest;
use crate::runtime::clock;
use crate::runtime::rankings_collection::RankingCollectionRef;
use restate_sdk::prelude::*;
use std::sync::Arc;

pub(super) struct Results<'a, 'ctx> {
    ctx: &'a ObjectContext<'ctx>,
    runtime: Arc<Runtime>,
    run_key: String,
    progress_key: String,
    progress: RunProgress,
    page_digests: Vec<EvidenceDigest>,
}

impl<'a, 'ctx> Results<'a, 'ctx> {
    pub async fn new(
        ctx: &'a ObjectContext<'ctx>,
        runtime: Arc<Runtime>,
        identity: RunIdentity,
        selected: u64,
        collection_ref: Option<crate::runtime::rankings_collection::RankingCollectionRef>,
    ) -> Result<Self, HandlerError> {
        let now = journal_time(ctx, &runtime).await?;
        let progress = RunProgress {
            request: identity.request,
            coverage: Coverage {
                selected,
                ..Coverage::default()
            },
            pages: 0,
            pending_rows: Vec::with_capacity(RESULT_PAGE_ROWS),
            complete: false,
            started_at_unix_ms: now,
            updated_at_unix_ms: now,
            summary: None,
            collection_ref,
        };
        let progress_key = format!("progress:{}", identity.key);
        ctx.set(
            &progress_key,
            restate_sdk::serde::Serialize::serialize(&Json(&progress)).map_err(terminal)?,
        );
        Ok(Self {
            ctx,
            runtime,
            run_key: identity.key,
            progress_key,
            progress,
            page_digests: Vec::new(),
        })
    }

    pub fn update_collection_ref(
        &mut self,
        collection_ref: RankingCollectionRef,
    ) -> Result<(), HandlerError> {
        self.progress.collection_ref = Some(collection_ref);
        self.ctx.set(
            &self.progress_key,
            restate_sdk::serde::Serialize::serialize(&Json(&self.progress)).map_err(terminal)?,
        );
        Ok(())
    }
    pub async fn record(
        &mut self,
        job_key: &str,
        digest: EvidenceDigest,
    ) -> Result<(), HandlerError> {
        let report: RowReport = self.runtime.load_json(&digest).await.map_err(terminal)?;
        if report.revision != ROW_PROTOCOL_REVISION
            || report.job.key().map_err(terminal)? != job_key
        {
            return Err(terminal(
                "row result does not bind the requested immutable source row",
            ));
        }
        self.progress
            .coverage
            .record(&report.resolution)
            .map_err(terminal)?;
        self.progress.pending_rows.push(RowReference {
            source: report.job.source,
            report: digest,
        });
        if self.progress.pending_rows.len() == RESULT_PAGE_ROWS {
            self.flush().await?;
        }
        self.progress.updated_at_unix_ms = journal_time(self.ctx, &self.runtime).await?;
        self.ctx.set(
            &self.progress_key,
            restate_sdk::serde::Serialize::serialize(&Json(&self.progress)).map_err(terminal)?,
        );
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), HandlerError> {
        if self.progress.pending_rows.is_empty() {
            return Ok(());
        }
        let rows = std::mem::take(&mut self.progress.pending_rows);
        let digest = self
            .publish("publish result page", RunPage { rows })
            .await?;
        self.ctx.set(
            &format!("page:{}:{}", self.run_key, self.progress.pages),
            Json(digest.clone()),
        );
        self.page_digests.push(digest);
        self.progress.pages = self
            .progress
            .pages
            .checked_add(1)
            .ok_or_else(|| terminal("result page count overflow"))?;
        Ok(())
    }

    pub async fn finish(mut self) -> Result<EvidenceDigest, HandlerError> {
        if self.progress.coverage.completed != self.progress.coverage.selected {
            return Err(terminal(
                "run ended before every selected row reached a terminal result",
            ));
        }
        self.flush().await?;
        self.progress.updated_at_unix_ms = journal_time(self.ctx, &self.runtime).await?;
        let summary = RunSummary {
            coverage: self.progress.coverage.clone(),
            started_at_unix_ms: self.progress.started_at_unix_ms,
            completed_at_unix_ms: self.progress.updated_at_unix_ms,
            page_digests: std::mem::take(&mut self.page_digests),
            collection_ref: self.progress.collection_ref.clone(),
        };
        let digest = self.publish("publish run summary", summary).await?;
        self.progress.complete = true;
        self.progress.summary = Some(digest.clone());
        self.ctx.set(&self.progress_key, Json(self.progress));
        self.ctx
            .set(&format!("result:{}", self.run_key), Json(digest.clone()));
        Ok(digest)
    }

    async fn publish<T: serde::Serialize + Send + 'static>(
        &self,
        name: &'static str,
        value: T,
    ) -> Result<EvidenceDigest, HandlerError> {
        let runtime = self.runtime.clone();
        let digest = self
            .ctx
            .run(|| async move { runtime.store_json(value).await.map(Json).map_err(terminal) })
            .name(name)
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(digest.0)
    }
}

/// Journaled wall-clock read for run progress timestamps.
///
/// The name is the journal identity of this entry and MUST NOT change while an invocation that
/// wrote it can still be replayed.
async fn journal_time(ctx: &ObjectContext<'_>, runtime: &Runtime) -> Result<u64, HandlerError> {
    clock::journal_unix_ms(ctx, &runtime.clock(), "observe run time").await
}
