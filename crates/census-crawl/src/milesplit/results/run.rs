
use super::super::fetch::fetch_result_set;
use super::super::map::absorb_result_set;
use super::super::wire::ResultSetRef;
use super::{Accumulator, Stats};
use crate::AdapterContext;
use census_domain::model::SchoolId;
use census_domain::school_index::SchoolIndex;
use std::collections::{HashMap, HashSet};

pub(super) struct Run {
    pub(super) index: SchoolIndex,
    pub(super) resolved: HashMap<String, Option<SchoolId>>,
    pub(super) stats: Stats,
    pub(super) accumulated: Accumulator,
    pub(super) done: HashSet<String>,
    pub(super) pending: Vec<(String, serde_json::Value)>,
}

impl Run {
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

    pub(super) fn reject(&mut self, entry: &str) {
        self.stats
            .failures
            .push(format!("{entry}: not a /meets/<id>/results/<rsid>/raw URL"));
    }
}
