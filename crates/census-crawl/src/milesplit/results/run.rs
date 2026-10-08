use super::super::map::ProviderSchools;
use super::super::owned::read_owned_meet;
use super::super::raw::RawPage;
use super::super::wire::ResultSetRef;
use super::{Accumulator, Stats};
use crate::net::FetchOutcome;
use crate::{AdapterContext, CrawlResult};
use std::collections::{HashMap, HashSet};

mod acquired;
mod capture;
mod evidence;
mod metadata;
mod projection;
mod receipt;
use acquired::AcquiredMeet;

pub(super) struct Run {
    pub(super) meet_id: String,
    pub(super) schools: ProviderSchools,
    pub(super) owned: HashMap<String, AcquiredMeet>,
    pub(super) pending_counts: HashMap<String, usize>,
    pub(super) stats: Stats,
    pub(super) accumulated: Accumulator,
    pub(super) seen: HashSet<String>,
    pub(super) pending: Vec<(String, serde_json::Value)>,
    pub(super) window_rows: usize,
}

impl Run {
    #[tracing::instrument(skip(self, ctx))]
    pub(super) async fn read(
        &mut self,
        ctx: &AdapterContext<'_>,
        reference: &ResultSetRef,
    ) -> CrawlResult<()> {
        let key = capture::digest(&(
            reference.site.code(),
            &reference.meet_id,
            &reference.rsid,
            &reference.url,
        ))?;
        if !self.seen.insert(key) {
            return Ok(());
        }
        self.acquire_owned(ctx, reference).await?;
        let owner = owned_key(reference);
        *self.pending_counts.entry(owner).or_default() =
            self.pending_counts.get(&owner).copied().unwrap_or(1);
        let (capture, page) = match metadata::fetch_metadata(ctx, reference).await {
            Ok(captured) => captured,
            Err(error) => {
                self.record_failure(reference, None, &error)?;
                return Ok(());
            }
        };
        capture::archive_metadata(ctx, reference, &capture)?;
        match page {
            Ok(page) => self.record_page(ctx, reference, page, capture),
            Err(error) => self.record_failure(reference, Some(&capture), &error),
        }
        self.decrement_pending(reference);
        Ok(())
    }

    #[tracing::instrument(skip(self, ctx))]
    async fn acquire_owned(
        &mut self,
        ctx: &AdapterContext<'_>,
        reference: &ResultSetRef,
    ) -> CrawlResult<()> {
        let key = owned_key(reference);
        if self.owned.contains_key(&key) {
            return Ok(());
        }
        let acquired = AcquiredMeet::new(read_owned_meet(ctx, reference).await?);
        if let Some(failure) = acquired.failure() {
            self.stats.failures.push(failure);
        }
        self.owned.insert(key, acquired);
        Ok(())
    }

    fn record_page(
        &mut self,
        ctx: &AdapterContext<'_>,
        reference: &ResultSetRef,
        page: RawPage,
        metadata: FetchOutcome,
    ) -> CrawlResult<()> {
        self.stats.result_sets = self.stats.result_sets.saturating_add(1);
        self.stats.skipped_lines = self.stats.skipped_lines.saturating_add(page.skipped.len());
        let acquired =
            self.owned
                .get(&owned_key(reference))
                .ok_or_else(|| crate::CrawlError::Invariant {
                    detail: "result projection has no acquired source-owned meet".into(),
                })?;
        let rows_before = self.stats.rows;
        let unresolved_before = unresolved_counts(&self.stats);
        let (mut projected, complete_parse) =
            projection::prepare(acquired, reference, &page, &self.schools, &mut self.stats)?;
        let unresolved = unresolved_before != unresolved_counts(&self.stats);
        note_unresolved(&mut self.stats, reference, unresolved);
        evidence::bind(
            &mut projected,
            reference,
            &page,
            &acquired.outcome.capture,
            &metadata,
        )?;
        receipt::identify_retained(&mut projected)?;
        let entry = receipt::projection(
            reference,
            &page,
            &metadata,
            acquired,
            &projected,
            complete_parse && !unresolved,
            self.stats.rows.saturating_sub(rows_before),
        )?;
        if ctx
            .store
            .journal_contains(super::RESULT_SET_PHASE, &entry.0)?
        {
            self.stats.result_sets_resumed = self.stats.result_sets_resumed.saturating_add(1);
        }
        self.accumulated.absorb(projected);
        self.pending.push(entry);
        Ok(())
    }

    fn record_failure(
        &mut self,
        reference: &ResultSetRef,
        metadata: Option<&FetchOutcome>,
        error: &crate::CrawlError,
    ) -> CrawlResult<()> {
        let reason = format!(
            "{}: raw meet/season metadata unavailable: {error}",
            reference.url
        );
        let acquired =
            self.owned
                .get(&owned_key(reference))
                .ok_or_else(|| crate::CrawlError::Invariant {
                    detail: "metadata failure has no acquired source-owned meet".into(),
                })?;
        self.pending
            .push(receipt::failure(reference, acquired, metadata, &reason)?);
        self.stats.failures.push(reason);
        Ok(())
    }

    pub(super) fn reject(&mut self, entry: &str) {
        self.stats
            .failures
            .push(format!("{entry}: not a /meets/<id>/results/<rsid>/raw URL"));
    }
    pub(super) fn rows(&self) -> usize {
        self.accumulated.rows()
    }

    pub(super) fn drain_accumulated(&mut self) -> Accumulator {
        std::mem::take(&mut self.accumulated)
    }

    pub(super) fn drain_pending(&mut self) -> Vec<(String, serde_json::Value)> {
        std::mem::take(&mut self.pending)
    }

    fn decrement_pending(&mut self, reference: &ResultSetRef) {
        let key = owned_key(reference);
        if let Some(count) = self.pending_counts.get_mut(&key) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.owned.remove(&key);
                self.pending_counts.remove(&key);
            }
        }
    }
}

fn unresolved_counts(stats: &Stats) -> (usize, usize, usize, usize) {
    (
        stats.rows_without_cohort,
        stats.rows_without_school,
        stats.rows_without_sport,
        stats.rows_without_name,
    )
}

fn note_unresolved(stats: &mut Stats, reference: &ResultSetRef, unresolved: bool) {
    if unresolved {
        stats.failures.push(format!(
            "{}: source-owned observations retained unresolved; exact provider school, published cohort, name or sport metadata unavailable",
            reference.url,
        ));
    }
}

fn owned_key(reference: &ResultSetRef) -> String {
    format!("{}/{}", reference.site.code(), reference.meet_id)
}

#[cfg(test)]
mod owned_tests;
