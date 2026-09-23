//! What one walk of the result-set route carries across result sets, and the unit of work it reads.
//!
//! Split out of [`super`] to keep both files inside the source-length budget: this file owns the run
//! state (the school resolver, the resume set, the counters, the entities minted so far) and the
//! per-result-set read; `super` owns the entry point, the shared types and the table appends.

use super::super::fetch::fetch_result_set;
use super::super::map::absorb_result_set;
use super::super::wire::ResultSetRef;
use super::{Accumulator, Stats};
use crate::AdapterContext;
use census_domain::model::SchoolId;
use census_domain::school_index::SchoolIndex;
use std::collections::{HashMap, HashSet};

/// What one walk of the route carries across result sets.
pub(super) struct Run {
    pub(super) index: SchoolIndex,
    pub(super) resolved: HashMap<String, Option<SchoolId>>,
    pub(super) stats: Stats,
    pub(super) accumulated: Accumulator,
    pub(super) done: HashSet<String>,
    /// The entries this walk has earned, committed with the rows they name by `super::append`.
    pub(super) pending: Vec<(String, serde_json::Value)>,
}

impl Run {
    /// Read one result set: fetch, parse, absorb, and buffer the unit of work.
    ///
    /// A fetch or parse failure is recorded against the result set instead of ending the walk. The
    /// entry is not written here: the walk writes nothing, and `super::append` commits every entry
    /// with the rows its result sets produced, so a set counts as read only once those rows are
    /// durable — a commit the store refuses ends the walk with the store's error rather than
    /// leaving a set marked read whose rows never landed.
    pub(super) async fn read(&mut self, ctx: &AdapterContext<'_>, reference: &ResultSetRef) {
        let key = format!("{}/{}", reference.meet_id, reference.rsid);
        if self.done.contains(&key) {
            self.stats.result_sets_resumed = self.stats.result_sets_resumed.saturating_add(1);
            return;
        }
        let page = match fetch_result_set(ctx.fetcher, reference, &ctx.fetch_options()).await {
            Ok(page) => page,
            Err(error) => {
                self.stats
                    .failures
                    .push(format!("{}: {error}", reference.url));
                return;
            }
        };
        self.stats.result_sets = self.stats.result_sets.saturating_add(1);
        self.stats.skipped_lines = self.stats.skipped_lines.saturating_add(page.skipped.len());
        if page.meet.rows_parsed == 0 {
            // An empty result set is data, not an error: it is journaled so the run does not repeat
            // it, and nothing is minted from it.
            self.stats.result_sets_empty = self.stats.result_sets_empty.saturating_add(1);
        } else {
            absorb_result_set(
                &page,
                reference,
                &ctx.observed_on,
                &self.index,
                &mut self.resolved,
                &mut self.stats,
                &mut self.accumulated,
            );
        }
        let payload = serde_json::json!({
            "meet": reference.meet_id,
            "rsid": reference.rsid,
            "rows": page.meet.rows_parsed,
            "skipped_lines": page.skipped.len(),
        });
        self.pending.push((key, payload));
    }

    /// Record an entry that is not a result-set URL; nothing is requested for it.
    pub(super) fn reject(&mut self, entry: &str) {
        self.stats
            .failures
            .push(format!("{entry}: not a /meets/<id>/results/<rsid>/raw URL"));
    }
}
