use super::super::map::{Accumulator, ResultStats, SOURCE_ID};
use super::super::parse::infer_level;
use super::super::wire::event_summary_url;
use super::absorb::{Fold, PublishedEvent};
use super::{
    CapturedBody, EffectReceipt, ResultOptions, WalkResult, CAPTURE_PHASE, EFFECT_PHASE,
    RETIRED_PHASE,
};
use crate::athleticlive_athletes::{school_year_for_date, MeetTarget};
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalMeet, CanonicalSchool, Evidence, SchoolId, SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::school_index::SchoolIndex;
use std::collections::{BTreeMap, HashMap, HashSet};

mod captures;
mod receipts;

use self::receipts::{parse_receipt_key, ReceiptIndex};

pub(super) const PARSER: &str = "athleticlive_results_v2";
pub(super) const ROLE_SUMMARY: &str = "summary";
pub(super) const ROLE_EVENT: &str = "event";
pub(super) const ROLE_STANDINGS: &str = "standings";

pub(super) struct Run {
    meet: CanonicalMeet,
    target: MeetTarget,
    observed_on: String,
    school_year: census_domain::model::SchoolYear,
    limit: Option<usize>,
    index: SchoolIndex,
    schools: Vec<CanonicalSchool>,
    resolved: HashMap<String, Option<SchoolId>>,
    stats: ResultStats,
    accumulator: Accumulator,
    failures: Vec<String>,
    indexed: ReceiptIndex,
    archived: HashSet<String>,
    captured: Vec<CapturedBody>,
    receipts: Vec<EffectReceipt>,
    resumed: usize,
    by_run: BTreeMap<String, PublishedEvent>,
    listed: Vec<u64>,
    read_documents: HashSet<u64>,
}

impl Run {
    pub(super) fn new(
        ctx: &AdapterContext<'_>,
        target: &MeetTarget,
        options: &ResultOptions,
    ) -> CrawlResult<Self> {
        let schools = consolidated_schools(ctx)?;
        let meet = meet_for(target, &options.observed_on);
        let mut accumulator = Accumulator::default();
        accumulator
            .meets
            .insert(meet.id.as_str().to_string(), meet.clone());
        let retired = ctx.store.journal_keys(RETIRED_PHASE)?;
        if !retired.is_empty() {
            return Err(CrawlError::Invariant {
                detail: format!(
                    "the store holds {} receipts under the retired `{RETIRED_PHASE}` phase, whose \
                     shape cannot prove which bytes an earlier run projected; this route refuses to \
                     resume it - run against a fresh store directory",
                    retired.len()
                ),
            });
        }
        let mut indexed = ReceiptIndex::default();
        for key in ctx.store.journal_keys(EFFECT_PHASE)? {
            if let Some((meet_id, role, path, digest)) = parse_receipt_key(&key) {
                indexed
                    .by_path
                    .insert(path, (digest.clone(), meet_id.clone(), role.clone()));
                indexed
                    .by_digest
                    .entry(digest)
                    .or_default()
                    .push((meet_id, role));
            }
        }
        Ok(Self {
            meet,
            target: target.clone(),
            observed_on: options.observed_on.clone(),
            school_year: school_year_for_date(&target.date, ctx.school_year),
            limit: options.limit,
            index: SchoolIndex::from_schools(&schools),
            schools,
            resolved: HashMap::new(),
            stats: ResultStats::default(),
            accumulator,
            failures: Vec::new(),
            indexed,
            archived: ctx.store.journal_keys(CAPTURE_PHASE)?,
            captured: Vec::new(),
            receipts: Vec::new(),
            resumed: 0,
            by_run: BTreeMap::new(),
            listed: Vec::new(),
            read_documents: HashSet::new(),
        })
    }

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

    pub(super) fn close(mut self) -> WalkResult {
        WalkResult {
            entities: self
                .accumulator
                .into_entities(std::mem::take(&mut self.stats)),
            schools: self.schools,
            captured: std::mem::take(&mut self.captured),
            receipts: std::mem::take(&mut self.receipts),
            failures: std::mem::take(&mut self.failures),
            resumed: self.resumed,
        }
    }
}

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

fn consolidated_schools(ctx: &AdapterContext<'_>) -> CrawlResult<Vec<CanonicalSchool>> {
    let schools: Vec<CanonicalSchool> =
        census_store::read::read_rows(&ctx.store.out_dir().join("schools.jsonl"))?;
    if schools.is_empty() {
        return Err(CrawlError::Invariant {
            detail: "no consolidated schools: run `collect` and `consolidate` before the \
                     athleticlive results route"
                .to_string(),
        });
    }
    Ok(schools)
}
