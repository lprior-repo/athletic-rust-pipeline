use super::map::build_entities;
use super::{batch_query, MeetTarget, Options, ENDPOINT, PAGE_SIZE, RESULT_WINDOW};
use crate::directory::acquisition::{fail, owe, text};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_store::Table;
use futures::{stream, StreamExt, TryStreamExt};
use serde_json::Value;
use std::collections::HashMap;

struct PageWalk<'a> {
    ctx: &'a AdapterContext<'a>,
    options: &'a Options,
    target: &'a MeetTarget,
    report: AdapterReport,
    from: usize,
    done: bool,
}

pub(super) async fn run_target(
    ctx: &AdapterContext<'_>,
    options: &Options,
    target: &MeetTarget,
    report: AdapterReport,
) -> CrawlResult<AdapterReport> {
    let run = PageWalk {
        ctx,
        options,
        target,
        report,
        from: 0,
        done: false,
    };
    let mut run = stream::iter(0..RESULT_WINDOW)
        .map(Ok::<_, CrawlError>)
        .try_fold(run, |mut run, _| async move {
            if !run.done {
                run.page().await?;
            }
            Ok(run)
        })
        .await?;
    if !run.done {
        let locator = run.locator();
        owe(&mut run.report, locator)?;
    }
    Ok(run.report)
}

impl PageWalk<'_> {
    fn locator(&self) -> String {
        format!(
            "{ENDPOINT}#meet={}&from={}",
            self.target.athleticlive_meet_id, self.from
        )
    }

    async fn page(&mut self) -> CrawlResult<()> {
        let locator = self.locator();
        let fetch = crate::net::FetchOptions {
            refresh: self.ctx.refresh || self.options.refresh,
            headers: vec![("accept".into(), "application/json".into())],
            ..self.ctx.fetch_options()
        };
        let capture = match self
            .ctx
            .fetcher
            .post_json(
                ENDPOINT,
                &batch_query(&[self.target.athleticlive_meet_id], self.from),
                &fetch,
            )
            .await
        {
            Ok(capture) if capture.status == 200 => capture,
            Ok(capture) => {
                self.done = true;
                return fail(
                    &mut self.report,
                    &locator,
                    format!("HTTP {}", capture.status),
                );
            }
            Err(error) => {
                self.done = true;
                return fail(&mut self.report, &locator, error);
            }
        };
        let parsed = text(&capture).and_then(|body| {
            serde_json::from_str::<Value>(body).map_err(|source| CrawlError::Decode {
                url: locator.clone(),
                source,
            })
        });
        let parsed = match parsed {
            Ok(parsed) => parsed,
            Err(error) => {
                self.done = true;
                return fail(&mut self.report, &locator, error);
            }
        };
        let Some(rows) = parsed.pointer("/hits/hits").and_then(Value::as_array) else {
            self.done = true;
            return fail(&mut self.report, &locator, "missing hits array");
        };
        if rows.len() > PAGE_SIZE {
            self.done = true;
            return fail(
                &mut self.report,
                &locator,
                "published page exceeds requested size",
            );
        }
        self.retain_page(&parsed, rows, &capture)
    }

    fn advance(&mut self, parsed: &Value, fetched: usize) -> CrawlResult<()> {
        let total = parsed
            .pointer("/hits/total/value")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok());
        let exact = parsed
            .pointer("/hits/total/relation")
            .and_then(Value::as_str)
            == Some("eq");
        self.from = self
            .from
            .checked_add(fetched)
            .ok_or_else(|| CrawlError::Arithmetic {
                detail: "athlete page offset".into(),
            })?;
        if exact && total == Some(self.from) {
            self.done = true;
            return Ok(());
        }
        if !exact
            || total.is_none_or(|total| total < self.from)
            || fetched == 0
            || self.from >= RESULT_WINDOW
        {
            self.done = true;
            let locator = self.locator();
            return owe(&mut self.report, locator);
        }
        Ok(())
    }

    fn emit(
        &mut self,
        batch: &mut crate::recording::RowBatch<'_>,
        row: &Value,
        ordinal: usize,
        stamp: &str,
    ) -> CrawlResult<usize> {
        let locator = format!("{}#row={ordinal}", self.locator());
        let hit = match row
            .get("_source")
            .cloned()
            .map(serde_json::from_value::<super::AthleteHit>)
        {
            Some(Ok(hit)) => hit,
            Some(Err(error)) => {
                fail(&mut self.report, &locator, error)?;
                return Ok(0);
            }
            None => {
                fail(&mut self.report, &locator, "missing athlete source")?;
                return Ok(0);
            }
        };
        let by_id = HashMap::from([(self.target.athleticlive_meet_id, self.target)]);
        let entities = build_entities(
            std::slice::from_ref(&hit),
            &by_id,
            stamp,
            self.ctx.school_year,
        );
        let phase = "athleticlive_athletes_projection_v2";
        if entities.athletes.is_empty() {
            owe(&mut self.report, &locator)?;
        }
        let observations =
            crate::athlete_observations_of(&entities.athletes, &entities.schools, stamp);
        let written = crate::recording::projection::append_new(
            self.ctx,
            batch,
            Table::Athletes,
            entities.athletes,
            phase,
        )?;
        crate::recording::projection::append_new(
            self.ctx,
            batch,
            Table::Schools,
            entities.schools,
            phase,
        )?;
        crate::recording::projection::append_new(
            self.ctx,
            batch,
            Table::Teams,
            entities.teams,
            phase,
        )?;
        crate::recording::projection::append_new(
            self.ctx,
            batch,
            Table::SourceObservations,
            observations,
            phase,
        )?;
        Ok(written)
    }

    fn retain_page(
        &mut self,
        parsed: &Value,
        rows: &[Value],
        capture: &crate::net::FetchOutcome,
    ) -> CrawlResult<()> {
        let locator = self.locator();
        let before = (self.report.errors, self.report.unfinished.len());
        let ctx = self.ctx;
        let mut batch = ctx.write_batch();
        let written = rows
            .iter()
            .enumerate()
            .try_fold(0usize, |count, (ordinal, row)| {
                count
                    .checked_add(self.emit(&mut batch, row, ordinal, &capture.fetched_at)?)
                    .ok_or_else(|| CrawlError::Arithmetic {
                        detail: "athlete committed rows".into(),
                    })
            })?;
        let next = self
            .from
            .checked_add(rows.len())
            .ok_or_else(|| CrawlError::Arithmetic {
                detail: "athlete cursor".into(),
            })?;
        let exact = parsed
            .pointer("/hits/total/relation")
            .and_then(Value::as_str)
            == Some("eq");
        let total = parsed.pointer("/hits/total/value").and_then(Value::as_u64);
        let consistent = total
            .zip(u64::try_from(next).ok())
            .is_some_and(|(total, next)| total >= next);
        let complete = exact
            && consistent
            && before == (self.report.errors, self.report.unfinished.len())
            && (!rows.is_empty() || total == u64::try_from(next).ok());
        let body = text(capture)?;
        let receipt = serde_json::json!({"parser":"athleticlive_athletes_projection_v2",
            "locator":locator, "meet":self.target.athleticlive_meet_id, "provider":self.target.tenant,
            "meet_name":self.target.name,"meet_date":self.target.date,"state":self.target.state,"school_year":self.ctx.school_year,
            "capture":{"url":capture.url,"response_url":capture.response_url,"method":capture.method,
                "status":capture.status,"content_digest":capture.content_digest,"bytes":capture.bytes,
                "fetched_at":capture.fetched_at,"content_type":capture.content_type},
            "next":next,"complete":complete,"rows":rows.len()});
        let key = census_domain::model::serialized_digest(&receipt).map_err(|error| {
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
            batch.journal_done("athleticlive_athletes_effect_v2", &key, &receipt)?;
        }
        batch.commit()?;
        self.report.rows = self
            .report
            .rows
            .checked_add(u64::try_from(written).map_err(|_| CrawlError::Arithmetic {
                detail: "athlete committed count".into(),
            })?)
            .ok_or_else(|| CrawlError::Arithmetic {
                detail: "athlete report count".into(),
            })?;
        self.advance(parsed, rows.len())
    }
}
