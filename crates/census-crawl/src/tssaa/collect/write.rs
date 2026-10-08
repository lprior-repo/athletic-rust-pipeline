use super::super::map::Emission;
use super::{counter_error, Run, JOURNAL};
use crate::net::FetchOutcome;
use crate::{school_observations_of, CrawlError, CrawlResult};
use census_domain::model::SourceNamespace;
use census_store::Table;
use serde_json::json;

impl Run<'_> {
    pub(super) fn emit(
        &mut self,
        key: &str,
        capture: &FetchOutcome,
        emission: Emission,
    ) -> CrawlResult<()> {
        emission
            .issues
            .iter()
            .try_for_each(|issue| self.fail(&capture.url, issue.clone()))?;
        self.emit_rows(capture, &emission)?;
        if !emission.issues.is_empty() {
            return Ok(());
        }
        let operation = format!("{JOURNAL}:{key}");
        if self
            .ctx
            .effect_is_committed(&operation, &capture.content_digest)?
        {
            self.skipped = self.skipped.checked_add(1).ok_or_else(counter_error)?;
            return Ok(());
        }
        let mut batch = self.ctx.write_batch();
        batch.journal_done(JOURNAL, key, &self.completion_payload(capture, &emission))?;
        batch.commit_once(&operation, &capture.content_digest)?;
        Ok(())
    }

    fn emit_rows(&mut self, capture: &FetchOutcome, emission: &Emission) -> CrawlResult<()> {
        if self
            .ctx
            .append_row_once(JOURNAL, Table::Schools, &emission.school)?
        {
            self.report.rows = self.report.rows.checked_add(1).ok_or_else(counter_error)?;
        }
        school_observations_of(
            &SourceNamespace::association_school("tssaa"),
            std::slice::from_ref(&emission.school),
            &capture.fetched_at,
        )
        .iter()
        .try_for_each(|row| {
            self.ctx
                .append_row_once(JOURNAL, Table::SourceObservations, row)
                .map(|_| ())
        })?;
        emission.coaches.iter().try_for_each(|coach| {
            if self.ctx.append_row_once(JOURNAL, Table::Coaches, coach)? {
                self.coaches = self.coaches.checked_add(1).ok_or_else(counter_error)?;
                self.report.with_email = self
                    .report
                    .with_email
                    .checked_add(u64::from(coach.has_published_email()))
                    .ok_or_else(counter_error)?;
            }
            Ok::<_, CrawlError>(())
        })
    }

    fn completion_payload(&self, capture: &FetchOutcome, emission: &Emission) -> serde_json::Value {
        json!({"state":"TN","school_id":emission.school.source_identities.first().map(|owner| &owner.id),
            "school":emission.school.name,"coach_rows":emission.coaches.len(),
            "capture_url":capture.url,"capture_sha256":capture.content_digest,"captured_at":capture.fetched_at,
            "school_year":self.ctx.school_year,"performance_as_of":self.ctx.performance_as_of,
            "complete":emission.issues.is_empty(),"issues":emission.issues})
    }
}
