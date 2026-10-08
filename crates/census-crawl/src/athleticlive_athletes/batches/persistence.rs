use super::super::map::BatchEntities;
use super::PageWalk;
use crate::recording::RowBatch;
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_store::Table;
use serde_json::Value;

pub(super) fn append_entities(
    ctx: &AdapterContext<'_>,
    batch: &mut RowBatch<'_>,
    entities: BatchEntities,
    stamp: &str,
) -> CrawlResult<usize> {
    let phase = "athleticlive_athletes_projection_v2";
    let observations = crate::athlete_observations_of(&entities.athletes, &entities.schools, stamp);
    let written = crate::recording::projection::append_new(
        ctx,
        batch,
        Table::Athletes,
        entities.athletes,
        phase,
    )?;
    crate::recording::projection::append_new(ctx, batch, Table::Schools, entities.schools, phase)?;
    crate::recording::projection::append_new(ctx, batch, Table::Teams, entities.teams, phase)?;
    crate::recording::projection::append_new(
        ctx,
        batch,
        Table::SourceObservations,
        observations,
        phase,
    )?;
    Ok(written)
}

impl PageWalk<'_> {
    pub(super) fn page_receipt(
        &self,
        capture: &crate::net::FetchOutcome,
        next: usize,
        complete: bool,
        rows: usize,
    ) -> Value {
        let locator = self.locator();
        serde_json::json!({"parser":"athleticlive_athletes_projection_v2",
            "locator":locator, "meet":self.target.athleticlive_meet_id, "provider":self.target.tenant,
            "meet_name":self.target.name,"meet_date":self.target.date,"state":self.target.state,"school_year":self.ctx.school_year,
            "capture":{"url":capture.url,"response_url":capture.response_url,"method":capture.method,
                "status":capture.status,"content_digest":capture.content_digest,"bytes":capture.bytes,
                "fetched_at":capture.fetched_at,"content_type":capture.content_type},
            "next":next,"complete":complete,"rows":rows})
    }

    pub(super) fn journal_page(
        &self,
        batch: &mut RowBatch<'_>,
        capture: &crate::net::FetchOutcome,
        body: &str,
        receipt: &Value,
    ) -> CrawlResult<()> {
        let key = census_domain::model::serialized_digest(receipt).map_err(|error| {
            CrawlError::Invariant {
                detail: error.to_string(),
            }
        })?;
        if !self
            .ctx
            .store
            .journal_contains("athleticlive_athletes_capture_v1", &capture.content_digest)?
        {
            batch.journal_done(
                "athleticlive_athletes_capture_v1",
                &capture.content_digest,
                &serde_json::json!({"body":body,"bytes":capture.bytes}),
            )?;
        }
        if !self
            .ctx
            .store
            .journal_contains("athleticlive_athletes_effect_v2", &key)?
        {
            batch.journal_done("athleticlive_athletes_effect_v2", &key, receipt)?;
        }
        Ok(())
    }

    pub(super) fn credit_rows(&mut self, written: usize) -> CrawlResult<()> {
        self.report.rows = self
            .report
            .rows
            .checked_add(u64::try_from(written).map_err(|_| CrawlError::Arithmetic {
                detail: "athlete committed count".into(),
            })?)
            .ok_or_else(|| CrawlError::Arithmetic {
                detail: "athlete report count".into(),
            })?;
        Ok(())
    }
}
