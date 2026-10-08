use super::super::{
    append,
    budget::{self, Footprint},
    Accumulator, EntityCounts, Stats,
};
use super::{evidence, projection, receipt};
use crate::net::FetchOutcome;
use crate::{AdapterContext, CrawlError, CrawlResult};
use sha2::{Digest, Sha256};

mod admission;
mod completion;
mod journal;

struct Window {
    indices: Vec<usize>,
    footprint: Footprint,
    ordinal: usize,
    rows: usize,
    complete: bool,
    digest: Sha256,
}

impl Window {
    fn new(complete: bool) -> CrawlResult<Self> {
        let mut indices = Vec::new();
        indices
            .try_reserve_exact(budget::WINDOW_ROWS)
            .map_err(budget::reserve)?;
        Ok(Self {
            indices,
            footprint: Footprint::default(),
            ordinal: 0,
            rows: 0,
            complete,
            digest: Sha256::new(),
        })
    }
}

struct Writer<'a, 'b> {
    input: &'a projection::Input<'b>,
    metadata: &'a FetchOutcome,
    stats: &'a mut Stats,
    counts: &'a mut EntityCounts,
    context: Footprint,
    failures_before: usize,
    window: Window,
}

pub(super) fn execute(
    ctx: &AdapterContext<'_>,
    input: &projection::Input<'_>,
    metadata: &FetchOutcome,
    stats: &mut Stats,
    counts: &mut EntityCounts,
) -> CrawlResult<()> {
    stats.result_sets = stats.result_sets.saturating_add(1);
    stats.skipped_lines = stats.skipped_lines.saturating_add(input.page.skipped.len());
    let owned = input
        .acquired
        .result_set(&input.reference.rsid, input.performance_as_of);
    let complete = owned
        .as_ref()
        .is_some_and(|owned| owned.page.individual_parse_complete());
    let indices = owned.as_ref().map_or(&[][..], |owned| owned.indices);
    if indices.is_empty() {
        stats.result_sets_empty = stats.result_sets_empty.saturating_add(1);
    }
    let context = admission::context(input, metadata)?;
    let failures_before = stats.failure_count;
    let mut writer = Writer {
        input,
        metadata,
        stats,
        counts,
        context,
        failures_before,
        window: Window::new(complete)?,
    };
    match indices
        .iter()
        .try_for_each(|index| writer.accept(ctx, *index))
    {
        Ok(()) => writer.finish(ctx),
        Err(error @ CrawlError::Resource { .. }) => writer.blocked(ctx, &error),
        Err(error) => Err(error),
    }
}

impl Writer<'_, '_> {
    fn accept(&mut self, ctx: &AdapterContext<'_>, index: usize) -> CrawlResult<()> {
        let footprint = admission::row(self.input, index)?.projection(self.context)?;
        if !footprint.fits() {
            return self.reject(ctx, index, footprint);
        }
        let next = self.window.footprint.checked_add(footprint)?;
        if self.window.indices.len() >= budget::WINDOW_ROWS || !next.fits() {
            self.flush(ctx, false)?;
        }
        self.window.footprint = self.window.footprint.checked_add(footprint)?;
        self.window.indices.push(index);
        self.stats.peak_window_bytes = self
            .stats
            .peak_window_bytes
            .max(self.window.footprint.bytes);
        Ok(())
    }

    fn flush(&mut self, ctx: &AdapterContext<'_>, final_window: bool) -> CrawlResult<()> {
        if self.window.indices.is_empty() && !final_window {
            return Ok(());
        }
        admission::metadata(self.stats, self.window.indices.len())?;
        admission::recording(ctx, self.window.footprint)?;
        let before = unresolved(self.stats);
        let projected = self.project()?;
        self.window.complete &= before == unresolved(self.stats);
        let rows = self.window.indices.len();
        let mut entry = receipt::projection(
            self.input,
            self.metadata,
            &projected,
            self.window.complete,
            rows,
        )?;
        if !final_window || self.window.ordinal > 0 {
            journal::window(&mut entry, self.input, &self.window)?;
        }
        self.persist(ctx, projected, &entry)?;
        self.window.digest.update(entry.0.as_bytes());
        self.window.digest.update([0]);
        self.window.ordinal = self.window.ordinal.saturating_add(1);
        self.window.rows = self.window.rows.saturating_add(rows);
        self.window.indices.clear();
        self.window.footprint = Footprint::default();
        Ok(())
    }

    fn project(&mut self) -> CrawlResult<Accumulator> {
        let mut projected = match projection::prepare(self.input, &self.window.indices, self.stats)?
        {
            projection::Prepared::Complete(projected) => projected,
            projection::Prepared::Unfinished { projected, error } => {
                self.window.complete = false;
                self.stats.failure(format!(
                    "{}: {error}; valid draft retained; source projection remains unfinished",
                    self.input.reference.url
                ))?;
                projected
            }
        };
        evidence::bind(
            &mut projected,
            self.input.reference,
            self.input.page,
            &self.input.acquired.outcome.capture,
            self.metadata,
        )?;
        receipt::identify_retained(&mut projected)?;
        Ok(projected)
    }

    fn persist(
        &mut self,
        ctx: &AdapterContext<'_>,
        projected: Accumulator,
        entry: &(String, serde_json::Value),
    ) -> CrawlResult<()> {
        if super::super::journal_contains(ctx, super::super::RESULT_SET_PHASE, &entry.0)? {
            self.stats.result_sets_resumed = self.stats.result_sets_resumed.saturating_add(1);
        }
        self.counts.add(append::window(ctx, projected, entry)?);
        Ok(())
    }

    fn reject(
        &mut self,
        ctx: &AdapterContext<'_>,
        index: usize,
        footprint: Footprint,
    ) -> CrawlResult<()> {
        self.flush(ctx, false)?;
        self.window.complete = false;
        let entry = journal::rejected(self.input, self.metadata, index, footprint)?;
        journal::commit(ctx, &entry)?;
        self.stats.failure(format!("{}: result row {index} exceeds byte/work admission; original locator retained unfinished", self.input.reference.url))
    }
}

fn unresolved(stats: &Stats) -> (usize, usize, usize, usize) {
    (
        stats.rows_without_cohort,
        stats.rows_without_school,
        stats.rows_without_sport,
        stats.rows_without_name,
    )
}
