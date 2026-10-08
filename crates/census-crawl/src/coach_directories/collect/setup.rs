use super::{CoachCounters, Options, Run, JOURNAL, SOURCE_ID};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::normalize_name;
use std::collections::{BTreeMap, HashMap, HashSet};

impl<'a> Run<'a> {
    pub(super) fn new(ctx: &'a AdapterContext<'a>, options: &'a Options) -> CrawlResult<Self> {
        let mut fetch = ctx.fetch_options();
        fetch.refresh |= options.refresh;
        Ok(Self {
            ctx,
            options,
            fetch,
            wanted: wanted_names(options),
            done: ctx.store.journal_keys(JOURNAL)?,
            report: AdapterReport::new(SOURCE_ID, "schools"),
            processed: 0,
            skipped: 0,
            coach_rows: 0,
            with_email: 0,
            counters: CoachCounters::default(),
            dropped_school_rows: 0,
            researched: HashMap::new(),
            frontiers: BTreeMap::new(),
            incomplete_states: HashSet::new(),
            drained_states: 0,
            seen_names: HashSet::new(),
        })
    }
}

fn wanted_names(options: &Options) -> HashSet<String> {
    options
        .school_names
        .iter()
        .map(|name| normalize_name(name))
        .filter(|name| !name.is_empty())
        .collect()
}

pub(super) fn unrequested() -> AdapterReport {
    let mut report = AdapterReport::new(SOURCE_ID, "schools");
    report.note("no requested jurisdiction is one of the 15 associations that publish staff on this platform, so nothing was fetched");
    report
}
