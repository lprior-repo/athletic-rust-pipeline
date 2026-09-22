//! The state one result-plane walk carries across captures, and the captures it reads.
//!
//! Part of [`super`]: this file owns the run (the meet it files under, the school index it resolves
//! labels against, the memo of resolved labels, the resume set and the counters), while
//! `results::absorb` owns the folds that read a payload into it.

use super::super::map::{Accumulator, ResultStats, SOURCE_ID};
use super::super::parse::infer_level;
use super::super::wire::event_summary_url;
use super::absorb::{absorb_document, absorb_standings, absorb_summary, Fold, PublishedEvent};
use super::{ResultOptions, StandingsCapture, WalkResult, PHASE};
use crate::school_index::SchoolIndex;
use crate::sources::athleticlive_athletes::{school_year_for_date, MeetTarget};
use crate::sources::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalMeet, CanonicalSchool, Evidence, SchoolId, SourceIdentity, SourceNamespace, SourceRef,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

/// What one walk carries across captures.
pub(super) struct Run {
    meet: CanonicalMeet,
    target: MeetTarget,
    observed_on: String,
    school_year: census_domain::model::SchoolYear,
    limit: Option<usize>,
    index: SchoolIndex,
    resolved: HashMap<String, Option<SchoolId>>,
    stats: ResultStats,
    accumulator: Accumulator,
    /// Captures that could not be folded, in the order they were read.
    failures: Vec<String>,
    /// Captures a previous run already journaled, and the phase's own keys.
    done: HashSet<String>,
    resumed: usize,
    /// The events this run minted, keyed by the run key their standings are published under.
    by_run: BTreeMap<String, PublishedEvent>,
    /// The individual event ids the summary listed, and the ones the documents carried.
    listed: Vec<u64>,
    read_documents: HashSet<u64>,
}

impl Run {
    /// Open a run: the meet it files under, the index it resolves labels against, and the resume set.
    pub(super) fn new(
        ctx: &AdapterContext<'_>,
        target: &MeetTarget,
        options: &ResultOptions,
    ) -> CrawlResult<Self> {
        let schools = consolidated_schools(ctx)?;
        // The meet is this route's own entity: it carries the harvest's facts plus the identities and
        // evidence the captures add, and every performance it writes points at it, so a run over
        // captures alone has to leave it in the store under the id the harvest route also mints.
        let meet = meet_for(target, &options.observed_on);
        let mut accumulator = Accumulator::default();
        accumulator
            .meets
            .insert(meet.id.as_str().to_string(), meet.clone());
        Ok(Self {
            meet,
            target: target.clone(),
            observed_on: options.observed_on.clone(),
            school_year: school_year_for_date(&target.date, ctx.school_year),
            limit: options.limit,
            index: SchoolIndex::from_schools(&schools),
            resolved: HashMap::new(),
            stats: ResultStats::default(),
            accumulator,
            failures: Vec::new(),
            done: ctx.store.journal_keys(PHASE)?,
            resumed: 0,
            by_run: BTreeMap::new(),
            listed: Vec::new(),
            read_documents: HashSet::new(),
        })
    }

    /// The fold's view of this run.
    fn fold(&mut self) -> Fold<'_> {
        Fold {
            meet: &self.meet,
            target: &self.target,
            observed_on: &self.observed_on,
            school_year: self.school_year,
            index: &self.index,
            resolved: &mut self.resolved,
            stats: &mut self.stats,
            accumulator: &mut self.accumulator,
            failures: &mut self.failures,
        }
    }

    /// Read the summary, then the documents, then the standings, in that order: a standings capture
    /// can only be filed once the document that names its run key has been read.
    pub(super) fn read_captures(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &ResultOptions,
    ) -> CrawlResult<()> {
        if let Some(path) = options.summary.as_deref() {
            self.read_once(ctx, path, |run| {
                let body = read_capture(path)?;
                let listed = {
                    let mut fold = run.fold();
                    absorb_summary(&mut fold, path, &body)
                };
                let Some(listed) = listed else {
                    return Ok(None);
                };
                let payload = json!({"role": "summary", "events": listed.len()});
                run.listed = listed;
                Ok(Some(payload))
            })?;
        }
        for path in &options.documents {
            if self
                .limit
                .is_some_and(|limit| self.read_documents.len() >= limit)
            {
                break;
            }
            self.read_once(ctx, path, |run| run.read_document(path))?;
        }
        for capture in &options.standings {
            self.read_once(ctx, &capture.path, |run| run.read_standings(capture))?;
        }
        let unfetched = self
            .listed
            .iter()
            .filter(|event_id| !self.read_documents.contains(event_id))
            .count();
        self.stats.events_unfetched = self.stats.events_unfetched.saturating_add(unfetched);
        Ok(())
    }

    /// Read one capture unless an earlier run journaled it, then journal what it yielded.
    ///
    /// A capture that could not be folded yields `None` and is not journaled, so the next run reads
    /// it again rather than treating the refusal as done.
    fn read_once(
        &mut self,
        ctx: &AdapterContext<'_>,
        path: &str,
        read: impl FnOnce(&mut Self) -> CrawlResult<Option<Value>>,
    ) -> CrawlResult<()> {
        if self.done.contains(path) {
            self.resumed = self.resumed.saturating_add(1);
            return Ok(());
        }
        if let Some(payload) = read(self)? {
            ctx.store.journal_done(PHASE, path, &payload)?;
        }
        Ok(())
    }

    /// Fold one event document and register the event under its run key.
    fn read_document(&mut self, path: &str) -> CrawlResult<Option<Value>> {
        let body = read_capture(path)?;
        let before = self.stats.rows_read;
        let event = {
            let mut fold = self.fold();
            absorb_document(&mut fold, path, &body)
        };
        let Some(event) = event else {
            return Ok(None);
        };
        let rows = self.stats.rows_read.saturating_sub(before);
        self.read_documents.insert(event.capture_id);
        let payload = json!({"role": "event", "event": event.capture_id, "rows": rows});
        if let Some(run_id) = event.run_id.clone() {
            self.by_run.insert(run_id, event);
        }
        Ok(Some(payload))
    }

    /// Fold one live-standings capture into the event that published its run key.
    fn read_standings(&mut self, capture: &StandingsCapture) -> CrawlResult<Option<Value>> {
        let Some(event) = self.by_run.get(&capture.run_id).cloned() else {
            self.failures.push(format!(
                "{}: no event document in this run published run key `{}`",
                capture.path, capture.run_id
            ));
            return Ok(None);
        };
        let body = read_capture(&capture.path)?;
        let rows = {
            let mut fold = self.fold();
            absorb_standings(&mut fold, &capture.run_id, &capture.path, &body, &event)
        };
        Ok(rows.map(|rows| json!({"role": "standings", "run": capture.run_id, "rows": rows})))
    }

    /// Close the walk: hand back its entities, its refusals and its resume count.
    pub(super) fn close(mut self) -> WalkResult {
        WalkResult {
            entities: self
                .accumulator
                .into_entities(std::mem::take(&mut self.stats)),
            failures: std::mem::take(&mut self.failures),
            resumed: self.resumed,
        }
    }
}

/// Read one capture from disk.
fn read_capture(path: &str) -> CrawlResult<String> {
    std::fs::read_to_string(path).map_err(|source| CrawlError::Io {
        path: PathBuf::from(path),
        source,
    })
}

/// The canonical meet one target names, with the identities and evidence this route adds.
///
/// The meet is minted from the target's own facts — never from a result payload — so it lands on the
/// id `meets::build_meets` wrote for the same harvest row, and the two routes describe one meet.
fn meet_for(target: &MeetTarget, observed_on: &str) -> CanonicalMeet {
    let mut meet = CanonicalMeet::new(
        Some(target.state),
        target.name.clone(),
        target.date.clone(),
        infer_level(&target.name),
    );
    meet.source_identities.push(SourceIdentity::new(
        SourceNamespace::TimerMeet {
            provider: target.tenant.clone(),
        },
        target.athleticlive_meet_id.to_string(),
    ));
    let source = SourceRef::new(
        SOURCE_ID,
        Some(event_summary_url(target.athleticlive_meet_id)),
    );
    let mut evidence = Evidence::parsed(source, observed_on);
    evidence.note = Some(format!(
        "meet {} as the harvest publishes it: {} ({}, {}), results read from captures",
        target.athleticlive_meet_id, target.name, target.date, target.tenant
    ));
    meet.evidence.push(evidence);
    meet
}

/// The consolidated schools this route resolves labels against.
///
/// An empty index means `collect` and `consolidate` have not been run yet, which is an operator
/// error rather than a parse failure: with no index every row would be refused, and an empty report
/// would read as a meet that published no results.
fn consolidated_schools(ctx: &AdapterContext<'_>) -> CrawlResult<Vec<CanonicalSchool>> {
    let schools: Vec<CanonicalSchool> =
        crate::store::read::read_rows(&ctx.store.out_dir().join("schools.jsonl"))?;
    if schools.is_empty() {
        return Err(CrawlError::Invariant {
            detail: "no consolidated schools: run `collect` and `consolidate` before the \
                     athleticlive results route"
                .to_string(),
        });
    }
    Ok(schools)
}
