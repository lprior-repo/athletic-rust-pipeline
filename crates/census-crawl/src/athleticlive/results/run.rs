use super::super::map::{Accumulator, ResultStats};
use super::super::parse::infer_level;
use super::super::wire::{event_doc_url, event_summary_url, standings_url};
use super::absorb::{absorb_document, absorb_standings, absorb_summary, Fold, PublishedEvent};
use super::{EntityCounts, StandingsCapture};
use crate::athleticlive_athletes::{school_year_for_date, MeetTarget};
use crate::net::cache::CacheMeta;
use crate::{AdapterContext, CrawlError, CrawlResult, PerformanceDateAssessment};
use census_domain::model::{
    CanonicalMeet, CanonicalSchool, SchoolId, SourceIdentity, SourceNamespace,
};
use census_domain::school_index::SchoolIndex;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

mod capture;
mod documents;
mod inputs;
mod publication;
mod receipts;

use publication::{capture_evidence, unresolved};

const MAX_CAPTURE_RECORDS: usize = 8192;

pub(super) struct Run {
    meet: CanonicalMeet,
    target: MeetTarget,
    performance_as_of: chrono::NaiveDate,
    school_year: census_domain::model::SchoolYear,
    index: SchoolIndex,
    schools: Vec<CanonicalSchool>,
    resolved: HashMap<String, Option<SchoolId>>,
    stats: ResultStats,
    accumulator: Accumulator,
    failures: Vec<String>,
    failure_count: usize,
    unfinished: Vec<String>,
    counts: EntityCounts,
    resumed: usize,
    by_run: HashMap<String, Option<PublishedEvent>>,
    listed: Vec<u64>,
    read_documents: HashSet<u64>,
}

impl Run {
    pub(super) fn new(ctx: &AdapterContext<'_>, target: &MeetTarget) -> CrawlResult<Self> {
        super::retired_receipts(ctx)?;
        let schools = ctx.store.scan(census_store::Table::Schools)?;
        let meet = base_meet(target);
        Ok(Self {
            meet,
            target: target.clone(),
            performance_as_of: ctx.performance_as_of,
            school_year: school_year_for_date(&target.date, ctx.school_year),
            index: SchoolIndex::from_schools(&schools),
            schools,
            resolved: HashMap::new(),
            stats: ResultStats::default(),
            accumulator: Accumulator::default(),
            failures: Vec::new(),
            failure_count: 0,
            unfinished: Vec::new(),
            counts: EntityCounts::default(),
            resumed: 0,
            by_run: HashMap::new(),
            listed: Vec::new(),
            read_documents: HashSet::new(),
        })
    }

    fn fold<'a>(&'a mut self, metadata: &'a CacheMeta) -> Fold<'a> {
        Fold {
            meet: &self.meet,
            target: &self.target,
            observed_on: &metadata.fetched_at,
            capture_sha256: &metadata.content_digest,
            performance_as_of: self.performance_as_of,
            school_year: self.school_year,
            index: &self.index,
            resolved: &mut self.resolved,
            stats: &mut self.stats,
            accumulator: &mut self.accumulator,
            failures: &mut self.failures,
        }
    }

    fn read_summary(
        &mut self,
        path: &str,
        body: &str,
        metadata: &CacheMeta,
    ) -> CrawlResult<Option<Value>> {
        capture::require_url(
            metadata,
            &event_summary_url(self.target.athleticlive_meet_id),
        )?;
        let Some(listed) = absorb_summary(&mut self.fold(metadata), path, body) else {
            return Ok(None);
        };
        if listed.len() > MAX_CAPTURE_RECORDS {
            return Err(resource("LIVE listed events", listed.len()));
        }
        let payload = json!({"role":"summary","events":listed.len()});
        self.listed = listed;
        Ok(Some(payload))
    }

    fn read_once(
        &mut self,
        ctx: &AdapterContext<'_>,
        path: &str,
        metadata: Option<&CacheMeta>,
        read: impl FnOnce(&mut Self, &str, &CacheMeta) -> CrawlResult<Option<Value>>,
    ) -> CrawlResult<()> {
        let Some(metadata) = metadata else {
            return self.reject(path, "capture has no physical producer metadata");
        };
        if let Err(error) = receipts::require_owner(ctx, path, metadata, self) {
            return self.reject(path, &error.to_string());
        }
        let body = match capture::read(ctx, path, Some(metadata)) {
            Ok(body) => body,
            Err(error) => return self.reject(path, &error.to_string()),
        };
        if matches!(
            crate::assess_performance_date(self.performance_as_of, &self.target.date),
            PerformanceDateAssessment::Unknown
        ) {
            return self.reject(
                path,
                "published meet date is absent or invalid; retained capture remains owed",
            );
        }
        let failures_before = self.failures.len();
        let unresolved_before = unresolved(&self.stats)?;
        self.meet.evidence.clear();
        self.meet.evidence.push(capture_evidence(metadata));
        let payload = match read(self, &body, metadata) {
            Ok(payload) => payload,
            Err(error) => {
                self.failures
                    .try_reserve(1)
                    .map_err(|_| resource("LIVE failure allocation", 1))?;
                self.failures.push(error.to_string());
                None
            }
        };
        self.retain_window(
            ctx,
            (path, &body, metadata),
            (failures_before, unresolved_before),
            payload,
        )
    }

    fn read_document(
        &mut self,
        path: &str,
        body: &str,
        metadata: &CacheMeta,
    ) -> CrawlResult<Option<Value>> {
        let doc = self.document(path, body)?;
        let id = doc
            .event_id()
            .ok_or_else(|| schema(path, "document has no event ID"))?;
        capture::require_url(metadata, &event_doc_url(id))?;
        if doc.meet_id() != Some(self.target.athleticlive_meet_id) {
            return Err(schema(path, "document has no matching published meet ID"));
        }
        self.mark_document(id)?;
        if self.future() {
            return Ok(Some(json!({"role":"future excluded","event":id})));
        }
        let Some(event) = absorb_document(&mut self.fold(metadata), path, doc) else {
            return Ok(None);
        };
        let payload = json!({"role":"event","event":event.capture_id});
        if let Some(run_id) = event.run_id.clone() {
            self.bind_run(run_id, event)?;
        }
        Ok(Some(payload))
    }

    fn bind_run(&mut self, run_id: String, event: PublishedEvent) -> CrawlResult<()> {
        self.by_run
            .try_reserve(1)
            .map_err(|_| resource("LIVE run allocation", 1))?;
        let bound = self
            .by_run
            .entry(run_id)
            .or_insert_with(|| Some(event.clone()));
        if bound
            .as_ref()
            .is_some_and(|known| known.id != event.id || known.capture_id != event.capture_id)
        {
            *bound = None;
        }
        Ok(())
    }

    fn mark_document(&mut self, id: u64) -> CrawlResult<()> {
        if self.read_documents.len() >= MAX_CAPTURE_RECORDS
            || self.by_run.len() >= MAX_CAPTURE_RECORDS
        {
            return Err(resource("LIVE event frontier", MAX_CAPTURE_RECORDS));
        }
        self.read_documents
            .try_reserve(1)
            .map_err(|_| resource("LIVE event frontier allocation", 1))?;
        self.read_documents.insert(id);
        Ok(())
    }

    fn read_standings(
        &mut self,
        row: &StandingsCapture,
        body: &str,
        metadata: &CacheMeta,
    ) -> CrawlResult<Option<Value>> {
        let url = standings_url(self.target.athleticlive_meet_id, &row.run_id)
            .ok_or_else(|| schema(&row.path, "run ID is not a published path segment"))?;
        capture::require_url(metadata, &url)?;
        if self.future() {
            return Ok(Some(json!({"role":"future excluded","run":row.run_id})));
        }
        let event = self
            .by_run
            .get(&row.run_id)
            .and_then(Option::as_ref)
            .cloned()
            .ok_or_else(|| {
                schema(
                    &row.path,
                    "no unique qualified event document publishes this run",
                )
            })?;
        let rows = absorb_standings(
            &mut self.fold(metadata),
            &row.run_id,
            &row.path,
            body,
            &event,
        );
        Ok(rows.map(|rows| json!({"role":"standings","run":row.run_id,"rows":rows})))
    }

    fn future(&self) -> bool {
        matches!(
            crate::assess_performance_date(self.performance_as_of, &self.target.date),
            PerformanceDateAssessment::Future
        )
    }
}

fn base_meet(target: &MeetTarget) -> CanonicalMeet {
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
    meet
}

pub(super) fn counter_error() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "LIVE result accounting overflow".to_string(),
    }
}
fn resource(resource: &'static str, requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit: MAX_CAPTURE_RECORDS,
    }
}
fn schema(path: &str, detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: path.to_string(),
        detail: detail.to_string(),
    }
}
