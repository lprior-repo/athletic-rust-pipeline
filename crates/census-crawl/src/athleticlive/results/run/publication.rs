use super::{counter_error, receipts, resource, Run, MAX_CAPTURE_RECORDS};
use crate::athleticlive::map::{ResultStats, SOURCE_ID};
use crate::athleticlive::results::{append, WalkResult};
use crate::net::cache::CacheMeta;
use crate::{AdapterContext, CrawlResult, PerformanceDateAssessment};
use census_domain::model::{Evidence, SourceRef};
use serde_json::{json, Value};

impl Run {
    pub(super) fn retain_window(
        &mut self,
        ctx: &AdapterContext<'_>,
        capture: (&str, &str, &CacheMeta),
        before: (usize, usize),
        parsed: Option<Value>,
    ) -> CrawlResult<()> {
        let new_failures = self
            .failures
            .len()
            .checked_sub(before.0)
            .ok_or_else(counter_error)?;
        self.failure_count = self
            .failure_count
            .checked_add(new_failures)
            .ok_or_else(counter_error)?;
        let complete =
            parsed.is_some() && new_failures == 0 && unresolved(&self.stats)? == before.1;
        if !complete {
            self.owe(capture.0)?;
        }
        self.failures.truncate(5);
        self.failures
            .iter_mut()
            .for_each(|failure| *failure = failure.chars().take(4096).collect());
        if !matches!(
            crate::assess_performance_date(self.performance_as_of, &self.target.date),
            PerformanceDateAssessment::Unknown
        ) {
            self.accumulator
                .meets
                .insert(self.meet.id.as_str().to_string(), self.meet.clone());
        }
        let entities =
            std::mem::take(&mut self.accumulator).into_entities(ResultStats::default())?;
        let receipt = receipts::receipt(self, capture, &entities, complete, parsed)?;
        if ctx
            .store
            .journal_contains(crate::athleticlive::results::EFFECT_PHASE, &receipt.key)?
        {
            self.resumed = self.resumed.checked_add(1).ok_or_else(counter_error)?;
        }
        let physical = AdapterContext {
            observed_on: capture.2.fetched_at.clone(),
            school_year: self.school_year,
            ..*ctx
        };
        self.counts.add(append(
            &physical,
            entities,
            &self.schools,
            capture.1,
            &receipt,
        )?)?;
        self.resolved.clear();
        Ok(())
    }

    pub(super) fn reject(&mut self, locator: &str, detail: &str) -> CrawlResult<()> {
        self.failure_count = self
            .failure_count
            .checked_add(1)
            .ok_or_else(counter_error)?;
        self.owe(locator)?;
        if self.failures.len() < 5 {
            self.failures
                .try_reserve(1)
                .map_err(|_| resource("LIVE failure allocation", 1))?;
            self.failures.push(detail.chars().take(4096).collect());
        }
        Ok(())
    }

    pub(super) fn owe(&mut self, locator: &str) -> CrawlResult<()> {
        if self.unfinished.iter().any(|value| value == locator) {
            return Ok(());
        }
        if locator.len() > 4096 {
            return Err(resource("LIVE unfinished locator bytes", locator.len()));
        }
        if self.unfinished.len() >= MAX_CAPTURE_RECORDS {
            return Err(resource("LIVE unfinished locators", self.unfinished.len()));
        }
        self.unfinished
            .try_reserve(1)
            .map_err(|_| resource("LIVE unfinished allocation", 1))?;
        self.unfinished.push(locator.to_string());
        Ok(())
    }

    pub(in crate::athleticlive::results) fn close(self) -> CrawlResult<WalkResult> {
        Ok(WalkResult {
            entities: self.accumulator.into_entities(self.stats)?,
            counts: self.counts,
            failures: self.failures,
            failure_count: self.failure_count,
            unfinished: self.unfinished,
            resumed: self.resumed,
        })
    }
}

pub(super) fn unresolved(stats: &ResultStats) -> CrawlResult<usize> {
    [
        stats.rows_skipped_no_name,
        stats.rows_skipped_no_school,
        stats.rows_skipped_unresolved_school,
        stats.rows_skipped_no_grade,
        stats.rows_skipped_unsupported_cohort,
        stats.events_unmapped,
    ]
    .into_iter()
    .try_fold(0usize, |count, value| {
        count.checked_add(value).ok_or_else(counter_error)
    })
}

pub(super) fn capture_evidence(metadata: &CacheMeta) -> Evidence {
    let mut evidence = Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(metadata.url.clone())),
        &metadata.fetched_at,
    );
    evidence.note = Some(json!({"capture_url":metadata.url,"sha256":metadata.content_digest,"acquired_at":metadata.fetched_at}).to_string());
    evidence
}
